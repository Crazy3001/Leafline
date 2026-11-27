<script lang="ts">
  // The desk view: Leafline laid out for a wide window, often on a second screen but just as well
  // beside a windowed game. A sidebar picks one view at a time: the overlay's five tabs, each
  // laid out for the width, Look up, the build planner, a tool, or the player's own columns.
  // It reads what the overlay reads and follows the overlay's "Go here" trip. A link opens its
  // view here, with a way back to where you were.
  import { tip } from "$lib/tip.svelte";
  import { ArrowLeft, PictureInPicture2, Pin, PinOff } from "@lucide/svelte";
  import { onMount, tick, untrack } from "svelte";

  import { backend } from "$lib/api";
  import Compass from "$lib/components/desk/Compass.svelte";
  import DeskColumns from "$lib/components/desk/DeskColumns.svelte";
  import DeskBar from "$lib/components/desk/DeskBar.svelte";
  import DeskSidebar from "$lib/components/desk/DeskSidebar.svelte";
  import DeskTools from "$lib/components/desk/DeskTools.svelte";
  import DeskMap from "$lib/components/desk/DeskMap.svelte";
  import DeskSearch, { type Kind as FoundKind } from "$lib/components/desk/DeskSearch.svelte";
  import ShareBanner from "$lib/components/desk/ShareBanner.svelte";
  import DeskQuests from "$lib/components/desk/DeskQuests.svelte";
  import NextUp from "$lib/components/desk/NextUp.svelte";
  import Segmented from "$lib/ui/Segmented.svelte";
  import DropLogView from "$lib/components/DropLogView.svelte";
  import BagCheck from "$lib/components/item/BagCheck.svelte";
  import CraftingPanel from "$lib/components/item/CraftingPanel.svelte";
  import TrackedItems from "$lib/components/item/TrackedItems.svelte";
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
  import {
    DESK_TOOL_PAGES,
    loadFolded,
    loadPrefs,
    loadTrain,
    loadView,
    saveFolded,
    savePrefs,
    saveTrain,
    saveView,
    viewOfPanel,
    type DeskTrain,
    type DeskView,
  } from "$lib/desk";
  import { characters } from "$lib/characters.svelte";
  import { blankQuery, gearView, itemView, lookup, watchItems } from "$lib/items.svelte";
  import { monsterView, watchMonsters } from "$lib/monsters.svelte";
  import { mapView, npcView } from "$lib/places.svelte";
  import { deskWindow, logRequest, plannerRequest, tabRequest } from "$lib/nav.svelte";
  import { placeLinks, watchPlaces } from "$lib/places.svelte";
  import { questView, watchQuests } from "$lib/quests.svelte";
  import { bag, watchBag } from "$lib/bag.svelte";
  import { live, watchLive } from "$lib/readings.svelte";
  import { watchSettings } from "$lib/settings.svelte";
  import { watchTyping } from "$lib/typing";
  import { followTrip, startWayfinder } from "$lib/wayfinder.svelte";
  import type { PinStatus } from "$lib/types";
  import { can, isWeb } from "$lib/platform";

  // Links to the training log and the planner open here, not in windows of their own.
  deskWindow.active = true;

  let prefs = $state(loadPrefs());
  let view = $state<DeskView>(loadView());
  /** Which page of Train shows. */
  let trainPage = $state<DeskTrain>(loadTrain());
  const TRAIN_PAGES: { id: DeskTrain; label: string }[] = [
    { id: "session", label: "This session" },
    { id: "where", label: "Where to train" },
    { id: "log", label: "Training log" },
  ];
  function pickTrain(page: DeskTrain) {
    trainPage = page;
    saveTrain(page);
  }
  /** The training log, asked for by a link or a window. */
  function openLog() {
    pickTrain("log");
    go("train");
  }
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
    tools: "Tools",
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
    // In the program the overlay window follows the trip and this one shows it; the website has
    // no overlay beside the desk, so the desk follows it itself.
    if (isWeb) startWayfinder();
    else followTrip();
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

  let deskSearch = $state<DeskSearch>();
  /** Words the top row's search hands to Look up when nothing in its list was picked. */
  let seedWords = $state("");

  /** A match picked in the top row's search: its page in Look up, or a quest on Quests. */
  function openFound(kind: FoundKind, id: number) {
    if (kind === "quests") {
      questView.id = id;
      go("quests");
      return;
    }
    lookup.mode = kind;
    if (kind === "items") itemView.id = id;
    else if (kind === "monsters") {
      monsterView.trail = [];
      monsterView.id = id;
    } else if (kind === "maps") {
      mapView.trail = [];
      mapView.id = id;
    } else npcView.id = id;
    land();
  }

  /** On Quests the top row's search filters the page's list: one search box, not two. */
  const questFilter = $derived({
    text: questView.search.text,
    placeholder: "Search quests, people, monsters…",
    label: "Search quests",
    onchange: (text: string) => {
      questView.search = { ...questView.search, text, offset: 0 };
      questView.id = null;
    },
  });

  async function searchWords(words: string) {
    seedWords = words;
    await openSearch();
  }

  /** A tool picked in Look up's results. */
  function openTool(tool: ToolInfo) {
    const action = tool.action;
    if ("page" in action) {
      if ((DESK_TOOL_PAGES as string[]).includes(action.page)) go(action.page);
    } else if ("tab" in action) go(action.tab);
    else if ("window" in action) {
      if (action.window === "build") go("planner");
      else openLog();
    }
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
    if (name === "log") openLog();
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
    if (logRequest.seq) untrack(openLog);
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

  /** The five tabs by number, in the sidebar's order. */
  const NUMBERED: DeskView[] = ["now", "train", "quests", "map", "me"];

  /**
   * "/" puts the cursor in the search box, as on many sites, and 1 to 5 open the five tabs, unless
   * you're typing somewhere. Keys in this window only; the game's never see them.
   */
  function keys(event: KeyboardEvent) {
    const typing = (event.target as Element | null)?.closest?.("input, textarea, select, [contenteditable]");
    if (typing || event.ctrlKey || event.altKey || event.metaKey) return;
    if (event.key === "/") {
      event.preventDefault();
      if (view === "lookup" || !deskSearch) void openSearch();
      else deskSearch.focus();
    } else if (/^[1-5]$/.test(event.key)) {
      event.preventDefault();
      pick(NUMBERED[Number(event.key) - 1]);
    }
  }

  const level = $derived(live.analysis?.hud?.level ?? live.session?.level ?? null);
  /** Reading the game times the session, so there's nothing to type in. */
  const typedHidden = $derived(
    can.liveReading ? live.liveStatus.state !== "off" : live.liveStatus.state === "reading" || live.liveStatus.state === "waiting",
  );
  /** Views that are one page wide and read better not stretched across a big screen. */
  const narrowViews: DeskView[] = ["lookup", "tools", "crafting", "drops", "timers", "sell", "tracked", "potions", "route", "share"];
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
    <!-- Who you are, the level and the session's numbers, over every view, Now too. -->
    <DeskBar />
    <!-- The website, while it isn't reading the game: how to start, or what's in the way. -->
    <ShareBanner />
    <div class="top">
      {#if from}
        <button class="pill" onclick={back}><ArrowLeft size={14} />Back to {viewName(from)}</button>
      {/if}
      {#if view !== "lookup"}
        <DeskSearch bind:this={deskSearch} onopen={openFound} onwords={searchWords} filter={view === "quests" ? questFilter : null} />
      {/if}
      <!-- Where you're heading and the next step from here, in the middle of the row. -->
      <div class="compass-slot"><Compass onpick={() => go("map")} /></div>
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
          <!-- The bar above says who you are and the session's numbers. The map you're on comes first,
               where the eye goes while playing, with the session's chart beside it; then what to do
               next and your quests. Typing a session in is on Train. -->
          <div class="now-top">
            <DeskMap ongo={() => go("map")} />
            <TrainPanel {level} map={live.analysis?.map ?? null} session={live.session} only="chart" />
          </div>
          <div class="now-r1">
            <NextUp onme={() => go("me")} />
            <DeskQuests ongo={() => go("quests")} />
          </div>
        {:else if view === "quests"}
          <div class="stack"><PanelView panel="quests" analysis={live.analysis} session={live.session} roomy topSearch /></div>
        {:else if view === "map"}
          <div class="flow two map-flow"><PanelView panel="map" analysis={live.analysis} session={live.session} roomy /></div>
        {:else if view === "me"}
          <div class="stack"><PanelView panel="character" analysis={live.analysis} session={live.session} roomy shelled wide /></div>
        {:else if view === "train"}
          <div class="train-pages"><Segmented label="Train shows" options={TRAIN_PAGES} value={trainPage} onchange={pickTrain} /></div>
          {#if trainPage === "log"}
            <TrainingLog embedded />
          {:else if trainPage === "where"}
            <div class="stack"><TrainPanel {level} map={live.analysis?.map ?? null} session={live.session} only="where" /></div>
          {:else}
            <!-- Without live reading, a session can be typed in. -->
            {#if !typedHidden}<div class="typed"><TypedSessionCard /></div>{/if}
            <div class="flow two"><TrainPanel {level} map={live.analysis?.map ?? null} session={live.session} only="session" glanced /></div>
          {/if}
        {:else if view === "tools"}
          <DeskTools ontool={openTool} />
        {:else if view === "lookup"}
          <div class="lookup-view">
            {#key searchFocus}<SearchSheet focus={searchFocus > 0} {landed} seed={seedWords} ontool={openTool} />{/key}
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
    gap: var(--space-2);
    padding: var(--space-3) var(--space-4);
  }

  /* The compass takes the middle of what's left. */
  .compass-slot {
    display: flex;
    flex: 1;
    justify-content: center;
    min-width: 0;
  }

  /* A narrow window: the compass gets a line of its own under the row. */
  @container desk-main (max-width: 640px) {
    .top {
      flex-wrap: wrap;
    }

    .compass-slot {
      flex-basis: 100%;
      order: 10;
    }
  }

  .pill {
    display: inline-flex;
    flex: none;
    align-items: center;
    gap: var(--space-1-5);
    height: var(--control-h);
    padding: 0 var(--space-3);
    border: var(--line-w) solid var(--control-line);
    border-radius: var(--radius-pill);
    background: var(--surface);
    color: var(--text);
    font-size: var(--text-md);
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

  .pill:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  .view {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding: 0 var(--space-4) var(--space-4);
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
    column-gap: var(--space-3);
  }

  .flow.two {
    columns: 2 460px;
  }

  .flow :global(.block),
  .flow :global(.card) {
    break-inside: avoid;
  }

  .flow :global(.block) {
    margin: 0 0 var(--space-3);
  }

  /* One card under another, as wide as the window: Quests, whose cards sit side by side inside. */
  .stack :global(.card) {
    margin-left: 0;
    margin-right: 0;
  }

  /* Now: this map and the session's chart side by side, then Next up and Quests. Narrow windows stack them. */
  .now-top {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: var(--space-4);
    align-items: start;
  }

  .now-r1 {
    display: grid;
    grid-template-columns: minmax(0, 1.7fr) minmax(0, 1fr);
    gap: var(--space-4);
    align-items: start;
  }

  /* Panels' own side margins are for the overlay; the rows space them here. */
  .now-top :global(.card),
  .now-r1 :global(.card) {
    margin: 0;
  }

  @container desk-main (max-width: 1000px) {
    .now-top,
    .now-r1 {
      grid-template-columns: minmax(0, 1fr);
    }
  }

  .typed {
    max-width: 640px;
  }

  .train-pages {
    display: flex;
  }
</style>
