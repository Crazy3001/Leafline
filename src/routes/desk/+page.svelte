<script lang="ts">
  // The desk view: the overlay's panels, whole, side by side in one window, often on a second
  // screen but just as well beside a windowed game.
  // It reads what the overlay reads, follows the overlay's "Go here" trip, and opens links in a
  // column of its own so the overlay needn't come up over the game.
  import { tip } from "$lib/tip.svelte";
  import { ArrowLeft, ChartLine, ChevronDown, Milestone, Pencil, Pin, PinOff, Plus, RotateCcw, X } from "@lucide/svelte";
  import { onMount, untrack } from "svelte";

  import { backend } from "$lib/api";
  import NextStep from "$lib/components/NextStep.svelte";
  import PanelView from "$lib/components/PanelView.svelte";
  import TrainingLog from "$lib/components/TrainingLog.svelte";
  import BuildPlanner from "$lib/components/planner/BuildPlanner.svelte";
  import {
    DESK_PANELS,
    MAX_COLUMNS,
    MAX_NAME,
    ZOOMS,
    defaultTab,
    find,
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
  } from "$lib/desk";
  import { itemView, watchItems } from "$lib/items.svelte";
  import { monsterView, watchMonsters } from "$lib/monsters.svelte";
  import { deskWindow, logRequest, plannerRequest, tabRequest } from "$lib/nav.svelte";
  import { panelInfo } from "$lib/panels";
  import { placeLinks, watchPlaces } from "$lib/places.svelte";
  import { questView, watchQuests } from "$lib/quests.svelte";
  import { bag, watchBag } from "$lib/bag.svelte";
  import { live, watchLive } from "$lib/readings.svelte";
  import { watchSettings } from "$lib/settings.svelte";
  import { watchTyping } from "$lib/typing";
  import Segmented from "$lib/ui/Segmented.svelte";
  import LevelRing from "$lib/ui/LevelRing.svelte";
  import Logo from "$lib/components/Logo.svelte";
  import { followTrip } from "$lib/wayfinder.svelte";

  // Links to the training log open it in a column here, not in its own window.
  deskWindow.active = true;

  let prefs = $state(loadPrefs());
  const tab = $derived(prefs.tabs.find((t) => t.id === prefs.active) ?? prefs.tabs[0]);
  const columns = $derived(tab.columns);
  /** The tab being renamed, and what has been typed so far. */
  let renaming = $state<{ id: string; text: string } | null>(null);
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
    // Opened from a link to the training log or the build planner: show that tab.
    backend.deskTakeTab().then((name) => name && openTab(name));
    const stop = backend.onDeskTab(openTab);
    const close = (event: KeyboardEvent) => {
      if (event.key === "Escape") menu = null;
    };
    window.addEventListener("keydown", close);
    return () => {
      window.removeEventListener("keydown", close);
      void stop.then((unlisten) => unlisten());
    };
  });

  $effect(() => {
    void backend.setPageZoom(prefs.zoom);
  });

  function save() {
    savePrefs($state.snapshot(prefs));
  }

  function change(next: DeskColumn[]) {
    visit = null;
    menu = null;
    tab.columns = next;
    save();
  }

  function activate(id: string) {
    visit = null;
    menu = null;
    renaming = null;
    prefs.active = id;
    save();
  }

  /** Puts the shown tab back to its name and layout from the start. */
  function reset() {
    const fresh = defaultTab(tab.id);
    tab.name = fresh.name;
    change(fresh.columns);
  }

  function startRename(id: string) {
    const target = prefs.tabs.find((t) => t.id === id);
    if (!target) return;
    activate(id);
    renaming = { id, text: target.name };
  }

  function finishRename(keep: boolean) {
    if (!renaming) return;
    const target = prefs.tabs.find((t) => t.id === renaming!.id);
    const name = renaming.text.trim().slice(0, MAX_NAME);
    renaming = null;
    if (keep && target && name) {
      target.name = name;
      save();
    }
  }

  /** Focuses and selects a box as it appears. */
  function selectAll(node: HTMLInputElement) {
    node.focus();
    node.select();
  }

  function setZoom(zoom: number) {
    prefs.zoom = zoom;
    save();
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

  /** The training log or the build planner: its own tab, or any tab the player put it in. */
  function showTab(panel: "log" | "planner") {
    untrack(() => {
      if (find(columns, panel)) {
        visit = null;
        return;
      }
      const own = prefs.tabs.find((t) => t.id === panel && find(t.columns, panel));
      const any = own ?? prefs.tabs.find((t) => find(t.columns, panel));
      if (any) activate(any.id);
      else bringUp(panel);
    });
  }

  /** A tab the backend asked for when it opened or brought forward this window. */
  function openTab(name: string) {
    if (name === "log" || name === "planner") showTab(name);
  }

  $effect(() => {
    if (logRequest.seq) showTab("log");
  });

  $effect(() => {
    if (plannerRequest.seq) showTab("planner");
  });

  // Reading the character's windows by hotkey shows its result in the Character panel.
  $effect(() => {
    const read = live.characterRead;
    if (read && !read.problem && !read.automatic) bringUp("character");
  });

  $effect(() => {
    if (live.itemRead?.found) bringUp("item");
  });

  // A bag check shows in the Item panel too.
  watchBag();
  let bagSeen = bag.seq;
  $effect(() => {
    if (bag.seq !== bagSeen) {
      bagSeen = bag.seq;
      bringUp("item");
    }
  });

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
  const level = $derived(hud?.level ?? live.session?.level ?? null);
  const percent = $derived(hud?.exp?.percent ?? hud?.expPercent ?? live.session?.percent ?? null);
  const name = $derived(hud?.name ?? live.session?.name ?? null);
  const job = $derived(hud?.job ?? live.session?.job ?? null);
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
      <LevelRing {level} {percent} size={42}><Logo size={18} /></LevelRing>
      <div class="id">
        <span class="name">{name ?? "Leafline"}</span>
        <span class="sub">
          {#if level != null}<span class="num">Lv {level}{job ? ` ${job}` : ""}</span>{/if}
          {#if percent != null}<span class="num exp">{percent.toFixed(1)}%</span>{/if}
          <span class="status {liveText.tone}"><span class="dot" aria-hidden="true"></span>{liveText.text}</span>
        </span>
      </div>
    </div>
    <div class="controls">
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
        use:tip={onTop ? "Stop keeping on top" : "Keep on top"}
        onclick={toggleOnTop}
        >{#if onTop}<PinOff size={13} />{:else}<Pin size={13} />{/if}Keep on top</button
      >
    </div>
  </header>

  <nav class="tabs" aria-label="Desk tabs">
    <div class="tablist" role="tablist">
      {#each prefs.tabs as t (t.id)}
        {#if renaming?.id === t.id}
          <input
            class="rename"
            aria-label="Tab name"
            maxlength={MAX_NAME}
            bind:value={renaming.text}
            use:selectAll
            onblur={() => finishRename(true)}
            onkeydown={(e) => {
              if (e.key === "Enter") finishRename(true);
              else if (e.key === "Escape") finishRename(false);
            }}
          />
        {:else}
          <button
            role="tab"
            class="tab"
            aria-selected={t.id === tab.id}
            use:tip={t.id === tab.id ? "Double-click to rename" : undefined}
            onclick={() => activate(t.id)}
            ondblclick={() => startRename(t.id)}>{t.name}</button
          >
        {/if}
      {/each}
    </div>
    <span class="tab-actions">
      <button class="small" onclick={() => startRename(tab.id)}><Pencil size={12} />Rename</button>
      <button class="small" use:tip={"Put this tab back to how it started"} onclick={reset}
        ><RotateCcw size={12} />Reset tab</button
      >
    </span>
  </nav>

  <NextStep />

  <main style:grid-template-columns={template}>
    {#each columns as column, c (c)}
      <div class="column">
        {#each column as _, s (s)}
          {@const panel = shown(c, s)}
          {@const info = deskInfo(panel)}
          {@const Icon = info.icon}
          {@const visiting = visit?.column === c && visit?.slot === s}
          <section class="panel pane" aria-label={info.title}>
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
                  <PanelView {panel} analysis={live.analysis} session={live.session} roomy />
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

  /* The window's own header colours, as the overlay's: dark in every look but Sprout Dark's plum. */
  header {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2) var(--space-4);
    margin-bottom: 12px;
    padding: 10px var(--space-4) 10px 12px;
    border-radius: var(--header-radius);
    background: var(--header-bg);
    color: var(--header-text);
  }

  .who,
  .controls {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2) var(--space-3);
    min-width: 0;
  }

  .id {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
  }

  .name {
    font-family: var(--font-display);
    font-size: var(--text-lg);
    font-weight: var(--weight-strong);
    line-height: 1.2;
  }

  .sub {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 2px 10px;
    color: var(--header-muted);
    font-size: var(--text-sm);
  }

  .num {
    font-family: var(--font-display);
    font-variant-numeric: tabular-nums;
  }

  .exp {
    color: var(--header-exp);
    font-weight: 600;
  }

  .faint {
    color: var(--header-faint);
    font-size: var(--text-sm);
  }

  .status {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--header-muted);
    --dot: var(--header-faint);
  }

  .status .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--dot);
  }

  .status.good {
    --dot: var(--header-good);
  }

  .status.warn {
    --dot: var(--header-accent);
  }

  .status.bad {
    --dot: var(--hp);
  }

  .field {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
  }

  /* Flat, as the shared Button is. */
  button.ontop,
  button.small {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: var(--control-h);
    padding: 0 12px;
    border: var(--line-w) solid var(--control-line);
    border-radius: var(--radius-pill);
    background: var(--surface);
    color: var(--text);
    font-size: var(--text-sm);
    white-space: nowrap;
    cursor: pointer;
  }

  button.small {
    height: var(--control-h-sm);
    padding: 0 10px;
    font-size: var(--text-xs);
  }

  button.ontop:hover,
  button.small:hover {
    background: var(--surface-2);
  }

  header button.ontop {
    border-color: var(--header-track);
    background: transparent;
    color: var(--header-text);
  }

  header button.ontop:hover {
    background: var(--header-hover);
  }

  button.ontop[aria-pressed="true"] {
    border-color: var(--accent);
    background: var(--accent);
    color: var(--accent-ink);
  }

  header button.ontop[aria-pressed="true"]:hover {
    background: var(--accent-strong);
  }

  button:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  .tabs {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2) var(--space-3);
    padding: 0 14px 12px;
  }

  /* Layouts as chips, the open one filled. */
  .tablist {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    min-width: 0;
  }

  .tab,
  .rename {
    height: calc(var(--control-h) - 2px);
    padding: 0 14px;
    border: var(--line-w) solid var(--control-line);
    border-radius: var(--radius-pill);
    font-size: var(--text-sm);
    white-space: nowrap;
  }

  .tab {
    background: var(--surface);
    color: var(--muted);
    font-weight: 500;
    cursor: pointer;
    transition:
      background 0.12s ease,
      color 0.12s ease;
  }

  .tab:hover {
    background: var(--surface-2);
    color: var(--text);
  }

  .tab[aria-selected="true"] {
    background: var(--accent);
    color: var(--accent-ink);
    font-weight: var(--weight-strong);
  }

  :global(:root:not([data-theme^="sprout"])) .tab[aria-selected="true"] {
    border-color: transparent;
  }

  .rename {
    height: 32px;
    padding: 0 var(--space-4);
    border: 1px solid transparent;
    border-radius: 7px;
    font-size: var(--text-md);
    white-space: nowrap;
  }

  .tab {
    background: transparent;
    color: var(--muted);
    cursor: pointer;
    transition:
      background 0.16s ease,
      color 0.12s ease;
  }

  .tab:hover {
    background: var(--hover);
    color: var(--text);
  }

  .tab[aria-selected="true"] {
    background: linear-gradient(180deg, var(--surface-3), var(--surface-2));
    box-shadow:
      inset 0 1px 0 var(--highlight),
      0 1px 4px rgba(0, 0, 0, 0.4);
    color: var(--accent-text-strong);
    font-weight: 600;
  }

  .rename {
    width: 150px;
    background: var(--surface);
    border-color: var(--accent);
    color: var(--text);
    outline: none;
  }

  .tab-actions {
    display: inline-flex;
    gap: var(--space-2);
  }

  /* The Go here bar sits under the tabs, as wide as the columns. */
  .desk > :global(.bar) {
    margin: 0 14px 12px;
  }

  main {
    flex: 1;
    min-height: 0;
    display: grid;
    gap: 12px;
    padding: 0 14px 14px;
  }

  .column {
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-width: 0;
    min-height: 0;
  }

  .panel {
    flex: 1 1 0;
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
  }

  /* A column: the look's outline and edge, a step darker than the cards inside it. */
  .pane {
    border: var(--line-w) solid var(--line);
    border-radius: var(--radius-lg);
    background: color-mix(in srgb, var(--surface) 45%, var(--bg));
    box-shadow: var(--card-shadow);
  }

  .panel-head {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    height: 44px;
    flex: none;
    padding: 0 var(--space-3) 0 var(--space-4);
  }

  h2 {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    margin: 0;
    font-family: var(--font-display);
    font-size: 15px;
    font-weight: var(--weight-strong);
  }

  h2 :global(svg) {
    color: var(--accent-text);
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
    padding: 6px;
    background: var(--surface-raised);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: var(--shadow-lg);
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
    padding: 0 10px;
    border: 0;
    border-radius: var(--radius-sm);
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
    background: var(--hover);
  }

  .item.current {
    background: var(--accent-soft);
    color: var(--accent-text-strong);
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
    padding: 0 0 var(--space-4);
  }
</style>
