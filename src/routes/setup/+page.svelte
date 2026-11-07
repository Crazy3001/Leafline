<script lang="ts">
  // First-launch setup: what Leafline is, the hotkeys, game settings that help it read, how to
  // show it (the overlay, or the desk view) and the map data. Nothing is bound or downloaded until
  // the player says so. Finishing starts live reading if it is on; closing the window without
  // finishing brings setup back next launch.
  import {
    AppWindow,
    ArrowLeft,
    ArrowRight,
    Check,
    ExternalLink,
    Layers,
    Monitor,
    MonitorUp,
    Sun,
    Type,
    X,
  } from "@lucide/svelte";
  import { onMount } from "svelte";

  import { backend } from "$lib/api";
  import HotkeyButton from "$lib/components/HotkeyButton.svelte";
  import Logo from "$lib/components/Logo.svelte";
  import MapDataPanel from "$lib/components/map/MapDataPanel.svelte";
  import { WIKI_URL, mapData } from "$lib/mapdata.svelte";
  import { changeSettings, display, keyName, quietHotkeys, usesNumpad, watchSettings } from "$lib/settings.svelte";
  import type { ShortcutInfo } from "$lib/types";

  const STEPS = ["Welcome", "Hotkeys", "Game settings", "Views", "Map data"];
  const SCREENS_STEP = 3;

  let step = $state(0);
  let shortcuts = $state<ShortcutInfo[]>([]);
  /** The keys being picked, by action, saved when the step is left. */
  let keys = $state<Record<string, string>>({});
  let keysError = $state<string | null>(null);
  /** After the keys are saved, the player presses each to prove it reaches Leafline. */
  let testing = $state(false);
  // A press during the test only ticks; the hotkeys work normally again when it ends.
  $effect(() => {
    if (testing) return quietHotkeys();
  });
  let arrived = $state<Record<string, boolean>>({});
  /** Screens Windows reports, or null until asked (or if it couldn't say). */
  let monitors = $state<number | null>(null);
  /** How the player wants Leafline: the overlay over the game, or the desk view. Only
   *  the desk view needs anything doing, at the end: it opens. */
  let place = $state<"overlay" | "desk" | null>(null);
  let deskOpened = $state(false);
  let finishing = $state(false);
  let finishError = $state<string | null>(null);

  onMount(() => {
    watchSettings();
    backend.shortcuts().then((list) => {
      shortcuts = list;
      // Keys already set win; otherwise offer the suggestions.
      keys = Object.fromEntries(list.map((s) => [s.action, s.keys || s.suggestedKeys]));
    });
    backend
      .monitorCount()
      .then((n) => {
        monitors = n;
        place ??= n > 1 ? "desk" : "overlay";
      })
      .catch(() => (place ??= "overlay"));
    const stop = backend.onHotkey((action) => (arrived[action] = true));
    return () => {
      stop.then((unlisten) => unlisten());
    };
  });


  /** The actions setup asks about: the main ones, plus any the player already set. */
  const offered = $derived(shortcuts.filter((s) => s.inSetup || s.keys));
  const taken = $derived(offered.filter((s) => s.taken && keys[s.action] === s.keys));
  /** The keys to press in the test: every action with one set. */
  const testRows = $derived(offered.filter((s) => s.keys));
  const testedAll = $derived(testRows.length > 0 && testRows.every((s) => arrived[s.action] || s.taken));
  const clash = $derived.by(() => {
    const seen = new Map<string, string>();
    for (const s of offered) {
      const k = keys[s.action];
      if (!k) continue;
      if (seen.has(k)) return `${k} is used twice.`;
      seen.set(k, s.action);
    }
    return null;
  });

  async function saveKeys(): Promise<boolean> {
    keysError = null;
    if (clash) {
      keysError = clash;
      return false;
    }
    try {
      await changeSettings({ hotkeys: { enabled: true, keys: { ...display.hotkeys.keys, ...keys } } });
      // A key another program holds can't be ours; say so before going on.
      shortcuts = await backend.shortcuts();
      return !shortcuts.some((s) => s.taken);
    } catch (e) {
      keysError = String(e);
      return false;
    }
  }

  async function next() {
    if (step === 1 && !testing) {
      if (!(await saveKeys())) return;
      arrived = {};
      testing = true;
      return;
    }
    testing = false;
    step = Math.min(step + 1, STEPS.length - 1);
  }

  function back() {
    if (step === 1 && testing) testing = false;
    else step -= 1;
  }

  async function tryDesk() {
    place = "desk";
    try {
      await backend.showDesk();
      deskOpened = true;
    } catch (e) {
      finishError = String(e);
    }
  }

  async function finish() {
    finishing = true;
    finishError = null;
    try {
      await changeSettings({ setupDone: true });
      if (place === "desk" && !deskOpened) await backend.showDesk();
      await backend.finishSetup();
    } catch (e) {
      finishError = String(e);
      finishing = false;
    }
  }

  const never = [
    "Read or change the game's memory, files or network traffic",
    "Draw inside the game. The overlay is its own window",
    "Press keys or click for you. Nothing is automated",
    "Send anything it reads off this PC",
  ];
</script>

<svelte:head><title>Leafline — Setup</title></svelte:head>

<div class="page">
  <header>
    <div class="brand"><Logo size={20} /><span>Leafline</span></div>
    <ol class="steps" aria-label="Setup steps">
      {#each STEPS as title, i (title)}
        <li class:current={i === step} class:done={i < step} aria-current={i === step ? "step" : undefined}>
          <span class="num tabular">{#if i < step}<Check size={11} />{:else}{i + 1}{/if}</span>{title}
        </li>
      {/each}
    </ol>
  </header>

  <main>
    {#if step === 0}
      <section>
        <h1>Welcome</h1>
        <p class="lead">
          Leafline sits beside MapleStory Classic and shows your EXP and meso rates, where you are and where to train,
          your quests, items and your build, from what the game shows on screen.
        </p>
        <p class="plain">
          Leafline is unofficial, and Nexon hasn't approved overlays like it. Use it at your own risk.
        </p>

        <div class="row">
          <label for="live">
            <span class="name">Read the game window while you play</span>
            <span class="help">
              A picture every {display.live.interval} seconds, the way OBS or Discord capture a window. Pictures stay in
              memory and are never saved unless you ask.
            </span>
          </label>
          <input
            id="live"
            class="switch"
            type="checkbox"
            role="switch"
            checked={display.live.enabled}
            onchange={(e) => changeSettings({ live: { ...display.live, enabled: e.currentTarget.checked } })}
          />
        </div>

        <div class="row">
          <label for="update-check">
            <span class="name">Check for updates at startup</span>
            <span class="help">
              Asks GitHub for the latest version number and says in Settings if there's a newer one. Nothing about the
              game or you is sent. Change it any time in Settings → About.
            </span>
          </label>
          <input
            id="update-check"
            class="switch"
            type="checkbox"
            role="switch"
            checked={display.updateCheck}
            onchange={(e) => changeSettings({ updateCheck: e.currentTarget.checked })}
          />
        </div>

        <h2>What Leafline never does</h2>
        <ul class="never">
          {#each never as item (item)}<li><span class="mark"><X size={12} /></span>{item}</li>{/each}
        </ul>
      </section>
    {:else if step === 1 && testing}
      <section>
        <h1>Test your hotkeys</h1>
        <p class="lead">
          Click into the game first, then press each one there. The game can keep some keys to itself while it's in
          front, so a key that only works with this window in front won't work while you play. While setup is open, a
          press only ticks it here; nothing else happens.
        </p>
        <dl class="keys">
          {#each testRows as shortcut (shortcut.action)}
            <div>
              <dt>{shortcut.label}</dt>
              <dd class="test">
                <kbd>{keyName(shortcut.keys)}</kbd>
                {#if shortcut.taken}
                  <span class="status warn">In use elsewhere</span>
                {:else if arrived[shortcut.action]}
                  <span class="status good"><Check size={12} />Works</span>
                {:else}
                  <span class="status">Press it</span>
                {/if}
              </dd>
            </div>
          {/each}
        </dl>
        {#if usesNumpad(display.hotkeys.keys)}
          <p class="note">Numpad keys only work while Num Lock is on.</p>
        {/if}
        {#if testRows.length === 0}
          <p class="note">No hotkeys are set. You can add them later in Settings.</p>
        {:else if testedAll}
          <p class="note good">All set.</p>
        {:else}
          <p class="note">
            One that doesn't tick never reached Leafline: the game keeps some keys to itself while it's in front, and
            another program can take one first. Go back and pick another; one with Win, such as Win+F10, usually gets
            through.
          </p>
        {/if}
      </section>
    {:else if step === 1}
      <section>
        <h1>Hotkeys</h1>
        <p class="lead">
          These work while the game has focus, and Leafline takes the key from the game. Each adds Win, which the game
          can't bind: a key on its own, even Pause or a numpad key, may never reach Leafline while the game is in front.
          Click one to change it. Next sets them, then you press each in the game to test it.
        </p>
        <dl class="keys">
          {#each offered as shortcut (shortcut.action)}
            <div>
              <dt>{shortcut.label}</dt>
              <dd>
                <HotkeyButton
                  keys={keys[shortcut.action] ?? ""}
                  taken={shortcut.taken && keys[shortcut.action] === shortcut.keys}
                  label={shortcut.label}
                  onchange={(k) => (keys[shortcut.action] = k)}
                />
              </dd>
            </div>
          {/each}
        </dl>
        <p class="note">
          Avoid Ctrl and Alt: the game sees them while you hold them, so a hotkey with Ctrl can make you attack. Esc
          cancels and Backspace clears. More hotkeys, such as pausing tracking, are in Settings.
        </p>
        {#if taken.length}
          <p class="note warn">
            Another program on this PC already uses {taken.map((s) => keyName(s.keys)).join(" and ")}, so Leafline can't.
            Pick {taken.length === 1 ? "another key" : "other keys"}, or clear {taken.length === 1 ? "it" : "them"}.
          </p>
        {/if}
        {#if keysError || clash}<p class="note warn">{keysError ?? clash}</p>{/if}
      </section>
    {:else if step === 2}
      <section>
        <h1>Game settings</h1>
        <p class="lead">Leafline reads the game's own text, so a few game settings help it read reliably.</p>
        <ul class="tips">
          <li>
            <span class="tip-icon"><Monitor size={15} /></span>
            <div>
              <span class="name">UI Size: Ideal Ratio</span>
              <span class="help">
                In the game's Graphics options. Leafline has been tested most with Ideal Ratio; other UI sizes may misread
                the HUD for now.
              </span>
            </div>
          </li>
          <li>
            <span class="tip-icon"><Sun size={15} /></span>
            <div>
              <span class="name">Windows HDR off</span>
              <span class="help">
                HDR washes the picture out. Turn it off in Windows display settings, then restart the game. Leafline warns
                you if it sees HDR on.
              </span>
            </div>
          </li>
          <li>
            <span class="tip-icon"><AppWindow size={15} /></span>
            <div>
              <span class="name">Windowed or Full Screen</span>
              <span class="help">
                Both work as long as Leafline's windows show above the game. Leafline finds the game window by itself; if
                it can't, pick it in Settings.
              </span>
            </div>
          </li>
          <li>
            <span class="tip-icon"><Type size={15} /></span>
            <div>
              <span class="name">English client</span>
              <span class="help">Other languages aren't read yet.</span>
            </div>
          </li>
        </ul>
      </section>
    {:else if step === SCREENS_STEP}
      <section>
        <h1>How to show Leafline</h1>
        <p class="lead">
          Leafline can sit over the game as a small overlay, or show every panel at once in one big window, the desk view.
          {#if monitors != null && monitors > 1}
            Windows has {monitors} screens here, so the desk view can have one to itself.
          {/if}
          You can switch or use both any time.
        </p>
        <div class="places" role="radiogroup" aria-label="How to show Leafline">
          <button class="place" role="radio" aria-checked={place === "overlay"} onclick={() => (place = "overlay")}>
            <span class="tip-icon"><Layers size={16} /></span>
            <span>
              <span class="name">Over the game</span>
              <span class="help">
                A small overlay beside the game, with tabs. Pin a tab into its own window, or shrink the overlay to a
                compact HUD.
              </span>
            </span>
          </button>
          <button class="place" role="radio" aria-checked={place === "desk"} onclick={() => (place = "desk")}>
            <span class="tip-icon"><MonitorUp size={16} /></span>
            <span>
              <span class="name">Desk view</span>
              <span class="help">
                Every panel side by side in one window you can move and resize, with the session graph, map details,
                quests and the training log in full. Put it on another screen, beside a windowed game, or bring it up
                between fights. The overlay stays for quick looks over the game.
              </span>
            </span>
          </button>
        </div>
        {#if place === "desk"}
          <div class="try">
            {#if deskOpened}
              <p class="note good">
                <Check size={12} /> The desk view is open. Put it wherever suits you; Leafline remembers where.
              </p>
            {:else}
              <button class="button" onclick={tryDesk}><MonitorUp size={13} />Open it now</button>
              <span class="help">Or it opens when you finish setup.</span>
            {/if}
          </div>
        {/if}
        <p class="note">
          Either way, the overlay's <MonitorUp size={11} /> button and the tray menu open the desk view any time.
        </p>
      </section>
    {:else}
      <section>
        <h1>Map data</h1>
        <p class="lead">
          The Map, Train and Quests tabs and the build planner need map, monster, quest, skill and item facts. They come
          from community data, are downloaded once and kept on this PC. Drop rates, quest turn-ins and taxi routes come
          from what players recorded on
          <button class="link" onclick={() => backend.openLink(WIKI_URL)}>mapleclassic.wiki<ExternalLink size={11} /></button>,
          fetched when first needed.
        </p>
        <div class="box"><MapDataPanel /></div>
        {#if !mapData.status?.ready}
          <p class="note">You can skip this and download it later in Settings → Data.</p>
        {/if}
      </section>
    {/if}
  </main>

  <footer>
    {#if finishError}<p class="note warn">{finishError}</p>{/if}
    <div class="nav">
      {#if step > 0}
        <button class="button" onclick={back}><ArrowLeft size={13} />{step === 1 && testing ? "Change keys" : "Back"}</button>
      {/if}
      <span class="spacer"></span>
      {#if step < STEPS.length - 1}
        <button class="button primary" onclick={next}>
          {step === 1 && !testing ? "Set and test" : "Next"}<ArrowRight size={13} />
        </button>
      {:else}
        <button class="button primary" onclick={finish} disabled={finishing}>
          <Check size={13} />{display.live.enabled ? "Start Leafline" : "Finish"}
        </button>
      {/if}
    </div>
  </footer>
</div>

<style>
  :global(body) {
    background: var(--bg);
  }

  .page {
    display: grid;
    grid-template-rows: auto 1fr auto;
    height: 100vh;
  }

  header {
    padding: 18px 28px 14px;
    border-bottom: 1px solid var(--border);
    background: var(--surface);
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 14px;
    font-family: var(--font-display);
    font-size: var(--text-lg);
    font-weight: 600;
  }

  .steps {
    display: flex;
    gap: 18px;
    margin: 0;
    padding: 0;
    list-style: none;
    color: var(--faint);
    font-size: var(--text-sm);
  }

  .steps li {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .steps li.current {
    color: var(--text);
  }

  .steps li.done {
    color: var(--muted);
  }

  .num {
    display: inline-grid;
    place-items: center;
    width: 18px;
    height: 18px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    font-size: var(--text-xs);
  }

  .current .num {
    border-color: var(--accent);
    color: var(--accent-strong);
  }

  .done .num {
    color: var(--good);
  }

  main {
    overflow-y: auto;
    padding: 22px 28px;
  }

  h1 {
    margin: 0 0 6px;
    font-family: var(--font-display);
    font-size: var(--text-xl);
    font-weight: 600;
  }

  h2 {
    margin: 22px 0 8px;
    font-size: var(--text-md);
    font-weight: 600;
  }

  .lead {
    margin: 0 0 14px;
    color: var(--muted);
  }

  .plain {
    margin: 0 0 6px;
    padding: 10px 12px;
    border-left: 2px solid var(--accent);
    background: var(--surface-2);
  }

  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 20px;
    padding: 14px 0;
    border-bottom: 1px solid var(--border);
  }

  .row label {
    display: grid;
    gap: 2px;
  }

  .name {
    display: block;
    font-weight: 500;
  }

  .help {
    display: block;
    color: var(--faint);
    font-size: var(--text-sm);
  }

  .never {
    display: grid;
    gap: 6px;
    margin: 0;
    padding: 0;
    list-style: none;
    color: var(--muted);
  }

  .never li {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .mark {
    display: inline-grid;
    color: var(--bad);
  }

  .keys {
    display: grid;
    margin: 0;
  }

  .keys div {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 8px 0;
  }

  .keys div + div {
    border-top: 1px solid var(--border);
  }

  dd {
    margin: 0;
  }

  dd.test {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  kbd {
    padding: 1px 6px;
    border: 1px solid var(--border-strong);
    border-bottom-width: 2px;
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    color: var(--text);
    font-family: var(--font-sans);
    font-size: var(--text-sm);
  }

  .status {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    min-width: 92px;
    color: var(--faint);
    font-size: var(--text-sm);
  }

  .status.good,
  .note.good {
    color: var(--good);
  }

  .status.warn {
    color: var(--warn);
  }

  .tips {
    display: grid;
    gap: 0;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .tips li {
    display: flex;
    gap: 12px;
    padding: 12px 0;
  }

  .tips li + li {
    border-top: 1px solid var(--border);
  }

  .tip-icon {
    display: inline-grid;
    flex: none;
    margin-top: 1px;
    color: var(--accent);
  }

  .places {
    display: grid;
    gap: 8px;
  }

  .place {
    display: flex;
    gap: 12px;
    width: 100%;
    padding: 12px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .place:hover {
    background: var(--surface-2);
  }

  .place[aria-checked="true"] {
    border-color: var(--accent);
    background: var(--accent-soft);
  }

  .place:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  .try {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
    margin-top: 12px;
  }

  .try .note {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0;
  }

  .box {
    padding: 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface);
  }

  .note {
    margin: 12px 0 0;
    color: var(--faint);
    font-size: var(--text-sm);
  }

  .note.warn {
    color: var(--warn);
  }

  footer {
    padding: 12px 28px 16px;
    border-top: 1px solid var(--border);
  }

  footer .note {
    margin: 0 0 8px;
  }

  .nav {
    display: flex;
    gap: 8px;
  }

  .spacer {
    flex: 1;
  }

  .button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 14px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text);
    cursor: pointer;
  }

  .button:hover:enabled {
    border-color: var(--accent);
  }

  .button.primary {
    border-color: var(--accent);
    background: var(--accent);
    color: #1c1305;
    font-weight: 600;
  }

  .button.primary:hover:enabled {
    background: var(--accent-strong);
  }

  .button:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .button:focus-visible,
  .link:focus-visible,
  .switch:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .link {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    padding: 0;
    border: 0;
    background: none;
    color: var(--accent);
    cursor: pointer;
  }

  /* The same square switch as src/lib/ui/Switch.svelte, on a checkbox so labels still work. */
  .switch {
    appearance: none;
    position: relative;
    flex: none;
    width: 36px;
    height: 20px;
    margin: 0;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface-3);
    cursor: pointer;
  }

  .switch::after {
    content: "";
    position: absolute;
    top: 2px;
    left: 2px;
    width: 14px;
    height: 14px;
    border-radius: 1px;
    background: var(--muted);
  }

  .switch:checked {
    border-color: var(--accent);
    background: var(--accent);
  }

  .switch:checked::after {
    transform: translateX(16px);
    background: var(--bg);
  }
</style>
