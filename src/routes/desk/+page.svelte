<script lang="ts">
  // The desk view: Leafline laid out for a wide window, often on a second screen but just as well
  // beside a windowed game. A sidebar picks one view at a time: the overlay's five tabs, each
  // laid out for the width, Look up, the build planner, a tool, or the player's own columns.
  // It reads what the overlay reads and follows the overlay's "Go here" trip. A link opens its
  // view here, with a way back to where you were.
  import { tip } from "$lib/tip.svelte";
  import { ArrowLeft, PictureInPicture2, Pin, PinOff, Search } from "@lucide/svelte";
  import { onMount, tick, untrack } from "svelte";

  import { backend } from "$lib/api";
  import DeskColumns from "$lib/components/desk/DeskColumns.svelte";
  import DeskGlance from "$lib/components/desk/DeskGlance.svelte";
  import DeskSidebar from "$lib/components/desk/DeskSidebar.svelte";
  import CharacterPicker from "$lib/components/CharacterPicker.svelte";
  import DropLogView from "$lib/components/DropLogView.svelte";
  import BagCheck from "$lib/components/item/BagCheck.svelte";
  import CraftingPanel from "$lib/components/item/CraftingPanel.svelte";
  import TrackedItems from "$lib/components/item/TrackedItems.svelte";
  import NextStep from "$lib/components/NextStep.svelte";
  import PanelView from "$lib/components/PanelView.svelte";
  import TimersCard from "$lib/components/TimersCard.svelte";
  import TrainingLog from "$lib/components/TrainingLog.svelte";
  import TrainPanel from "$lib/components/TrainPanel.svelte";
  import TypedSessionCard from "$lib/components/TypedSessionCard.svelte";
  import BuildPlanner from "$lib/components/planner/BuildPlanner.svelte";
  import PotionPage from "$lib/shell/PotionPage.svelte";
  import RoutePage from "$lib/shell/RoutePage.svelte";
  import SearchSheet from "$lib/shell/SearchSheet.svelte";
  import SharePage from "$lib/shell/SharePage.svelte";
  import { toolPageTitle, type ToolInfo } from "$lib/shell/tabs";
  import { DESK_TOOL_PAGES, loadFolded, loadPrefs, loadView, saveFolded, savePrefs, saveView, viewOfPanel, type DeskView } from "$lib/desk";
  import { characters } from "$lib/characters.svelte";
  import { blankQuery, gearView, itemView, lookup, watchItems } from "$lib/items.svelte";
  import { monsterView, watchMonsters } from "$lib/monsters.svelte";
  import { deskWindow, logRequest, plannerRequest, tabRequest } from "$lib/nav.svelte";
  import { placeLinks, watchPlaces } from "$lib/places.svelte";
  import { questView, watchQuests } from "$lib/quests.svelte";
  import { bag, watchBag } from "$lib/bag.svelte";
  import { live, watchLive } from "$lib/readings.svelte";
  import { watchSettings } from "$lib/settings.svelte";
  import { watchTyping } from "$lib/typing";
  import { followTrip } from "$lib/wayfinder.svelte";
  import type { PinStatus } from "$lib/types";
  import { can, isWeb } from "$lib/platform";
  import ReadWindowCard from "$lib/components/ReadWindowCard.svelte";

  // Links to the training log and the planner open here, not in windows of their own.
  deskWindow.active = true;

  let prefs = $state(loadPrefs());
  let view = $state<DeskView>(loadView());
  let folded = $state(loadFolded());
  /** Where a link took you from, for the Back button. */
  let from = $state<DeskView | null>(null);
  let onTop = $state(false);
  let pins = $state<PinStatus>({ pinned: [], hidden: false });
  /** Counts up when something outside opens a page in Look up, so it shows the page. */
  let landed = $state(0);
  /** Counts up to put the cursor in Look up's box. */
  let searchFocus = $state(0);

  const NAMES: Record<string, string> = {
    now: "Now",
    quests: "Quests",
    map: "Map",
    train: "Train",
    me: "Me",
    lookup: "Look up",
    planner: "Build planner",
    layout: "My layout",
  };
  const viewName = (v: DeskView) => NAMES[v] ?? toolPageTitle(v as never);

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
    backend.pinStatus().then((s) => (pins = s));
    const unpins = backend.onPins((s) => (pins = s));
    // Opened from a link to the training log or the build planner: show that view.
    backend.deskTakeTab().then((name) => name && openTab(name));
    const stop = backend.onDeskTab(openTab);
    return () => {
      void stop.then((unlisten) => unlisten());
      void unpins.then((unlisten) => unlisten());
    };
  });

  $effect(() => {
    void backend.setPageZoom(prefs.zoom);
  });

  function savePrefsNow() {
    savePrefs($state.snapshot(prefs));
  }

  /** Shows a view picked in the sidebar. */
  function pick(next: DeskView) {
    from = null;
    view = next;
    saveView(next);
  }

  /** Shows a view a link asked for, remembering where you were. */
  function go(next: DeskView) {
    untrack(() => {
      if (next === view) return;
      from = view;
      view = next;
      saveView(next);
    });
  }

  function back() {
    if (!from) return;
    const to = from;
    from = null;
    view = to;
    saveView(to);
  }

  async function toggleOnTop() {
    onTop = (await backend.setDeskOnTop(!onTop)).onTop;
  }

  const poppedOut = $derived(pins.pinned.includes("now"));

  async function togglePopOut() {
    pins = poppedOut ? await backend.unpinPanel("now") : await backend.pinPanel("now");
  }

  async function openSearch() {
    if (view !== "lookup") go("lookup");
    await tick();
    searchFocus += 1;
    document.querySelector<HTMLInputElement>(".lookup-view input")?.focus();
  }

  /** A tool picked in Look up's results. */
  function openTool(tool: ToolInfo) {
    const action = tool.action;
    if ("page" in action) {
      if ((DESK_TOOL_PAGES as string[]).includes(action.page)) go(action.page);
    } else if ("tab" in action) go(action.tab);
    else if ("window" in action) go(action.window === "build" ? "planner" : "train");
    else if (action.search === "gear") {
      itemView.id = null;
      itemView.query = { ...blankQuery(), kind: "equip" };
      gearView.forMe = true;
      lookup.mode = "items";
      landed += 1;
    }
  }

  /** A view the backend asked for when it opened or brought forward this window. */
  function openTab(name: string) {
    if (name === "log") go("train");
    else if (name === "planner") go("planner");
  }

  /** A page opened in Look up from a link, an item check or a bag check. */
  function land() {
    landed += 1;
    go("lookup");
  }

  // A link to an item, monster, map, NPC or quest, from this window or another, shows here.
  $effect(() => {
    if (itemView.seen || monsterView.seen || placeLinks.seen) untrack(land);
  });

  $effect(() => {
    if (questView.seen) go("quests");
  });

  // A panel's link to another one, such as the Character panel's line to Train.
  $effect(() => {
    if (tabRequest.seq && tabRequest.panel) go(viewOfPanel(tabRequest.panel));
  });

  $effect(() => {
    if (logRequest.seq) go("train");
  });

  $effect(() => {
    if (plannerRequest.seq) go("planner");
  });

  // Reading the character's windows by hotkey shows its result under Me.
  $effect(() => {
    const read = live.characterRead;
    if (read && !read.problem && !read.automatic) go("me");
  });

  $effect(() => {
    if (live.itemRead?.found) untrack(land);
  });

  // A bag check shows in Look up too.
  watchBag();
  let bagSeen = bag.seq;
  $effect(() => {
    if (bag.seq !== bagSeen) {
      bagSeen = bag.seq;
      untrack(land);
    }
  });

  /** "/" puts the cursor in the search box, as on many sites, unless you're typing somewhere. */
  function keys(event: KeyboardEvent) {
    const typing = (event.target as Element | null)?.closest?.("input, textarea, select, [contenteditable]");
    if (event.key === "/" && !typing && !event.ctrlKey && !event.altKey && !event.metaKey) {
      event.preventDefault();
      void openSearch();
    }
  }

  const level = $derived(live.analysis?.hud?.level ?? live.session?.level ?? null);
  /** Views that are one page wide and read better not stretched across a big screen. */
  const narrowViews: DeskView[] = ["lookup", "crafting", "drops", "timers", "sell", "tracked", "potions", "route", "share"];
</script>

<svelte:head><title>{isWeb ? "Leafline" : "Leafline — Desk view"}</title></svelte:head>
<svelte:window onkeydown={keys} />

<div class="desk">
  <DeskSidebar
    {view}
    onview={pick}
    {folded}
    onfold={() => {
      folded = !folded;
      saveFolded(folded);
    }}
    zoom={prefs.zoom}
    onzoom={(z) => {
      prefs.zoom = z;
      savePrefsNow();
    }}
  />

  <div class="main">
    <div class="top">
      {#if from}
        <button class="pill" onclick={back}><ArrowLeft size={14} />Back to {viewName(from)}</button>
      {/if}
      {#if view !== "lookup"}
        <button class="search" onclick={openSearch}>
          <Search size={15} /><span>Search items, monsters, maps, NPCs, quests…</span><kbd>/</kbd>
        </button>
      {/if}
      <div class="step"><NextStep /></div>
      {#if can.popOut}<button
        class="pill"
        aria-pressed={poppedOut}
        use:tip={poppedOut ? "Close the small Now window" : "A small window of Now's numbers, kept over the game"}
        onclick={togglePopOut}><PictureInPicture2 size={14} />{poppedOut ? "Popped out" : "Pop out"}</button
      >{/if}
      {#if can.onTop}<button
        class="pill icon"
        aria-pressed={onTop}
        aria-label={onTop ? "Stop keeping this window on top" : "Keep this window on top"}
        use:tip={onTop ? "Stop keeping on top" : "Keep this window on top"}
        onclick={toggleOnTop}>{#if onTop}<PinOff size={14} />{:else}<Pin size={14} />{/if}</button
      >{/if}
    </div>

    <div class="view" class:narrow={narrowViews.includes(view)} class:full={view === "planner" || view === "layout"} aria-label={viewName(view)}>
      {#key view}
        {#if view === "now"}
          <DeskGlance />
          <CharacterPicker />
          <div class="flow">
            {#if isWeb}<div class="block"><ReadWindowCard /></div>{/if}
            {#if can.liveReading ? live.liveStatus.state === "off" : live.liveStatus.state !== "reading" && live.liveStatus.state !== "waiting"}
              <div class="block"><TypedSessionCard /></div>
            {/if}
            {#if (live.analysis?.quest?.quests ?? []).some((q) => q.objectives.length)}
              <section class="block card quests-glance">
                <PanelView panel="quests" analysis={live.analysis} session={live.session} pinned />
              </section>
            {/if}
            <TrainPanel {level} map={live.analysis?.map ?? null} session={live.session} whole glanced />
          </div>
        {:else if view === "quests"}
          <div class="stack"><PanelView panel="quests" analysis={live.analysis} session={live.session} roomy /></div>
        {:else if view === "map"}
          <div class="flow two map-flow"><PanelView panel="map" analysis={live.analysis} session={live.session} roomy /></div>
        {:else if view === "me"}
          <div class="stack"><PanelView panel="character" analysis={live.analysis} session={live.session} roomy shelled wide /></div>
        {:else if view === "train"}
          <TrainingLog embedded />
        {:else if view === "lookup"}
          <div class="lookup-view">
            {#key searchFocus}<SearchSheet focus={searchFocus > 0} {landed} ontool={openTool} />{/key}
          </div>
        {:else if view === "planner"}
          <div class="pane full-pane"><BuildPlanner embedded /></div>
        {:else if view === "layout"}
          <DeskColumns bind:prefs onsave={savePrefsNow} />
        {:else if view === "crafting"}
          <CraftingPanel />
        {:else if view === "drops"}
          <DropLogView character={characters.viewing ?? ""} />
        {:else if view === "timers"}
          <TimersCard explain />
        {:else if view === "sell"}
          <BagCheck />
        {:else if view === "tracked"}
          <TrackedItems map={live.analysis?.map ?? null} explain />
        {:else if view === "potions"}
          <PotionPage />
        {:else if view === "route"}
          <RoutePage />
        {:else if view === "share"}
          <SharePage />
        {/if}
      {/key}
    </div>
  </div>
</div>

<style>
  :global(html),
  :global(body) {
    background: var(--bg);
    overflow: hidden;
  }

  .desk {
    display: flex;
    height: calc(100vh / var(--page-zoom, 1));
    background: var(--bg);
  }

  .main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    container: desk-main / inline-size;
  }

  .top {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 16px;
  }

  .search {
    display: flex;
    align-items: center;
    gap: 10px;
    flex: 0 1 340px;
    min-width: 180px;
    height: var(--control-h);
    padding: 0 12px;
    border: var(--line-w) solid var(--control-line);
    border-radius: var(--radius);
    background: var(--surface);
    color: var(--faint);
    font: inherit;
    font-size: var(--text-md);
    text-align: left;
    cursor: text;
  }

  .search span {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .search:hover {
    border-color: var(--border-strong);
  }

  kbd {
    padding: 1px 6px;
    border-radius: 5px;
    background: var(--surface-3);
    color: var(--muted);
    font-family: var(--font-display);
    font-size: var(--text-xs);
  }

  /* The Go here bar fills what's left; it hides itself with no trip. */
  .step {
    flex: 1;
    min-width: 0;
  }

  .step :global(.bar) {
    margin: 0;
  }

  .pill {
    display: inline-flex;
    flex: none;
    align-items: center;
    gap: 6px;
    height: var(--control-h);
    padding: 0 13px;
    border: var(--line-w) solid var(--control-line);
    border-radius: var(--radius-pill);
    background: var(--surface);
    color: var(--text);
    font-size: var(--text-sm);
    white-space: nowrap;
    cursor: pointer;
  }

  .pill.icon {
    width: var(--control-h);
    justify-content: center;
    padding: 0;
  }

  .pill:hover {
    background: var(--surface-2);
  }

  .pill[aria-pressed="true"] {
    border-color: var(--accent);
    background: var(--accent);
    color: var(--accent-ink);
  }

  .search:focus-visible,
  .pill:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  .view {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 0 16px 16px;
    overflow-y: auto;
  }

  .view.narrow > :global(*) {
    width: 100%;
    max-width: 1080px;
  }

  .view.full {
    overflow: hidden;
  }

  .pane {
    border: var(--line-w) solid var(--line);
    border-radius: var(--radius-lg);
    background: color-mix(in srgb, var(--surface) 45%, var(--bg));
    box-shadow: var(--card-shadow);
  }

  .full-pane {
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  /*
   * A view's cards flow into columns as wide as the window allows, top to bottom then across,
   * so a wide screen holds them side by side and a narrow one stacks them.
   */
  .flow {
    columns: 3 400px;
    column-gap: 12px;
  }

  .flow.two {
    columns: 2 460px;
  }

  .flow :global(.block),
  .flow :global(.card) {
    break-inside: avoid;
  }

  .flow :global(.block) {
    margin: 0 0 12px;
  }

  /* The character switch for Now's cards, under the strip. */
  .view > :global(.picker) {
    margin: 0;
    padding: 0 4px;
  }

  /* One card under another, as wide as the window: Quests, whose cards sit side by side inside. */
  .stack :global(.card) {
    margin-left: 0;
    margin-right: 0;
  }

  /* Quests in the Now view: the pinned window's short form, framed as a card. */
  .quests-glance {
    display: block;
    padding: 12px 14px;
  }

  .quests-glance :global(.card) {
    margin: 0;
    border: 0;
    box-shadow: none;
    background: none;
    padding: 0;
  }
</style>
