<script lang="ts">
  // Settings: how the overlay looks, travel in routes, the hotkeys, downloaded data, and what
  // Leafline is.
  import { CarTaxiFront, Check, Download, ExternalLink, FilePenLine, ScrollText, ShieldCheck, X } from "@lucide/svelte";
  import { onMount } from "svelte";

  import { backend } from "$lib/api";
  import Logo from "$lib/components/Logo.svelte";
  import MapDataPanel from "$lib/components/map/MapDataPanel.svelte";
  import { OSMS_URL, WIKI_URL } from "$lib/mapdata.svelte";
  import { changeSettings, display, watchSettings } from "$lib/settings.svelte";
  import type { ShortcutInfo, TravelChoice, TravelFile, TravelInfo } from "$lib/types";

  const SECTIONS = [
    { id: "overlay", title: "Overlay" },
    { id: "routes", title: "Routes" },
    { id: "hotkeys", title: "Hotkeys" },
    { id: "data", title: "Data" },
    { id: "about", title: "About" },
  ];
  const SIZES = [
    { scale: 0.9, label: "Small" },
    { scale: 1, label: "Normal" },
    { scale: 1.15, label: "Large" },
    { scale: 1.3, label: "Larger" },
  ];

  let shortcuts = $state<ShortcutInfo[]>([]);
  let version = $state("");
  let opacity = $state(100);
  let saveTimer: ReturnType<typeof setTimeout> | undefined;
  let exported = $state<string | null>(null);
  let travel = $state<TravelInfo | null>(null);

  const KINDS: { key: keyof Omit<TravelChoice, "enabled">; kind: TravelFile["services"][number]["kind"]; label: string; help: string }[] = [
    { key: "cabs", kind: "cab", label: "Taxis", help: "Regular Cab between towns, and Phil out of Lith Harbor" },
    { key: "vipCabs", kind: "vipCab", label: "VIP Cab", help: "Straight to Ant Tunnel Park, for a much higher fare" },
    { key: "scrolls", kind: "scroll", label: "Return scrolls", help: "To a town from any map, if you carry one" },
  ];

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
    backend.shortcuts().then((s) => (shortcuts = s));
    backend.version().then((v) => (version = v));
    loadTravel();
  });

  $effect(() => {
    opacity = Math.round(display.opacity * 100);
  });

  /** Saves the slider a moment after it stops moving; every window follows. */
  function slide(value: number) {
    opacity = value;
    clearTimeout(saveTimer);
    saveTimer = setTimeout(() => changeSettings({ opacity: value / 100 }), 120);
  }

  const does = [
    "Reads what your screen shows, from screenshots, with the OCR built into Windows on this PC.",
    "Shows map, monster and quest facts from community data, downloaded only when you ask.",
    "Looks up player-logged drops on mapleclassic.wiki when you ask, a request a second at most.",
    "Keeps your settings, sessions and price log on this PC.",
  ];
  const doesNot = [
    "Read or change the game's memory or files.",
    "Read, block or change the game's network traffic.",
    "Inject into the game or draw inside it. The overlay is a separate window.",
    "Press keys or click for you. Nothing is automated: no botting, macros or auto-potions.",
    "Upload your screenshots, readings or records anywhere.",
  ];
  const credits = [
    {
      what: "Map, monster, NPC and quest data",
      who: "OSMS Data Explorer",
      licence: "CC BY 4.0",
      url: OSMS_URL,
    },
    {
      what: "Player-observed drop rates, taxi routes and return scrolls",
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
        <div class="label" id="size-label">
          <span class="name">Size</span>
          <span class="help">Text and controls in the overlay and pinned panels.</span>
        </div>
        <div class="control segmented" role="radiogroup" aria-labelledby="size-label">
          {#each SIZES as size (size.scale)}
            <button
              role="radio"
              aria-checked={Math.abs(display.scale - size.scale) < 0.01}
              onclick={() => changeSettings({ scale: size.scale })}>{size.label}</button
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
        <details class="connections">
          <summary>
            The connections ({travel.file.services.length}){#if travel.custom}<span class="tag">your copy</span>{/if}
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
        {#if travel.error}<p class="note warn">Your travel file couldn't be read, so Leafline's own list is used: {travel.error}</p>{/if}
        <div class="row">
          <div class="label">
            <span class="name">Edit the connections</span>
            <span class="help">
              {travel.custom ? "Your copy is in use" : "Leafline's list is in use"}{travel.file.checked
                ? `, checked against mapleclassic.wiki on ${travel.file.checked}`
                : ""}. Fares are in mesos at level 20 and up. Delete your copy to go back to Leafline's list.
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
      <p class="lead">They work while the game has focus. Leafline never sends keys to the game.</p>
      <dl class="keys">
        {#each shortcuts as shortcut (shortcut.action)}
          <div>
            <dt>{shortcut.label}</dt>
            <dd>{#each shortcut.keys.split("+") as key, i}{#if i}<span class="plus">+</span>{/if}<kbd>{key}</kbd>{/each}</dd>
          </div>
        {/each}
      </dl>
      <p class="note">Hotkeys can't be changed yet.</p>
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
          <strong>Offline mode.</strong> Leafline reads saved screenshots only. Live screen capture stays off until Nexon
          says whether read-only overlays are allowed; until then, use Leafline at your own risk.
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
    border-radius: 5px;
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
    border-radius: 999px;
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
    border-radius: 999px;
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
    border-radius: 999px;
    background: color-mix(in srgb, var(--travel) 16%, transparent);
    color: var(--travel);
    font-size: 11px;
  }

  .note.warn {
    color: var(--warn);
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

  kbd {
    min-width: 22px;
    padding: 1px 6px;
    border: 1px solid var(--border-strong);
    border-bottom-width: 2px;
    border-radius: 5px;
    background: var(--surface-2);
    color: var(--muted);
    font-family: var(--font-sans);
    font-size: 11.5px;
    text-align: center;
  }

  .plus {
    color: var(--faint);
    font-size: 11px;
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
    border-radius: 50%;
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
