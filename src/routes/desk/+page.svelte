<script lang="ts">
  // The desk view: the overlay's panels, whole, side by side in one window, often on a second
  // screen but just as well beside a windowed game.
  // It reads what the overlay reads, follows the overlay's "Go here" trip, and opens links in a
  // column of its own so the overlay needn't come up over the game.
  import { ArrowLeft, ChartLine, ChevronDown, Milestone, Pin, PinOff, Plus, X } from "@lucide/svelte";
  import { onMount, untrack } from "svelte";

  import { backend } from "$lib/api";
  import NextStep from "$lib/components/NextStep.svelte";
  import PanelView from "$lib/components/PanelView.svelte";
  import TrainingLog from "$lib/components/TrainingLog.svelte";
  import BuildPlanner from "$lib/components/planner/BuildPlanner.svelte";
  import {
    DESK_PANELS,
    MAX_COLUMNS,
    ZOOMS,
    find,
    layoutOf,
    loadPrefs,
    place,
    remove,
    savePrefs,
    split,
    unused,
    weight,
    withColumns,
    type DeskColumn,
    type DeskPanel,
    type PresetId,
  } from "$lib/desk";
  import { itemView, watchItems } from "$lib/items.svelte";
  import { monsterView, watchMonsters } from "$lib/monsters.svelte";
  import { deskWindow, logRequest, plannerRequest, tabRequest } from "$lib/nav.svelte";
  import { panelInfo } from "$lib/panels";
  import { placeLinks, watchPlaces } from "$lib/places.svelte";
  import { questView, watchQuests } from "$lib/quests.svelte";
  import { live, watchLive } from "$lib/readings.svelte";
  import { watchSettings } from "$lib/settings.svelte";
  import { watchTyping } from "$lib/typing";
  import Segmented from "$lib/ui/Segmented.svelte";
  import { followTrip } from "$lib/wayfinder.svelte";

  // Links to the training log open it in a column here, not in its own window.
  deskWindow.active = true;

  let prefs = $state(loadPrefs());
  const columns = $derived(layoutOf(prefs));
  let onTop = $state(false);

  /** A panel a link brought up where no column showed it, over another until put back. */
  let visit = $state<{ column: number; slot: number; panel: DeskPanel } | null>(null);
  /** The column and place whose Change menu is open. */
  let menu = $state<{ column: number; slot: number } | null>(null);

  // Clicks leave the game focused; typing in a box takes focus while it lasts.
  onMount(() => watchTyping());

  onMount(() => {
    watchLive();
    watchSettings();
    watchItems();
    watchMonsters();
    watchPlaces();
    watchQuests();
    followTrip();
    backend.deskStatus().then((s) => (onTop = s.onTop));
    const close = (event: KeyboardEvent) => {
      if (event.key === "Escape") menu = null;
    };
    window.addEventListener("keydown", close);
    return () => window.removeEventListener("keydown", close);
  });

  $effect(() => {
    void backend.setPageZoom(prefs.zoom);
  });

  function change(next: DeskColumn[]) {
    visit = null;
    menu = null;
    prefs.mine = next;
    prefs.preset = "mine";
    savePrefs($state.snapshot(prefs));
  }

  function choose(preset: PresetId) {
    visit = null;
    menu = null;
    prefs.preset = preset;
    savePrefs($state.snapshot(prefs));
  }

  function setZoom(zoom: number) {
    prefs.zoom = zoom;
    savePrefs($state.snapshot(prefs));
  }

  async function toggleOnTop() {
    onTop = (await backend.setDeskOnTop(!onTop)).onTop;
  }

  /** What a place shows now: the panel a link brought up there, or the layout's own. */
  function shown(column: number, slot: number): DeskPanel {
    return visit && visit.column === column && visit.slot === slot ? visit.panel : columns[column][slot];
  }

  /** Makes sure a panel is in view after a link, taking over the last column's top for a while. */
  function bringUp(panel: DeskPanel) {
    // Only the link itself counts: a later change of layout mustn't bring it up again.
    untrack(() => {
      if (find(columns, panel)) visit = null;
      else visit = { column: columns.length - 1, slot: 0, panel };
    });
  }

  // A link to an item, monster, map, NPC or quest, from this window or another, shows here.
  $effect(() => {
    if (itemView.seen || monsterView.seen || placeLinks.seen) bringUp("item");
  });

  $effect(() => {
    if (questView.seen) bringUp("quests");
  });

  // A panel's link to another one, such as the Character panel's line to Train.
  $effect(() => {
    if (tabRequest.seq && tabRequest.panel) bringUp(tabRequest.panel);
  });

  $effect(() => {
    if (logRequest.seq) bringUp("log");
  });

  $effect(() => {
    if (plannerRequest.seq) bringUp("planner");
  });

  // Reading the character's windows by hotkey shows its result in the Character panel.
  $effect(() => {
    const read = live.characterRead;
    if (read && !read.problem && !read.automatic) bringUp("character");
  });

  $effect(() => {
    if (live.itemRead?.found) bringUp("item");
  });

  const presetOptions = $derived([
    { id: "training" as const, label: "Training" },
    { id: "questing" as const, label: "Questing" },
    { id: "log" as const, label: "Log" },
    { id: "planner" as const, label: "Build planner" },
    ...(prefs.mine ? [{ id: "mine" as const, label: "My layout" }] : []),
  ]);

  const columnOptions = Array.from({ length: MAX_COLUMNS }, (_, i) => ({ id: i + 1, label: `${i + 1}` }));
  const zoomOptions = ZOOMS.map((z) => ({ id: z, label: `${Math.round(z * 100)}%` }));

  /** Where else a panel is, for the Change menu. */
  function whereElse(panel: DeskPanel, column: number, slot: number): string | null {
    const at = find(columns, panel);
    if (!at || (at[0] === column && at[1] === slot)) return null;
    return at[0] === column ? (at[1] === 0 ? "Above" : "Below") : `Column ${at[0] + 1}`;
  }

  /** A panel's name and icon. The training log and the planner aren't overlay tabs. */
  function deskInfo(panel: DeskPanel) {
    if (panel === "log") return { title: "Training log", icon: ChartLine };
    if (panel === "planner") return { title: "Build planner", icon: Milestone };
    return panelInfo(panel);
  }

  const template = $derived(columns.map((c) => `minmax(0, ${weight(c)}fr)`).join(" "));

  const hud = $derived(live.analysis?.hud ?? null);
  const liveText = $derived.by(() => {
    const s = live.liveStatus;
    if (s.state === "reading") return { tone: "good", text: `Reading ${s.window}` };
    if (s.state === "waiting") return { tone: "warn", text: s.reason };
    if (s.state === "failed") return { tone: "bad", text: `Live reading stopped: ${s.message}` };
    return { tone: "quiet", text: "Not reading the game" };
  });

  function closeMenuOutside(event: MouseEvent) {
    if (menu && !(event.target as Element).closest?.(".menu, .change")) menu = null;
  }
</script>

<svelte:head><title>Leafline — Desk view</title></svelte:head>
<svelte:window onclick={closeMenuOutside} />

<div class="desk">
  <header>
    <div class="who">
      <span class="brand">Leafline</span>
      {#if hud?.name || hud?.level}
        <span class="sep" aria-hidden="true"></span>
        {#if hud?.name}<span class="name">{hud.name}</span>{/if}
        {#if hud?.level}<span class="num muted">Lv {hud.level}{hud.job ? ` ${hud.job}` : ""}</span>{/if}
      {/if}
      <span class="status {liveText.tone}"><span class="dot" aria-hidden="true"></span>{liveText.text}</span>
    </div>
    <div class="controls">
      <Segmented label="Layout" options={presetOptions} value={prefs.preset} onchange={choose} />
      <span class="field">
        <span class="faint">Columns</span>
        <Segmented
          label="Columns"
          size="sm"
          options={columnOptions}
          value={columns.length}
          onchange={(n) => change(withColumns(columns, n))}
        />
      </span>
      <span class="field">
        <span class="faint">Text</span>
        <Segmented label="Text size" size="sm" options={zoomOptions} value={prefs.zoom} onchange={setZoom} />
      </span>
      <button
        class="ontop"
        aria-pressed={onTop}
        title={onTop ? "Stop keeping on top" : "Keep on top"}
        onclick={toggleOnTop}
        >{#if onTop}<PinOff size={13} />{:else}<Pin size={13} />{/if}Keep on top</button
      >
    </div>
  </header>

  <NextStep />

  <main style:grid-template-columns={template}>
    {#each columns as column, c (c)}
      <div class="column">
        {#each column as _, s (s)}
          {@const panel = shown(c, s)}
          {@const info = deskInfo(panel)}
          {@const Icon = info.icon}
          {@const visiting = visit?.column === c && visit?.slot === s}
          <section class="panel" aria-label={info.title}>
            <div class="panel-head">
              <h2><Icon size={14} />{info.title}</h2>
              <span class="actions">
                {#if visiting}
                  <button class="small" onclick={() => (visit = null)}
                    ><ArrowLeft size={12} />Back to {deskInfo(columns[c][s]).title}</button
                  >
                {/if}
                <button
                  class="small change"
                  aria-expanded={menu?.column === c && menu?.slot === s}
                  onclick={() => (menu = menu?.column === c && menu?.slot === s ? null : { column: c, slot: s })}
                  >Change<ChevronDown size={12} /></button
                >
              </span>
              {#if menu?.column === c && menu?.slot === s}
                <div class="menu" role="menu">
                  <p class="menu-label">Show here</p>
                  {#each DESK_PANELS as option (option)}
                    {@const elsewhere = whereElse(option, c, s)}
                    <button
                      role="menuitemradio"
                      aria-checked={option === columns[c][s]}
                      class="item"
                      class:current={option === columns[c][s]}
                      onclick={() => change(place(columns, c, s, option))}
                    >
                      <span>{deskInfo(option).title}</span>
                      {#if option === columns[c][s]}<span class="note">Here now</span>
                      {:else if elsewhere}<span class="note">{elsewhere}, swaps</span>{/if}
                    </button>
                  {/each}
                  <div class="menu-rule"></div>
                  {#if column.length < 2 && unused(columns).length}
                    <button role="menuitem" class="item" onclick={() => change(split(columns, c))}
                      ><span><Plus size={12} /> Add {deskInfo(unused(columns)[0]).title} below</span></button
                    >
                  {/if}
                  {#if columns.length > 1 || column.length > 1}
                    <button role="menuitem" class="item" onclick={() => change(remove(columns, c, s))}
                      ><span><X size={12} /> Remove {info.title} from the desk</span></button
                    >
                  {/if}
                </div>
              {/if}
            </div>
            <div class="panel-body">
              {#key panel}
                {#if panel === "log"}
                  <TrainingLog embedded />
                {:else if panel === "planner"}
                  <BuildPlanner embedded />
                {:else}
                  <PanelView {panel} analysis={live.analysis} session={live.session} />
                {/if}
              {/key}
            </div>
          </section>
        {/each}
      </div>
    {/each}
  </main>
</div>

<style>
  :global(html),
  :global(body) {
    background: var(--bg);
    overflow: hidden;
  }

  .desk {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--bg);
  }

  header {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2) var(--space-4);
    padding: var(--space-2) var(--space-4);
    border-bottom: 1px solid var(--border-strong);
    background: var(--surface);
  }

  .who,
  .controls {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2) var(--space-3);
    min-width: 0;
  }

  .brand {
    font-family: var(--font-display);
    font-size: var(--text-xl);
    font-weight: 600;
    color: var(--accent);
  }

  .sep {
    width: 1px;
    height: 20px;
    background: var(--border);
  }

  .name {
    font-weight: 600;
  }

  .num {
    font-family: var(--font-display);
    font-variant-numeric: tabular-nums;
  }

  .muted {
    color: var(--muted);
  }

  .faint {
    color: var(--faint);
    font-size: var(--text-sm);
  }

  .status {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: var(--text-sm);
    color: var(--faint);
  }

  .status .dot {
    width: 7px;
    height: 7px;
    border-radius: 1px;
    background: currentColor;
  }

  .status.good {
    color: var(--good);
  }

  .status.warn {
    color: var(--warn);
  }

  .status.bad {
    color: var(--bad);
  }

  .field {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
  }

  button.ontop,
  button.small {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: var(--control-h);
    padding: 0 10px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    color: var(--text);
    font-size: var(--text-sm);
    white-space: nowrap;
    cursor: pointer;
  }

  button.small {
    height: var(--control-h-sm);
    padding: 0 8px;
    font-size: var(--text-xs);
  }

  button.ontop:hover,
  button.small:hover {
    background: var(--surface-3);
  }

  button.ontop[aria-pressed="true"] {
    border-color: color-mix(in srgb, var(--accent) 60%, transparent);
    background: var(--accent-soft);
    color: var(--accent-strong);
  }

  button:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  /* The Go here bar runs the window's width, under the header. */
  .desk > :global(.bar) {
    margin: 0;
    border-radius: 0;
  }

  main {
    flex: 1;
    min-height: 0;
    display: grid;
    gap: 1px;
    background: var(--border);
  }

  .column {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
    min-height: 0;
  }

  .panel {
    flex: 1 1 0;
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--surface);
  }

  .panel-head {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    height: 40px;
    flex: none;
    padding: 0 var(--space-3) 0 var(--space-4);
    border-bottom: 1px solid var(--border);
  }

  h2 {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--text-lg);
    font-weight: 600;
  }

  .actions {
    display: inline-flex;
    gap: var(--space-2);
  }

  .menu {
    position: absolute;
    top: 36px;
    right: var(--space-3);
    z-index: 10;
    width: 260px;
    padding: 6px 0;
    background: var(--surface-3);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    box-shadow: var(--shadow);
  }

  .menu-label {
    margin: 0;
    padding: 4px 12px;
    font-size: var(--text-xs);
    color: var(--faint);
  }

  .menu-rule {
    margin: 6px 0;
    border-top: 1px solid var(--border);
  }

  .item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    height: 32px;
    padding: 0 12px;
    border: 0;
    background: transparent;
    color: var(--text);
    font-size: var(--text-md);
    text-align: left;
    cursor: pointer;
  }

  .item span {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .item:hover {
    background: rgba(255, 255, 255, 0.06);
  }

  .item.current {
    background: var(--accent-soft);
    color: var(--accent-strong);
  }

  .note {
    font-size: var(--text-xs);
    color: var(--faint);
  }

  .item.current .note {
    color: inherit;
  }

  .panel-body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: var(--space-3) var(--space-4) var(--space-4);
  }
</style>
