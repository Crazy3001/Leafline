<script lang="ts">
  import { Camera, Lock, LockOpen, Minimize2, PanelsTopLeft, Pin, PinOff, Settings, X } from "@lucide/svelte";
  import { onMount } from "svelte";

  import { backend, fileName } from "$lib/api";
  import CompactHud from "$lib/components/CompactHud.svelte";
  import Logo from "$lib/components/Logo.svelte";
  import PanelView from "$lib/components/PanelView.svelte";
  import { PANELS, panelInfo, signature } from "$lib/panels";
  import { live, watchLive } from "$lib/readings.svelte";
  import { display, watchSettings } from "$lib/settings.svelte";
  import type { Panel } from "$lib/types";

  const TAB_KEY = "leafline.overlay.tab";

  let tab = $state<Panel>(storedTab());
  /** What each panel showed when last looked at, to mark tabs with something new. */
  let seen = $state<Partial<Record<Panel, string | null>>>({});

  onMount(() => {
    watchLive();
    watchSettings();
  });

  const signatures = $derived(
    Object.fromEntries(PANELS.map((p) => [p.id, signature(p.id, live.analysis, live.session)])) as Record<
      Panel,
      string | null
    >,
  );

  // The open tab is always seen; others get a dot when what they show changes.
  $effect(() => {
    const current = signatures[tab];
    seen[tab] = current;
  });

  // A price check is asked for by hotkey, so its result opens by itself.
  $effect(() => {
    if (live.itemRead?.found) select("item");
  });

  const fresh = (panel: Panel) => panel !== tab && signatures[panel] != null && signatures[panel] !== seen[panel];
  const pinned = $derived(live.pins.pinned.includes(tab));

  function select(panel: Panel) {
    tab = panel;
    try {
      localStorage.setItem(TAB_KEY, panel);
    } catch {
      // Remembering the tab is a convenience only.
    }
  }

  function storedTab(): Panel {
    try {
      const saved = localStorage.getItem(TAB_KEY);
      if (PANELS.some((p) => p.id === saved)) return saved as Panel;
    } catch {
      // Private mode or blocked storage: start on the first tab.
    }
    return "stats";
  }

  async function togglePin() {
    live.pins = pinned ? await backend.unpinPanel(tab) : await backend.pinPanel(tab);
  }

  async function toggleLock() {
    await backend.setClickThrough(!live.status.clickThrough);
    live.status.clickThrough = !live.status.clickThrough;
  }

  const liveOn = $derived(live.liveStatus.state === "reading" || live.liveStatus.state === "waiting");
  let liveError = $state<string | null>(null);

  const liveTitle = $derived.by(() => {
    const s = live.liveStatus;
    if (s.state === "reading") return `Reading ${s.window} every ${display.live.interval} s. Click to stop (Ctrl+Alt+R).`;
    if (s.state === "waiting") return `${s.reason} Click to stop (Ctrl+Alt+R).`;
    if (!display.live.enabled) return "Reading saved screenshots only. Live reading is off; turn it on in Settings.";
    return "Start reading the game window (Ctrl+Alt+R)";
  });

  /** One line under the header for what needs the player's attention, most urgent first. */
  const notice = $derived.by((): { tone: "bad" | "warn" | "good" | "quiet"; text: string } | null => {
    const s = live.liveStatus;
    if (liveError) return { tone: "bad", text: liveError };
    if (s.state === "failed") return { tone: "bad", text: `Live reading stopped: ${s.message}` };
    if (live.itemRead?.problem) return { tone: "warn", text: `Couldn't read the item: ${live.itemRead.problem}` };
    if (live.itemRead?.found === 0)
      return { tone: "quiet", text: "No equipment tooltip found. Hover a piece of equipment in game, then press Ctrl+Alt+I." };
    if (live.savedFrame) return { tone: "good", text: `Saved ${fileName(live.savedFrame)} to Pictures\\Leafline.` };
    if (s.state === "waiting") return { tone: "quiet", text: s.reason };
    if (s.state === "reading" && s.hdr)
      return { tone: "warn", text: "Windows HDR is on for the game's display, so readings will be off. Turn HDR off, then restart the game." };
    if (live.analysis?.failed.length) return { tone: "warn", text: `Can't read the ${live.analysis.failed.join(", ")} right now.` };
    return null;
  });

  async function toggleLive() {
    liveError = null;
    try {
      if (liveOn) await backend.stopLive();
      else if (!display.live.enabled) await backend.showSettings();
      else await backend.startLive();
    } catch (e) {
      liveError = String(e);
    }
  }

  async function saveFrame() {
    liveError = null;
    try {
      await backend.saveLiveFrame();
    } catch (e) {
      liveError = String(e);
    }
  }

  function onTabKey(event: KeyboardEvent) {
    const i = PANELS.findIndex((p) => p.id === tab);
    if (event.key === "ArrowRight") select(PANELS[(i + 1) % PANELS.length].id);
    else if (event.key === "ArrowLeft") select(PANELS[(i - 1 + PANELS.length) % PANELS.length].id);
    else return;
    event.preventDefault();
  }
</script>

<svelte:head><title>Leafline</title></svelte:head>

<div
  class="shell"
  class:compact={live.status.compact}
  style:opacity={display.opacity * (live.status.clickThrough ? 0.85 : 1)}
>
  {#if live.status.compact}
    <CompactHud />
  {:else}
    <header data-tauri-drag-region>
      <span class="brand" data-tauri-drag-region>
        <Logo size={16} />
        <span data-tauri-drag-region>Leafline</span>
        <button
          class="mode"
          class:on={live.liveStatus.state === "reading"}
          class:waiting={live.liveStatus.state === "waiting"}
          title={liveTitle}
          onclick={toggleLive}
        >
          {#if liveOn}<span class="live-dot" aria-hidden="true"></span>Live{:else}Offline{/if}
        </button>
      </span>
      <span class="actions">
        {#if live.liveStatus.state === "reading"}
          <button
            title="Save this game picture to Pictures\Leafline, to report a misread (Ctrl+Alt+F)"
            aria-label="Save game picture"
            onclick={saveFrame}
          >
            <Camera size={14} />
          </button>
        {/if}
        {#if import.meta.env.DEV}
          <button title="Open workbench (Ctrl+Alt+W)" aria-label="Open workbench" onclick={() => backend.showWorkbench()}>
            <PanelsTopLeft size={14} />
          </button>
        {/if}
        <button title="Settings" aria-label="Settings" onclick={() => backend.showSettings()}>
          <Settings size={14} />
        </button>
        <button
          title="Compact HUD (Ctrl+Alt+H)"
          aria-label="Compact HUD"
          onclick={() => backend.setCompact(true)}
        >
          <Minimize2 size={14} />
        </button>
        <button
          title={live.status.clickThrough
            ? "Unlock with Ctrl+Alt+L"
            : "Lock: clicks pass through to the game (Ctrl+Alt+L)"}
          class:active={live.status.clickThrough}
          onclick={toggleLock}
        >
          {#if live.status.clickThrough}<Lock size={14} />{:else}<LockOpen size={14} />{/if}
        </button>
        <button title="Hide (Ctrl+Alt+O). Pinned panels stay." aria-label="Hide overlay" onclick={() => backend.hideOverlay()}>
          <X size={14} />
        </button>
      </span>
    </header>

    {#if notice}
      <div class="notice {notice.tone}" role="status">
        <span>{notice.text}</span>
        {#if notice.tone === "bad"}
          <button class="dismiss" aria-label="Dismiss" onclick={() => (liveError = null)}><X size={12} /></button>
        {/if}
      </div>
    {/if}

    <nav class="tabs">
      <div class="tablist" role="tablist" aria-label="Panels" tabindex="-1" onkeydown={onTabKey}>
        {#each PANELS as panel (panel.id)}
          {@const Icon = panel.icon}
          <button
            role="tab"
            id="tab-{panel.id}"
            aria-selected={tab === panel.id}
            aria-controls="panel"
            tabindex={tab === panel.id ? 0 : -1}
            class:empty={signatures[panel.id] == null}
            class:pinned={live.pins.pinned.includes(panel.id)}
            title={live.pins.pinned.includes(panel.id) ? `${panel.title} (also pinned)` : undefined}
            onclick={() => select(panel.id)}
          >
            <span class="icon">
              <Icon size={14} />
              {#if fresh(panel.id)}<span class="dot" aria-label="Updated"></span>{/if}
            </span>
            <span>{panel.title}</span>
          </button>
        {/each}
      </div>
      <button
        class="pin"
        class:active={pinned}
        title={pinned
          ? `Unpin ${panelInfo(tab).title}`
          : `Pin ${panelInfo(tab).title}: keep it on screen in its own window, even when the overlay is hidden`}
        onclick={togglePin}
      >
        {#if pinned}<PinOff size={14} />{:else}<Pin size={14} />{/if}
      </button>
    </nav>

    <div class="main" id="panel" role="tabpanel" aria-labelledby="tab-{tab}">
      {#if live.analysis || live.session}
        {#key tab}
          <div class="panel"><PanelView panel={tab} analysis={live.analysis} session={live.session} /></div>
        {/key}
      {:else}
        <div class="welcome">
          <Logo size={28} />
          <h1>Welcome to Leafline</h1>
          <p class="hint">It reads what your game screen shows and puts useful facts beside it.</p>
          <ol class="steps">
            <li>
              <span class="n">1</span>
              <span
                >Turn on <button class="inline" onclick={() => backend.showSettings()}>live reading</button> to read the
                game as you play.</span
              >
            </li>
            <li>
              <span class="n">2</span>
              <span>Download map data in the Map tab for routes, training spots and quest help.</span>
            </li>
            <li>
              <span class="n">3</span>
              <span
                ><kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>O</kbd> shows or hides this window; <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd
                  >L</kbd
                > lets clicks pass through it.</span
              >
            </li>
          </ol>
          <button class="inline small" onclick={() => backend.showSettings()}>Settings and what Leafline does</button>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  :global(html),
  :global(body) {
    background: transparent;
    overflow: hidden;
  }

  .shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--glass);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg);
    overflow: hidden;
    transition: opacity 0.2s ease;
  }

  .shell.compact {
    height: auto;
    max-height: 100vh;
    border-radius: var(--radius);
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 36px;
    padding: 0 6px 0 12px;
    user-select: none;
    cursor: grab;
  }

  .brand {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    font-family: var(--font-display);
    font-weight: 600;
  }

  .mode {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 1px 7px;
    border: 0;
    border-radius: var(--radius-sm);
    font-family: var(--font-sans);
    font-size: 10px;
    font-weight: 600;
    color: var(--accent);
    background: var(--accent-soft);
    cursor: pointer;
    transition: filter 0.12s ease;
  }

  .mode:hover {
    filter: brightness(1.25);
  }

  .mode:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  .mode.on {
    color: var(--good);
    background: var(--good-soft);
  }

  .mode.waiting {
    color: var(--warn);
    background: var(--warn-soft);
  }

  .live-dot {
    width: 6px;
    height: 6px;
    border-radius: var(--radius-sm);
    background: currentColor;
  }

  .mode.on .live-dot {
    animation: pulse 1.6s ease-in-out infinite;
  }

  @keyframes pulse {
    50% {
      opacity: 0.35;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .mode.on .live-dot {
      animation: none;
    }
  }

  .notice {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    padding: 6px 10px 6px 12px;
    border-top: 1px solid var(--border);
    font-size: 11.5px;
    line-height: 1.4;
    color: var(--muted);
  }

  .notice > span {
    flex: 1;
  }

  .notice.warn {
    color: var(--warn);
    background: var(--warn-soft);
  }

  .notice.bad {
    color: var(--bad);
    background: color-mix(in srgb, var(--bad) 12%, transparent);
  }

  .notice.good {
    color: var(--good);
    background: var(--good-soft);
  }

  .dismiss {
    display: grid;
    flex: none;
    place-items: center;
    width: 18px;
    height: 18px;
    padding: 0;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: inherit;
    cursor: pointer;
  }

  .actions {
    display: inline-flex;
    gap: 2px;
  }

  .actions button,
  .pin {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--muted);
    cursor: pointer;
  }

  .actions button:hover,
  .pin:hover {
    background: rgba(255, 255, 255, 0.07);
    color: var(--text);
  }

  .actions button.active,
  .pin.active {
    color: var(--accent);
  }

  .tabs {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0 6px 0 8px;
    border-bottom: 1px solid var(--border);
  }

  .tablist {
    display: flex;
    flex: 1;
    min-width: 0;
    outline: none;
  }

  [role="tab"] {
    position: relative;
    display: inline-flex;
    flex: 1 1 auto;
    align-items: center;
    justify-content: center;
    gap: 4px;
    min-width: 0;
    height: 34px;
    padding: 0 3px;
    border: 0;
    border-bottom: 2px solid transparent;
    background: transparent;
    color: var(--muted);
    font-size: 12px;
    font-weight: 500;
    white-space: nowrap;
    cursor: pointer;
    transition:
      color 0.12s ease,
      border-color 0.12s ease;
  }

  [role="tab"]:hover {
    color: var(--text);
  }

  [role="tab"][aria-selected="true"] {
    border-bottom-color: var(--accent);
    color: var(--text);
  }

  [role="tab"].empty:not([aria-selected="true"]) {
    color: var(--faint);
  }

  [role="tab"]:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
    border-radius: var(--radius-sm);
  }

  .icon {
    position: relative;
    display: inline-grid;
  }

  .pinned .icon {
    color: var(--accent);
  }

  .dot {
    position: absolute;
    top: -2px;
    right: -3px;
    width: 6px;
    height: 6px;
    border: 1px solid var(--surface);
    border-radius: var(--radius-sm);
    background: var(--accent);
  }

  .main {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }

  .panel {
    display: grid;
    align-content: start;
    gap: 0;
  }

  @keyframes enter {
    from {
      opacity: 0;
      transform: translateY(3px);
    }
  }

  .welcome {
    display: grid;
    justify-items: center;
    gap: 6px;
    padding: 28px 18px;
    text-align: center;
    color: var(--muted);
  }

  .welcome h1 {
    margin: 6px 0 0;
    font-family: var(--font-display);
    font-size: 15px;
    font-weight: 600;
    color: var(--text);
  }

  .welcome p {
    margin: 0;
  }

  .hint {
    max-width: 260px;
    font-size: 12px;
    color: var(--faint);
  }

  .steps {
    display: grid;
    gap: 10px;
    margin: 12px 0 6px;
    padding: 0;
    list-style: none;
    text-align: left;
    font-size: 12px;
  }

  .steps li {
    display: flex;
    gap: 9px;
  }

  .n {
    display: grid;
    flex: none;
    place-items: center;
    width: 18px;
    height: 18px;
    border-radius: var(--radius-sm);
    background: var(--accent-soft);
    color: var(--accent);
    font-size: 10.5px;
    font-weight: 600;
  }

  .inline {
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--accent-strong);
    font-size: inherit;
    cursor: pointer;
  }

  .inline:hover,
  .inline:focus-visible {
    text-decoration: underline;
    outline: none;
  }

  .inline.small {
    font-size: 11.5px;
    color: var(--muted);
  }

  kbd {
    padding: 0 4px;
    border: 1px solid var(--border-strong);
    border-bottom-width: 2px;
    border-radius: var(--radius-sm);
    font-family: var(--font-sans);
    font-size: 10.5px;
    color: var(--muted);
  }
</style>
