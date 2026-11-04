<script lang="ts">
  // Settings: how the overlay looks, travel in routes, the hotkeys, downloaded data, and what
  // Leafline is.
  import {
    CarTaxiFront,
    Check,
    ClipboardCopy,
    Download,
    ExternalLink,
    FilePenLine,
    FolderOpen,
    LoaderCircle,
    RefreshCw,
    ScrollText,
    Play,
    RotateCcw,
    ShieldCheck,
    Square,
    X,
  } from "@lucide/svelte";
  import { onMount } from "svelte";

  import { backend } from "$lib/api";
  import HotkeyButton from "$lib/components/HotkeyButton.svelte";
  import Logo from "$lib/components/Logo.svelte";
  import MapDataPanel from "$lib/components/map/MapDataPanel.svelte";
  import { OSMS_URL, WIKI_URL } from "$lib/mapdata.svelte";
  import { live, watchLive } from "$lib/readings.svelte";
  import { changeSettings, display, hotkey, keyHint, keyName, watchSettings } from "$lib/settings.svelte";
  import type {
    CompactShape,
    GameWindow,
    HotkeyChoice,
    LiveChoice,
    ShortcutInfo,
    TravelChoice,
    TravelFile,
    TravelInfo,
  } from "$lib/types";

  const SECTIONS = [
    { id: "live", title: "Live reading" },
    { id: "overlay", title: "Overlay" },
    { id: "routes", title: "Routes" },
    { id: "hotkeys", title: "Hotkeys" },
    { id: "data", title: "Data" },
    { id: "about", title: "About" },
  ];
  const SHAPES: { shape: CompactShape; label: string }[] = [
    { shape: "card", label: "Card" },
    { shape: "wide", label: "Wide" },
    { shape: "bar", label: "Bar" },
  ];
  const PAUSE_AFTER = [10, 15, 30, 60, 120];

  let shortcuts = $state<ShortcutInfo[]>([]);
  let version = $state("");
  let opacity = $state(100);
  let size = $state(100);
  let saveTimer: ReturnType<typeof setTimeout> | undefined;
  let sizeTimer: ReturnType<typeof setTimeout> | undefined;
  let hotkeyError = $state<string | null>(null);
  let framesFolder = $state("");
  let copiedReport = $state<string | null>(null);
  let exported = $state<string | null>(null);
  let travel = $state<TravelInfo | null>(null);
  let downloading = $state(false);
  let travelError = $state<string | null>(null);

  const KINDS: { key: keyof Omit<TravelChoice, "enabled">; kind: TravelFile["services"][number]["kind"]; label: string; help: string }[] = [
    { key: "cabs", kind: "cab", label: "Taxis", help: "Regular Cab between towns, and Phil out of Lith Harbor" },
    { key: "vipCabs", kind: "vipCab", label: "VIP Cab", help: "Straight to Ant Tunnel Park, for a much higher fare" },
    { key: "scrolls", kind: "scroll", label: "Return scrolls", help: "To a town from any map, if you carry one" },
  ];

  const INTERVALS = [1, 2, 3, 5];
  let windows = $state<GameWindow[]>([]);
  let liveError = $state<string | null>(null);

  function setLive(change: Partial<LiveChoice>) {
    changeSettings({ live: { ...display.live, ...change } });
  }

  async function loadWindows() {
    windows = await backend.liveWindows();
  }

  /** "" is automatic, "saved" a picked window that isn't open, otherwise an index into `windows`. */
  const pickedWindow = $derived.by(() => {
    const w = display.live.window;
    if (!w) return "";
    const i = windows.findIndex((c) => c.title === w.title && c.class === w.class);
    return i >= 0 ? String(i) : "saved";
  });
  const autoWindow = $derived(windows.filter((w) => w.likelyGame));

  function pickWindow(value: string) {
    if (value === "saved") return;
    const w = value === "" ? null : windows[Number(value)];
    setLive({ window: w ? { title: w.title, class: w.class } : null });
  }

  const liveRunning = $derived(live.liveStatus.state === "reading" || live.liveStatus.state === "waiting");

  async function toggleLive() {
    liveError = null;
    try {
      if (liveRunning) await backend.stopLive();
      else await backend.startLive();
    } catch (e) {
      liveError = String(e);
    }
  }

  function setTravel(change: Partial<TravelChoice>) {
    changeSettings({ travel: { ...display.travel, ...change } });
  }

  function loadTravel() {
    backend.travelInfo().then((t) => (travel = t));
  }

  async function editTravel() {
    await backend.editTravelFile();
    loadTravel();
  }

  async function downloadTravel() {
    downloading = true;
    travelError = null;
    try {
      travel = await backend.downloadTravel();
    } catch (e) {
      travelError = `Couldn't download: ${e}`;
    } finally {
      downloading = false;
    }
  }

  function fetchedOn(seconds: number | undefined) {
    return seconds ? new Date(seconds * 1000).toLocaleDateString(undefined, { dateStyle: "medium" }) : "";
  }

  /** A service's routes, grouped by where they leave from: `Henesys → Perion, Ellinia`. */
  function routeLines(service: TravelFile["services"][number]) {
    const groups = new Map<string, string[]>();
    for (const route of service.routes) {
      const from = route.from ?? "Any map";
      const fare = route.fare ? ` (${route.fare.toLocaleString()})` : "";
      groups.set(from, [...(groups.get(from) ?? []), route.to + fare]);
    }
    return [...groups].map(([from, to]) => ({ from, to: to.join(", ") }));
  }

  async function exportCsv(what: "prices" | "sessions") {
    try {
      const rows = await backend.exportCsv(what);
      if (rows != null) exported = `Saved ${rows} row${rows === 1 ? "" : "s"}.`;
    } catch (e) {
      exported = `Couldn't export: ${e}`;
    }
  }

  onMount(() => {
    watchSettings();
    watchLive();
    loadWindows();
    backend.shortcuts().then((s) => (shortcuts = s));
    backend.version().then((v) => (version = v));
    loadTravel();
  });

  // The folder follows the setting, including a change made in setup.
  $effect(() => {
    void display.framesFolder;
    backend.framesFolder().then((f) => (framesFolder = f));
  });

  async function pickFramesFolder() {
    const picked = await backend.pickFolder();
    if (picked) changeSettings({ framesFolder: picked });
  }

  /** OneDrive, Dropbox and Google Drive folders sync what is put in them off this PC. */
  const framesSync = $derived(/[\\/](OneDrive|Dropbox|Google Drive)[^\\/]*([\\/]|$)/i.test(framesFolder));

  $effect(() => {
    opacity = Math.round(display.opacity * 100);
    size = Math.round(display.scale * 100);
  });

  function slideSize(value: number) {
    size = value;
    clearTimeout(sizeTimer);
    sizeTimer = setTimeout(() => changeSettings({ scale: value / 100 }), 200);
  }

  async function setHotkeys(change: Partial<HotkeyChoice>) {
    hotkeyError = null;
    try {
      await changeSettings({ hotkeys: { ...display.hotkeys, ...change } });
    } catch (e) {
      hotkeyError = String(e);
      Object.assign(display, await backend.settings());
    }
    shortcuts = await backend.shortcuts();
  }

  function setKey(action: string, keys: string) {
    setHotkeys({ keys: { ...display.hotkeys.keys, [action]: keys } });
  }

  function useSuggested() {
    setHotkeys({ keys: Object.fromEntries(shortcuts.filter((s) => s.suggestedKeys).map((s) => [s.action, s.suggestedKeys])) });
  }

  /** Hotkeys holding Ctrl or Alt, which the game also sees while they are held. */
  const gameModifiers = $derived(shortcuts.filter((s) => /(^|\+)(Ctrl|Alt)\+/.test(s.keys)));

  async function copyReport() {
    try {
      await navigator.clipboard.writeText(await backend.diagnostics());
      copiedReport = "Copied. It holds no names, chat or file paths; paste it into your bug report.";
    } catch (e) {
      copiedReport = `Couldn't copy: ${e}`;
    }
  }

  /** Saves the slider a moment after it stops moving; every window follows. */
  function slide(value: number) {
    opacity = value;
    clearTimeout(saveTimer);
    saveTimer = setTimeout(() => changeSettings({ opacity: value / 100 }), 120);
  }

  const does = [
    "Reads what the game window shows (or saved screenshots), with the OCR built into Windows on this PC. Turn live reading off below.",
    "Shows map, monster, quest, skill and equipment facts from community data, downloaded only when you ask.",
    "Downloads the player-logged drop table from mapleclassic.wiki when you first need it, and who takes each quest in along with the map data. Both renew weekly and are looked up on this PC.",
    "Keeps your settings, sessions, item notes, price log and builds on this PC.",
  ];
  const doesNot = [
    "Read or change the game's memory or files.",
    "Read, block or change the game's network traffic.",
    "Inject into the game or draw inside it. The overlay is a separate window.",
    "Press keys or click for you. Nothing is automated: no botting, macros or auto-potions.",
    "Save pictures of your game unless you press Save frame, or upload screenshots, readings or records anywhere.",
  ];
  const credits = [
    {
      what: "Map, monster, NPC, quest, skill and equipment data",
      who: "OSMS Data Explorer",
      licence: "CC BY 4.0",
      url: OSMS_URL,
    },
    {
      what: "Player-observed drop rates, quest turn-ins, taxi routes and return scrolls",
      who: "mapleclassic.wiki contributors",
      licence: "CC BY-NC-SA 4.0",
      url: WIKI_URL,
    },
    { what: "Icons", who: "Lucide", licence: "ISC", url: "https://lucide.dev" },
    { what: "App framework", who: "Tauri", licence: "MIT / Apache-2.0", url: "https://tauri.app" },
    { what: "Interface", who: "Svelte", licence: "MIT", url: "https://svelte.dev" },
  ];
</script>

<svelte:head><title>Leafline — Settings</title></svelte:head>

<div class="page">
  <nav class="nav" aria-label="Settings sections">
    <div class="brand"><Logo size={18} /><span>Settings</span></div>
    {#each SECTIONS as section (section.id)}
      <a href="#{section.id}">{section.title}</a>
    {/each}
  </nav>

  <main>
    <section id="live">
      <h2>Live reading</h2>
      <p class="lead">
        Reads the game window every few seconds, the way OBS or Discord capture it, and keeps the overlay up to date as
        you play. Leafline only looks: it sends the game no keys, clicks or messages, and never touches its memory, files
        or network traffic.
      </p>

      <div class="row">
        <label for="live">
          <span class="name">Live reading</span>
          <span class="help">Leafline is unofficial and Nexon hasn't approved overlays, so use it at your own risk.</span>
        </label>
        <div class="control">
          <input
            id="live"
            class="switch"
            type="checkbox"
            role="switch"
            checked={display.live.enabled}
            onchange={(e) => setLive({ enabled: e.currentTarget.checked })}
          />
        </div>
      </div>

      <fieldset class="live-options" disabled={!display.live.enabled}>
        <legend class="sr-only">Live reading options</legend>
        <div class="row">
          <label for="window">
            <span class="name">Game window</span>
            <span class="help">
              {#if display.live.window}
                Your pick is remembered, even after the game restarts.
              {:else if autoWindow.length === 1}
                Found: {autoWindow[0].title}, {autoWindow[0].width}×{autoWindow[0].height}.
              {:else if autoWindow.length > 1}
                More than one window looks like the game. Pick one.
              {:else}
                The game isn't running, or its window has an unusual name. Pick it once it's open.
              {/if}
            </span>
          </label>
          <div class="control picker">
            <select id="window" value={pickedWindow} onchange={(e) => pickWindow(e.currentTarget.value)}>
              <option value="">Find it automatically</option>
              {#if pickedWindow === "saved" && display.live.window}
                <option value="saved">{display.live.window.title} (not open)</option>
              {/if}
              {#each windows as w, i (w.id)}
                <option value={String(i)}>{w.title} · {w.width}×{w.height}</option>
              {/each}
            </select>
            <button
              class="button icon-only"
              title="Look for windows again"
              aria-label="Look for windows again"
              onclick={loadWindows}
            >
              <RefreshCw size={13} />
            </button>
          </div>
        </div>

        <div class="row">
          <div class="label" id="interval-label">
            <span class="name">Read every</span>
            <span class="help">Shorter catches quick changes; longer uses less CPU.</span>
          </div>
          <div class="control segmented" role="radiogroup" aria-labelledby="interval-label">
            {#each INTERVALS as seconds (seconds)}
              <button
                role="radio"
                aria-checked={display.live.interval === seconds}
                onclick={() => setLive({ interval: seconds })}>{seconds} s</button
              >
            {/each}
          </div>
        </div>

        <div class="row">
          <div class="label" id="pause-label">
            <span class="name">Pause tracking after</span>
            <span class="help">How long without EXP before the clock stops, so breaks don't lower your rates.</span>
          </div>
          <div class="control segmented" role="radiogroup" aria-labelledby="pause-label">
            {#each PAUSE_AFTER as seconds (seconds)}
              <button
                role="radio"
                aria-checked={display.live.pauseAfter === seconds}
                onclick={() => setLive({ pauseAfter: seconds })}>{seconds} s</button
              >
            {/each}
          </div>
        </div>

        <div class="row">
          <div class="label">
            <span class="name">
              {#if live.liveStatus.state === "reading"}
                <span class="status on">Reading</span>{live.liveStatus.window}, {live.liveStatus.width}×{live.liveStatus
                  .height}
              {:else if live.liveStatus.state === "waiting"}
                <span class="status waiting">Waiting</span>
              {:else}
                <span class="status">Stopped</span>
              {/if}
            </span>
            <span class="help">
              {#if live.liveStatus.state === "reading"}
                {live.liveStatus.frames} reading{live.liveStatus.frames === 1 ? "" : "s"}, the last in {Math.round(
                  live.liveStatus.readMs,
                )} ms.
              {:else if live.liveStatus.state === "waiting"}
                {live.liveStatus.reason}
              {:else if live.liveStatus.state === "failed"}
                Stopped by an error: {live.liveStatus.message}
              {:else}
                Starting begins a new session.{#if hotkey("ToggleLive")}
                  {hotkey("ToggleLive")} starts and stops it from the game.{/if} It starts by itself with Leafline.
              {/if}
            </span>
          </div>
          <div class="control">
            <button class="button" onclick={toggleLive}>
              {#if liveRunning}<Square size={13} />Stop{:else}<Play size={13} />Start{/if}
            </button>
          </div>
        </div>
      </fieldset>
      {#if liveError}<p class="note warn">{liveError}</p>{/if}
      {#if live.liveStatus.state === "reading" && live.liveStatus.hdr}
        <p class="note warn">
          Windows HDR is on for the game's display, which washes the picture out and spoils readings. Turn HDR off in
          Windows display settings, then restart the game.
        </p>
      {/if}
      <p class="note">
        Works in windowed mode, and in Full Screen as long as the overlay shows above the game. Pictures stay in memory
        and are never saved, unless you press Save frame{keyHint("SaveFrame")} to keep one. Items are read one at a time
        when you hover one and use the Read item hotkey{keyHint("ReadItem")}, whether live reading is running or not. This
        switch has to be on for that too.
      </p>
    </section>

    <section id="overlay">
      <h2>Overlay</h2>
      <p class="lead">How the overlay and pinned panels look over the game.</p>

      <div class="row">
        <label for="opacity">
          <span class="name">Opacity</span>
          <span class="help">Lower it to see more of the game behind the panels.</span>
        </label>
        <div class="control slider">
          <input
            id="opacity"
            type="range"
            min="40"
            max="100"
            step="5"
            value={opacity}
            oninput={(e) => slide(Number(e.currentTarget.value))}
          />
          <span class="value tabular">{opacity}%</span>
        </div>
      </div>

      <div class="row">
        <label for="size">
          <span class="name">Size</span>
          <span class="help">Text and controls in the overlay and pinned panels.</span>
        </label>
        <div class="control slider">
          <input
            id="size"
            type="range"
            min="60"
            max="200"
            step="5"
            value={size}
            oninput={(e) => slideSize(Number(e.currentTarget.value))}
          />
          <span class="value tabular">{size}%</span>
        </div>
      </div>

      <div class="row">
        <label for="on-top">
          <span class="name">Always on top</span>
          <span class="help">Keep the overlay and pinned panels above the game.</span>
        </label>
        <div class="control">
          <input
            id="on-top"
            class="switch"
            type="checkbox"
            role="switch"
            checked={display.alwaysOnTop}
            onchange={(e) => changeSettings({ alwaysOnTop: e.currentTarget.checked })}
          />
        </div>
      </div>

      <div class="row">
        <div class="label" id="shape-label">
          <span class="name">Compact shape</span>
          <span class="help">Card has your quest counters. Wide is a row of four numbers. Bar is one thin strip.</span>
        </div>
        <div class="control segmented" role="radiogroup" aria-labelledby="shape-label">
          {#each SHAPES as { shape, label } (shape)}
            <button
              role="radio"
              aria-checked={display.compactShape === shape}
              onclick={() => changeSettings({ compactShape: shape })}>{label}</button
            >
          {/each}
        </div>
      </div>

      <div class="preview" style:opacity={display.opacity}>
        <span class="eyebrow">Preview</span>
        <span class="sample">Lv 30 · 22.6k EXP / hr</span>
      </div>
    </section>

    <section id="routes">
      <h2>Routes</h2>
      <p class="lead">
        Routes count portals. Turn on travel to count a taxi ride or a return scroll as one step too. Leafline only shows
        the way; it never moves your character.
      </p>

      <div class="row">
        <label for="travel">
          <span class="name">Taxis and return scrolls</span>
          <span class="help">Used in the Map tab's routes, training spots and quest hints. Off by default.</span>
        </label>
        <div class="control">
          <input
            id="travel"
            class="switch"
            type="checkbox"
            role="switch"
            checked={display.travel.enabled}
            onchange={(e) => setTravel({ enabled: e.currentTarget.checked })}
          />
        </div>
      </div>

      <fieldset class="kinds" disabled={!display.travel.enabled}>
        <legend class="sr-only">Kinds of travel</legend>
        {#each KINDS as kind (kind.key)}
          <label class="kind">
            <input
              type="checkbox"
              checked={display.travel[kind.key]}
              onchange={(e) => setTravel({ [kind.key]: e.currentTarget.checked })}
            />
            <span class="kind-icon">{#if kind.kind === "scroll"}<ScrollText size={13} />{:else}<CarTaxiFront size={13} />{/if}</span>
            <span>
              <span class="name">{kind.label}</span>
              <span class="help">{kind.help}</span>
            </span>
          </label>
        {/each}
      </fieldset>

      {#if travel}
        <div class="row">
          <div class="label">
            <span class="name">Taxi routes and scrolls</span>
            <span class="help">
              {#if travel.origin === "custom"}
                Your own copy is in use.
              {:else if travel.origin === "wiki"}
                Downloaded from mapleclassic.wiki{travel.file?.fetchedAt ? ` on ${fetchedOn(travel.file.fetchedAt)}` : ""}.
              {:else}
                Not downloaded yet. They come from what players recorded on mapleclassic.wiki, fetched only when you ask.
              {/if}
            </span>
          </div>
          <div class="control">
            <button class="button" onclick={downloadTravel} disabled={downloading}>
              {#if downloading}<span class="spin"><LoaderCircle size={13} /></span>{:else if travel.origin === "none"}<Download
                  size={13}
                />{:else}<RefreshCw size={13} />{/if}
              {travel.origin === "none" ? "Download" : "Update"}
            </button>
          </div>
        </div>
        {#if travelError}<p class="note warn">{travelError}</p>{/if}
        {#if travel.origin === "custom"}
          <p class="note">Update refreshes the download, but your own copy stays in use until you delete it.</p>
        {/if}
      {/if}

      {#if travel?.file}
        <details class="connections">
          <summary>
            The connections ({travel.file.services.length}){#if travel.origin === "custom"}<span class="tag">your copy</span>{/if}
          </summary>
          <ul>
            {#each travel.file.services as service (service.name)}
              <li>
                <div class="service-head">
                  <span class="kind-icon"
                    >{#if service.kind === "scroll"}<ScrollText size={12} />{:else}<CarTaxiFront size={12} />{/if}</span
                  >
                  <span class="name">{service.name}</span>
                  <button class="link source" onclick={() => backend.openLink(service.source)} title={service.source}>
                    Source<ExternalLink size={11} />
                  </button>
                </div>
                {#if service.note}<p class="help">{service.note}</p>{/if}
                {#each routeLines(service) as line (line.from)}
                  <p class="route-line"><span class="from">{line.from}</span> → {line.to}</p>
                {/each}
              </li>
            {/each}
          </ul>
          {#if travel.missing?.length}
            <p class="note warn">
              Not found in the map data, so left out of routes: {travel.missing.join(", ")}.
            </p>
          {/if}
        </details>
      {/if}
      {#if travel}
        {#if travel.error}
          <p class="note warn">Your travel file couldn't be read, so it isn't used: {travel.error}</p>
        {/if}
        <div class="row">
          <div class="label">
            <span class="name">Edit the connections</span>
            <span class="help">
              Opens your own copy{travel.origin === "wiki" ? ", made from the download" : ""}, which then replaces it.
              Fares are in mesos at level 20 and up. Delete your copy to go back to the download.
            </span>
          </div>
          <div class="control">
            <button class="button" onclick={editTravel}><FilePenLine size={13} />Edit file</button>
          </div>
        </div>
        <p class="note">
          Taxi routes, fares and scrolls are what players recorded on mapleclassic.wiki (CC BY-NC-SA 4.0), not the game's
          data, and may change. Check them in game.
        </p>
      {/if}
    </section>

    <section id="hotkeys">
      <h2>Hotkeys</h2>
      <p class="lead">
        They work while the game has focus, and Leafline takes the key from the game. F-keys, Pause and numpad keys
        work on their own; other keys need Ctrl, Alt or Win with them, so they keep working in chat. The game can't bind
        Pause, Win or the numpad. Leafline never sends keys to the game.
      </p>
      <div class="row">
        <label for="hotkeys">
          <span class="name">Hotkeys</span>
          <span class="help">Click one to change it, then press the keys. Esc cancels; Backspace clears it.</span>
        </label>
        <div class="control">
          <input
            id="hotkeys"
            class="switch"
            type="checkbox"
            role="switch"
            checked={display.hotkeys.enabled}
            onchange={(e) => setHotkeys({ enabled: e.currentTarget.checked })}
          />
        </div>
      </div>
      <dl class="keys">
        {#each shortcuts as shortcut (shortcut.action)}
          <div>
            <dt>{shortcut.label}</dt>
            <dd>
              <HotkeyButton
                keys={shortcut.keys}
                taken={shortcut.taken}
                label={shortcut.label}
                disabled={!display.hotkeys.enabled}
                onchange={(keys) => setKey(shortcut.action, keys)}
              />
            </dd>
          </div>
        {/each}
      </dl>
      {#if hotkeyError}<p class="note warn">{hotkeyError}</p>{/if}
      {#if shortcuts.some((s) => s.taken)}
        <p class="note warn">
          Another program on this PC already uses {shortcuts
            .filter((s) => s.taken)
            .map((s) => keyName(s.keys))
            .join(", ")}, so those hotkeys do nothing. Pick other keys.
        </p>
      {/if}
      {#if gameModifiers.length}
        <p class="note warn">
          The game also sees Ctrl and Alt while you hold them, so {gameModifiers.map((s) => s.label.toLowerCase()).join(", ")}
          can make your character attack or jump if those are your keys. An F-key, Pause or a numpad key avoids that.
        </p>
      {/if}
      <div class="reset-keys">
        <button class="button" onclick={useSuggested}>Use the suggested keys</button>
        {#if Object.values(display.hotkeys.keys).some(Boolean)}
          <button class="button" onclick={() => setHotkeys({ keys: {} })}>Clear all</button>
        {/if}
      </div>
    </section>

    <section id="data">
      <h2>Data</h2>
      <p class="lead">
        Map data powers the Map, Train and Quests tabs. Nothing is downloaded until you press the button, and nothing
        you record leaves this PC.
      </p>
      <div class="box"><MapDataPanel /></div>

      <div class="row">
        <div class="label">
          <span class="name">Your records</span>
          <span class="help">The price log and saved sessions, as CSV for a spreadsheet.</span>
        </div>
        <div class="control exports">
          <button class="button" onclick={() => exportCsv("prices")}><Download size={13} />Price log</button>
          <button class="button" onclick={() => exportCsv("sessions")}><Download size={13} />Sessions</button>
        </div>
      </div>
      {#if exported}<p class="note">{exported}</p>{/if}

      <div class="row">
        <div class="label">
          <span class="name">Data folder</span>
          <span class="help">Your records, settings and the last 7 daily backups of your records.</span>
        </div>
        <div class="control">
          <button class="button" onclick={() => backend.openDataFolder()}><FolderOpen size={13} />Open</button>
        </div>
      </div>

      <div class="row">
        <div class="label">
          <span class="name">Saved pictures</span>
          <span class="help">
            Where Save frame puts pictures of the game{display.framesFolder ? "" : ", a folder on this PC that doesn't sync"}:
            <span class="path">{framesFolder}</span>
          </span>
        </div>
        <div class="control exports">
          <button class="button" onclick={() => backend.openFramesFolder()}><FolderOpen size={13} />Open</button>
          <button class="button" onclick={pickFramesFolder}>Change…</button>
          {#if display.framesFolder}
            <button
              class="button icon-only"
              title="Back to the local folder"
              aria-label="Back to the local folder"
              onclick={() => changeSettings({ framesFolder: null })}><RotateCcw size={13} /></button
            >
          {/if}
        </div>
      </div>
      {#if framesSync}
        <p class="note warn">
          This folder looks like it syncs to the cloud, so saved pictures of your game would leave this PC.
        </p>
      {/if}

      <div class="row">
        <div class="label">
          <span class="name">Bug report details</span>
          <span class="help">
            Copies how Leafline is doing, for a bug report. Sent nowhere, and no names, chat or file paths.
          </span>
        </div>
        <div class="control">
          <button class="button" onclick={copyReport}><ClipboardCopy size={13} />Copy</button>
        </div>
      </div>
      {#if copiedReport}<p class="note">{copiedReport}</p>{/if}
    </section>

    <section id="about">
      <h2>About Leafline</h2>
      <p class="lead">
        A free, unofficial overlay for MapleStory Classic{version ? `, version ${version}` : ""}. It is not made,
        endorsed or supported by Nexon.
      </p>

      <div class="policy">
        <ShieldCheck size={16} />
        <p>
          <strong>Read-only.</strong> Leafline reads the game window while live reading is on, and saved screenshots.
          Nexon hasn't said whether read-only overlays are allowed; use Leafline at your own risk.
        </p>
      </div>

      <div class="lists">
        <div>
          <h3>What it does</h3>
          <ul>
            {#each does as item}<li><span class="mark good"><Check size={12} /></span>{item}</li>{/each}
          </ul>
        </div>
        <div>
          <h3>What it never does</h3>
          <ul>
            {#each doesNot as item}<li><span class="mark bad"><X size={12} /></span>{item}</li>{/each}
          </ul>
        </div>
      </div>

      <div class="row">
        <div class="label">
          <span class="name">First-launch setup</span>
          <span class="help">Hotkeys, game settings that help Leafline read, and the map data, in four steps.</span>
        </div>
        <div class="control">
          <button class="button" onclick={() => backend.showSetup()}>Run setup again</button>
        </div>
      </div>

      <h3>Credits and licences</h3>
      <table class="credits">
        <tbody>
          {#each credits as credit (credit.who)}
            <tr>
              <td>{credit.what}</td>
              <td>
                <button class="link" onclick={() => backend.openLink(credit.url)}>
                  {credit.who}<ExternalLink size={11} />
                </button>
              </td>
              <td class="licence">{credit.licence}</td>
            </tr>
          {/each}
        </tbody>
      </table>
      <p class="note">
        Monster drop rates are what players logged, not the game's own tables. Game numbers can differ in Classic; trust
        the game's own tooltips when they disagree. MapleStory is a trademark of Nexon.
      </p>
    </section>
  </main>
</div>

<style>
  :global(body) {
    background: var(--bg);
  }

  .reset-keys {
    display: flex;
    gap: 6px;
    margin-top: 8px;
  }

  .path {
    font-family: var(--font-mono);
    font-size: 11px;
    word-break: break-all;
  }

  .page {
    display: grid;
    grid-template-columns: 150px 1fr;
    height: 100vh;
  }

  .nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 18px 10px;
    border-right: 1px solid var(--border);
    background: var(--surface);
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0 8px 14px;
    font-family: var(--font-display);
    font-weight: 600;
  }

  .nav a {
    padding: 6px 10px;
    border-radius: var(--radius-sm);
    color: var(--muted);
    text-decoration: none;
  }

  .nav a:hover,
  .nav a:focus-visible {
    background: rgba(255, 255, 255, 0.05);
    color: var(--text);
    outline: none;
  }

  main {
    overflow-y: auto;
    padding: 6px 28px 40px;
    scroll-behavior: smooth;
  }

  section {
    padding: 22px 0 10px;
    border-bottom: 1px solid var(--border);
  }

  section:last-child {
    border-bottom: 0;
  }

  h2 {
    margin: 0 0 4px;
    font-family: var(--font-display);
    font-size: 17px;
    font-weight: 600;
  }

  h3 {
    margin: 18px 0 8px;
    font-size: 12.5px;
    font-weight: 600;
  }

  .lead {
    margin: 0 0 14px;
    color: var(--muted);
  }

  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 20px;
    padding: 12px 0;
  }

  .row + .row {
    border-top: 1px solid var(--border);
  }

  .row label,
  .label {
    display: grid;
    gap: 2px;
  }

  .name {
    font-weight: 500;
  }

  .help {
    color: var(--faint);
    font-size: 12px;
  }

  .control {
    flex: none;
  }

  .slider {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .slider input {
    width: 180px;
    accent-color: var(--accent);
  }

  .value {
    width: 38px;
    color: var(--muted);
    text-align: right;
  }

  .segmented {
    display: flex;
    padding: 2px;
    border-radius: var(--radius-sm);
    background: rgba(255, 255, 255, 0.05);
  }

  .segmented button {
    padding: 5px 12px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--muted);
    cursor: pointer;
  }

  .segmented button[aria-checked="true"] {
    background: var(--surface-3);
    color: var(--text);
  }

  .segmented button:focus-visible,
  .link:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  .switch {
    appearance: none;
    position: relative;
    width: 34px;
    height: 19px;
    margin: 0;
    border-radius: var(--radius-sm);
    background: var(--surface-3, rgba(255, 255, 255, 0.12));
    cursor: pointer;
    transition: background 0.15s ease;
  }

  .switch::after {
    content: "";
    position: absolute;
    top: 2px;
    left: 2px;
    width: 15px;
    height: 15px;
    border-radius: var(--radius-sm);
    background: var(--text);
    transition: transform 0.15s ease;
  }

  .switch:checked {
    background: var(--travel);
  }

  .switch:checked::after {
    transform: translateX(15px);
  }

  .switch:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .kinds {
    display: grid;
    gap: 2px;
    margin: 0 0 6px;
    padding: 0;
    border: 0;
  }

  .kinds:disabled {
    opacity: 0.45;
  }

  .kind {
    display: flex;
    align-items: flex-start;
    gap: 9px;
    padding: 6px 0;
    cursor: pointer;
  }

  .kind input {
    margin: 2px 0 0;
    accent-color: var(--travel);
  }

  .kind > span:last-child {
    display: grid;
    gap: 1px;
  }

  .kind-icon {
    display: inline-grid;
    flex: none;
    margin-top: 1px;
    color: var(--travel);
  }

  .connections {
    margin: 4px 0 2px;
    padding: 8px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
  }

  .connections summary {
    color: var(--muted);
    cursor: pointer;
  }

  .connections ul {
    display: grid;
    gap: 10px;
    margin: 10px 0 0;
    padding: 0;
    list-style: none;
  }

  .connections li {
    display: block;
    color: var(--text);
  }

  .service-head {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .source {
    margin-left: auto;
    font-size: 11.5px;
  }

  .connections .help,
  .route-line {
    margin: 2px 0 0 18px;
  }

  .route-line {
    color: var(--muted);
    font-size: 12px;
  }

  .route-line .from {
    color: var(--text);
  }

  .tag {
    margin-left: 8px;
    padding: 1px 6px;
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--travel) 16%, transparent);
    color: var(--travel);
    font-size: 11px;
  }

  .note.warn {
    color: var(--warn);
  }

  .spin {
    display: inline-grid;
    animation: spin 0.9s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .button:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }

  .exports {
    display: flex;
    gap: 6px;
  }

  .button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 5px 12px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text);
    cursor: pointer;
  }

  .button:hover {
    border-color: var(--accent);
  }

  .button:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  .preview {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin: 6px 0 12px;
    padding: 10px 12px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    background: var(--glass);
  }

  .sample {
    font-weight: 600;
  }

  .keys {
    display: grid;
    gap: 2px;
    margin: 0;
  }

  .keys div {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 7px 0;
  }

  .keys div + div {
    border-top: 1px solid var(--border);
  }

  dt {
    color: var(--text);
  }

  dd {
    display: flex;
    align-items: center;
    gap: 3px;
    margin: 0;
  }

  .note {
    margin: 10px 0 12px;
    color: var(--faint);
    font-size: 12px;
  }

  .box {
    padding: 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface-2);
    margin-bottom: 12px;
  }

  .policy {
    display: flex;
    gap: 10px;
    padding: 12px;
    border: 1px solid color-mix(in srgb, var(--good) 30%, transparent);
    border-radius: var(--radius);
    background: var(--good-soft);
    color: var(--muted);
  }

  .policy :global(svg) {
    flex: none;
    margin-top: 2px;
    color: var(--good);
  }

  .policy p {
    margin: 0;
  }

  .policy strong {
    color: var(--good);
  }

  .live-options {
    margin: 0;
    padding: 0;
    border: 0;
    border-top: 1px solid var(--border);
    transition: opacity 0.15s ease;
  }

  .live-options:disabled {
    opacity: 0.45;
  }

  .picker {
    display: flex;
    gap: 6px;
  }

  .picker select {
    width: 260px;
    padding: 5px 8px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    color: var(--text);
    font: inherit;
  }

  .picker select:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  .icon-only {
    padding: 5px 8px;
  }

  .status {
    display: inline-block;
    margin-right: 6px;
    padding: 0 7px;
    border-radius: var(--radius-sm);
    background: rgba(255, 255, 255, 0.06);
    color: var(--muted);
    font-size: 11px;
    font-weight: 600;
  }

  .status.on {
    background: var(--good-soft);
    color: var(--good);
  }

  .status.waiting {
    background: var(--warn-soft);
    color: var(--warn);
  }

  .lists {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 20px;
  }

  ul {
    display: grid;
    gap: 7px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li {
    display: flex;
    gap: 8px;
    color: var(--muted);
    font-size: 12.5px;
  }

  .mark {
    display: inline-grid;
    flex: none;
    place-items: center;
    width: 18px;
    height: 18px;
    border-radius: var(--radius-sm);
  }

  .mark.good {
    color: var(--good);
    background: var(--good-soft);
  }

  .mark.bad {
    color: var(--bad);
    background: color-mix(in srgb, var(--bad) 13%, transparent);
  }

  .credits {
    width: 100%;
    border-collapse: collapse;
    font-size: 12.5px;
  }

  .credits td {
    padding: 6px 0;
    border-top: 1px solid var(--border);
    color: var(--muted);
  }

  .credits tr:first-child td {
    border-top: 0;
  }

  .credits td + td {
    padding-left: 12px;
    white-space: nowrap;
  }

  .licence {
    text-align: right;
    color: var(--faint);
  }

  .link {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--text);
    cursor: pointer;
  }

  .link:hover {
    color: var(--accent-strong);
  }

  @media (max-width: 600px) {
    .page {
      grid-template-columns: 1fr;
      height: auto;
    }

    .nav {
      flex-direction: row;
      flex-wrap: wrap;
      border-right: 0;
      border-bottom: 1px solid var(--border);
    }

    .lists {
      grid-template-columns: 1fr;
    }
  }
</style>
