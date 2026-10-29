<script lang="ts">
  import { Lock, LockOpen, PanelsTopLeft, Pin, PinOff, X } from "@lucide/svelte";
  import { onMount } from "svelte";

  import { backend } from "$lib/api";
  import Logo from "$lib/components/Logo.svelte";
  import PanelView from "$lib/components/PanelView.svelte";
  import { PANELS, panelInfo, signature } from "$lib/panels";
  import { live, watchLive } from "$lib/readings.svelte";
  import type { Panel } from "$lib/types";

  const TAB_KEY = "leafline.overlay.tab";

  let tab = $state<Panel>(storedTab());
  /** What each panel showed when last looked at, to mark tabs with something new. */
  let seen = $state<Partial<Record<Panel, string | null>>>({});

  onMount(watchLive);

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

  function onTabKey(event: KeyboardEvent) {
    const i = PANELS.findIndex((p) => p.id === tab);
    if (event.key === "ArrowRight") select(PANELS[(i + 1) % PANELS.length].id);
    else if (event.key === "ArrowLeft") select(PANELS[(i - 1 + PANELS.length) % PANELS.length].id);
    else return;
    event.preventDefault();
  }
</script>

<svelte:head><title>Leafline</title></svelte:head>

<div class="shell" class:locked={live.status.clickThrough}>
  <header data-tauri-drag-region>
    <span class="brand" data-tauri-drag-region>
      <Logo size={16} />
      <span data-tauri-drag-region>Leafline</span>
      <span class="mode" data-tauri-drag-region>Offline</span>
    </span>
    <span class="actions">
      <button title="Open workbench (Ctrl+Alt+W)" onclick={() => backend.showWorkbench()}>
        <PanelsTopLeft size={14} />
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
      <button title="Hide (Ctrl+Alt+O). Pinned panels stay." onclick={() => backend.hideOverlay()}>
        <X size={14} />
      </button>
    </span>
  </header>

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
          onclick={() => select(panel.id)}
        >
          <Icon size={14} />
          <span>{panel.title}</span>
          {#if fresh(panel.id)}<span class="dot" aria-label="Updated"></span>{/if}
          {#if live.pins.pinned.includes(panel.id)}<span class="pinmark" title="Pinned"><Pin size={9} /></span>{/if}
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
        <p>Nothing to show yet.</p>
        <p class="hint">
          Open the workbench with <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>W</kbd> and load a screenshot. Its readings
          appear here, just as they will during play.
        </p>
      </div>
    {/if}
  </div>
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

  .shell.locked {
    opacity: 0.82;
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
    letter-spacing: 0.01em;
  }

  .mode {
    padding: 1px 6px;
    border-radius: 999px;
    font-family: var(--font-sans);
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.04em;
    color: var(--accent);
    background: var(--accent-soft);
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
    flex: 1;
    align-items: center;
    justify-content: center;
    gap: 5px;
    height: 34px;
    padding: 0 4px;
    border: 0;
    border-bottom: 2px solid transparent;
    background: transparent;
    color: var(--muted);
    font-size: 12px;
    font-weight: 500;
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
    border-radius: 4px;
  }

  .dot {
    align-self: flex-start;
    margin: 9px 0 0 -3px;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent);
  }

  .pinmark {
    display: inline-grid;
    color: var(--accent);
    opacity: 0.85;
  }

  .main {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }

  .panel {
    display: grid;
    align-content: start;
    gap: 8px;
    padding: 8px;
    animation: enter 0.16s ease-out;
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
    gap: 4px;
    padding: 40px 20px;
    text-align: center;
    color: var(--muted);
  }

  .welcome p {
    margin: 0;
  }

  .hint {
    max-width: 240px;
    font-size: 12px;
    color: var(--faint);
  }

  kbd {
    padding: 0 4px;
    border: 1px solid var(--border-strong);
    border-bottom-width: 2px;
    border-radius: 4px;
    font-family: var(--font-sans);
    font-size: 10.5px;
    color: var(--muted);
  }
</style>
