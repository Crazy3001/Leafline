<script lang="ts">
  // The main window. A header that says who you are and how far through the level; the next step
  // of a route when one is set; one of five tabs; and the tab bar. Search, an item check, the Tools
  // list and each tool rise as a sheet over the tab, so the tab is still there when they close.
  import { X } from "@lucide/svelte";
  import { onMount } from "svelte";

  import { backend, fileName } from "$lib/api";
  import { bag, watchBag } from "$lib/bag.svelte";
  import { characters } from "$lib/characters.svelte";
  import CharacterPicker from "$lib/components/CharacterPicker.svelte";
  import CompactHud from "$lib/components/CompactHud.svelte";
  import DropLogView from "$lib/components/DropLogView.svelte";
  import BagCheck from "$lib/components/item/BagCheck.svelte";
  import CraftingPanel from "$lib/components/item/CraftingPanel.svelte";
  import TrackedItems from "$lib/components/item/TrackedItems.svelte";
  import NextStep from "$lib/components/NextStep.svelte";
  import PanelView from "$lib/components/PanelView.svelte";
  import TimersCard from "$lib/components/TimersCard.svelte";
  import { reportUrl } from "$lib/feedback";
  import { blankQuery, gearView, itemView, lookup, watchItems } from "$lib/items.svelte";
  import { monsterView, watchMonsters } from "$lib/monsters.svelte";
  import { tabRequest } from "$lib/nav.svelte";
  import { signature } from "$lib/panels";
  import { placeLinks, watchPlaces } from "$lib/places.svelte";
  import { questView, watchQuests } from "$lib/quests.svelte";
  import { live, watchLive } from "$lib/readings.svelte";
  import { display, hotkey, keyHint, watchSettings } from "$lib/settings.svelte";
  import BottomNav from "$lib/shell/BottomNav.svelte";
  import NowScreen from "$lib/shell/NowScreen.svelte";
  import SearchSheet from "$lib/shell/SearchSheet.svelte";
  import ShellHeader from "$lib/shell/ShellHeader.svelte";
  import { TABS, tabId, tabInfo, tabOfPanel, toolPageTitle, type Tab, type ToolInfo, type ToolPage } from "$lib/shell/tabs";
  import ToolsList from "$lib/shell/ToolsList.svelte";
  import PotionPage from "$lib/shell/PotionPage.svelte";
  import RoutePage from "$lib/shell/RoutePage.svelte";
  import SharePage from "$lib/shell/SharePage.svelte";
  import Welcome from "$lib/shell/Welcome.svelte";
  import { tip } from "$lib/tip.svelte";
  import { watchTyping } from "$lib/typing";
  import type { Panel } from "$lib/types";
  import Sheet from "$lib/ui/Sheet.svelte";
  import { startWayfinder } from "$lib/wayfinder.svelte";

  const TAB_KEY = "leafline.overlay.tab.v2";

  /** What is open over the tab, if anything. */
  type Over = { kind: "search"; focus: boolean } | { kind: "tools" } | { kind: "tool"; page: ToolPage };

  let tab = $state<Tab>(storedTab());
  let over = $state<Over | null>(null);
  /** Counts the pages something other than the player opened in search: an item check, a link. */
  let landed = $state(0);
  /** What each tab's panel showed when last looked at, to mark tabs with something new. */
  let seen = $state<Partial<Record<Tab, string | null>>>({});

  // Clicks leave the game focused; typing in a box takes focus while it lasts.
  onMount(() => watchTyping());

  onMount(() => {
    watchLive();
    watchSettings();
    watchItems();
    watchMonsters();
    watchPlaces();
    watchQuests();
    startWayfinder();
    // A pinned panel's way to the whole of its tab.
    const stop = backend.onShowPanel((panel) => showPanel(panel));
    return () => void stop.then((unlisten) => unlisten());
  });

  // Now's numbers change every second, so it never gets a dot; the others follow their panel.
  const signatures = $derived(
    Object.fromEntries(
      TABS.map((t) => [t.id, signature(t.panel, live.analysis, live.session)]),
    ) as Record<Tab, string | null>,
  );

  // The open tab is always seen; others get a dot when what they show changes after its first
  // reading, so a fresh start doesn't light every tab at once.
  $effect(() => {
    seen[tab] = signatures[tab];
    for (const t of TABS) if (seen[t.id] == null && signatures[t.id] != null) seen[t.id] = signatures[t.id];
  });

  // An item check is asked for by hotkey, so its result opens by itself, over whatever tab is open.
  $effect(() => {
    if (live.itemRead?.found) search("items");
  });

  // So is a bag check, shown with the items.
  watchBag();
  let bagSeen = bag.seq;
  $effect(() => {
    if (bag.seq !== bagSeen) {
      bagSeen = bag.seq;
      itemView.id = null;
      search("items");
    }
  });

  // So is reading the character's windows, whose result is in the Me tab. When the live pictures
  // catch a change by themselves, a line says so and the tab stays where it is.
  $effect(() => {
    const read = live.characterRead;
    if (read && !read.problem && !read.automatic) select("me");
  });

  // So is a link to an item, monster, map or NPC from anywhere: the build planner, a list of drops.
  $effect(() => {
    if (itemView.seen || monsterView.seen || placeLinks.seen) search();
  });

  $effect(() => {
    if (questView.seen) select("quests");
  });

  // A panel's link to another, such as the character card's line to Train.
  $effect(() => {
    if (tabRequest.seq && tabRequest.panel) showPanel(tabRequest.panel);
  });

  const fresh = (t: Tab) => t !== tab && signatures[t] != null && seen[t] != null && signatures[t] !== seen[t];
  const isPinned = (t: Tab) => live.pins.pinned.includes(tabInfo(t).panel);
  const pinned = $derived(isPinned(tab));
  const pinTitle = $derived(
    pinned ? `Unpin ${tabInfo(tab).pinName}` : `Pin ${tabInfo(tab).pinName} in a window of its own`,
  );

  /** Opens a tab, closing whatever was over it. */
  function select(next: Tab) {
    tab = next;
    over = null;
    try {
      localStorage.setItem(TAB_KEY, next);
    } catch {
      // Remembering the tab is a convenience only.
    }
  }

  /**
   * Opens search over the tab on a page something else asked for (an item check, a link, a tool),
   * on one kind of thing when asked. The cursor stays out of the box, so the game keeps the keys.
   */
  function search(mode?: typeof lookup.mode) {
    if (mode) lookup.mode = mode;
    landed += 1;
    if (over?.kind !== "search") over = { kind: "search", focus: false };
  }

  /** Where a link to one of the old panels lands now. */
  function showPanel(panel: Panel) {
    const to = tabOfPanel(panel);
    if (to === "search") search();
    else select(to);
  }

  function storedTab(): Tab {
    try {
      const saved = tabId(localStorage.getItem(TAB_KEY));
      if (saved) return saved;
    } catch {
      // Private mode or blocked storage: start on the first tab.
    }
    return "now";
  }

  async function togglePin() {
    const panel = tabInfo(tab).panel;
    live.pins = pinned ? await backend.unpinPanel(panel) : await backend.pinPanel(panel);
  }

  async function toggleLock() {
    await backend.setClickThrough(!live.status.clickThrough);
    live.status.clickThrough = !live.status.clickThrough;
  }

  const liveOn = $derived(live.liveStatus.state === "reading" || live.liveStatus.state === "waiting");
  let liveError = $state<string | null>(null);

  const liveTitle = $derived.by(() => {
    const s = live.liveStatus;
    const key = keyHint("ToggleLive");
    if (s.state === "reading") return `Reading ${s.window} every ${display.live.interval} s. Click to stop${key}.`;
    if (s.state === "waiting") return `${s.reason} Click to stop${key}.`;
    if (!display.live.enabled) return "Live reading is off; turn it on in Settings.";
    return `Start reading the game window${key}`;
  });

  /**
   * One line under the header for what needs the player's attention, most urgent first. `report`
   * offers the bug form, for something Leafline got wrong rather than a setting to change.
   */
  const notice = $derived.by((): { tone: "bad" | "warn" | "good" | "quiet"; text: string; report?: boolean } | null => {
    const s = live.liveStatus;
    if (liveError) return { tone: "bad", text: liveError, report: true };
    if (s.state === "failed") return { tone: "bad", text: `Live reading stopped: ${s.message}`, report: true };
    if (live.itemRead?.problem)
      return { tone: "warn", text: `Couldn't read the item: ${live.itemRead.problem}`, report: true };
    if (live.itemRead?.found === 0)
      return {
        tone: "quiet",
        text: `No item tooltip found. Hover an item in game, then ${hotkey("ReadItem") ? `press ${hotkey("ReadItem")}` : "use the Read item hotkey"}. If one was showing, Save frame in Tools keeps that picture so it can be looked at.`,
      };
    if (live.characterRead?.problem) return { tone: "warn", text: live.characterRead.problem, report: true };
    if (live.characterRead) {
      const { build, result } = live.characterRead;
      const unknown = result.unknown.length ? ` Not on this job: ${result.unknown.join(", ")}.` : "";
      const changed = result.changes.length ? `${build} updated: ${result.changes.join(", ")}.` : `${build} already matches your windows.`;
      return { tone: "good", text: changed + unknown };
    }
    if (live.savedFrame) return { tone: "good", text: `Saved ${fileName(live.savedFrame)}. Settings → Data opens the folder.` };
    if (s.state === "waiting") return { tone: "quiet", text: s.reason };
    if (s.state === "reading" && s.hdr)
      return { tone: "warn", text: "Windows HDR is on for the game's display, so readings will be off. Turn HDR off, then restart the game." };
    if (live.analysis?.failed.length)
      return { tone: "warn", text: `Can't read the ${live.analysis.failed.join(", ")} right now.`, report: true };
    if (s.state === "reading" && s.smoothedUi)
      return { tone: "quiet", text: "UI Size Default Ratio smooths the game's text, so some readings may be missed. Ideal Ratio, in the game's Graphics options, reads best." };
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

  function openTool(tool: ToolInfo) {
    const action = tool.action;
    if ("page" in action) over = { kind: "tool", page: action.page };
    else if ("tab" in action) select(action.tab);
    else if ("search" in action && action.search === "gear") {
      // Straight to the best gear for your level, not the item list.
      itemView.id = null;
      itemView.query = { ...blankQuery(), kind: "equip" };
      gearView.forMe = true;
      search("items");
    } else if ("search" in action) search("items");
    else {
      over = null;
      if (action.window === "training") void backend.showTraining();
      else void backend.showBuild();
    }
  }

  function windowAction(action: "desk" | "compact" | "frame" | "settings" | "hide") {
    over = null;
    if (action === "desk") void backend.showDesk();
    else if (action === "compact") void backend.setCompact(true);
    else if (action === "frame") void saveFrame();
    else if (action === "settings") void backend.showSettings();
    else void backend.hideOverlay();
  }

  const toggleOver = (kind: "search" | "tools") =>
    (over = over?.kind === kind ? null : kind === "search" ? { kind, focus: true } : { kind });
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
    <ShellHeader
      {liveTitle}
      {pinned}
      {pinTitle}
      searching={over?.kind === "search"}
      tooling={over?.kind === "tools" || over?.kind === "tool"}
      onlive={toggleLive}
      onsearch={() => toggleOver("search")}
      onpin={togglePin}
      onlock={toggleLock}
      ontools={() => toggleOver("tools")}
    />

    {#if notice}
      <div class="notice {notice.tone}" role="status">
        <span>{notice.text}</span>
        {#if notice.report}
          <button class="report" use:tip={"GitHub's bug form, in your browser"} onclick={() => backend.openLink(reportUrl())}>
            Report it
          </button>
        {/if}
        {#if notice.tone === "bad"}
          <button class="dismiss" aria-label="Dismiss" onclick={() => (liveError = null)}><X size={12} /></button>
        {/if}
      </div>
    {/if}

    <NextStep />

    <div class="stage">
      <div class="main" id="panel" role="tabpanel" aria-labelledby="tab-{tab}">
        {#key tab}
          {#if tab === "now"}
            <NowScreen analysis={live.analysis} session={live.session} ongo={select} />
          {:else if live.analysis || live.session || tab === "me" || tab === "quests"}
            <!-- A build can be planned and quests looked up with nothing read yet. -->
            <div class="panel"><PanelView panel={tabInfo(tab).panel} analysis={live.analysis} session={live.session} shelled /></div>
          {:else}
            <Welcome />
          {/if}
        {/key}
      </div>

      {#if over?.kind === "search"}
        <Sheet title="Search" onclose={() => (over = null)}>
          <SearchSheet focus={over.focus} {landed} ontool={openTool} />
        </Sheet>
      {:else if over?.kind === "tools"}
        <Sheet title="Tools" onclose={() => (over = null)}>
          <ToolsList canSaveFrame={live.liveStatus.state === "reading"} ontool={openTool} onwindow={windowAction} />
        </Sheet>
      {:else if over?.kind === "tool"}
        <Sheet title={toolPageTitle(over.page)} onclose={() => (over = null)} onback={() => (over = { kind: "tools" })}>
          <div class="tool">
            {#if over.page === "crafting"}
              <CraftingPanel />
            {:else if over.page === "sell"}
              <BagCheck />
            {:else if over.page === "timers"}
              <TimersCard />
            {:else if over.page === "drops"}
              <DropLogView character={characters.viewing ?? ""} />
            {:else if over.page === "tracked"}
              <TrackedItems map={live.analysis?.map ?? null} />
            {:else if over.page === "potions"}
              <PotionPage />
            {:else if over.page === "route"}
              <RoutePage />
            {:else if over.page === "share"}
              <SharePage />
            {:else if over.page === "characters"}
              {#if characters.known.length > 1}
                <CharacterPicker />
              {:else}
                <p class="tool-note">
                  {characters.active ? `Leafline has only seen ${characters.active} so far.` : "Leafline hasn't read a character's name yet."}
                  Once it has sessions for a second character, pick here whose numbers the tabs show.
                </p>
              {/if}
            {/if}
          </div>
        </Sheet>
      {/if}
    </div>

    <BottomNav {tab} {fresh} pinned={isPinned} onselect={select} />
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
    border: var(--line-w) solid var(--window-line);
    border-radius: var(--radius-lg);
    background: var(--bg);
    overflow: hidden;
    transition: opacity 0.2s ease;
  }

  .shell.compact {
    height: auto;
    max-height: 100vh;
    border-radius: var(--radius);
  }

  .notice {
    display: flex;
    flex: none;
    align-items: flex-start;
    gap: 6px;
    margin: 0 10px 10px;
    padding: 7px 8px 7px 12px;
    border-radius: var(--radius);
    background: var(--surface-2);
    color: var(--muted);
    font-size: var(--text-sm);
    line-height: 1.4;
  }

  .notice > span {
    flex: 1;
  }

  .notice.warn {
    background: var(--warn-soft);
    color: var(--warn);
  }

  .notice.bad {
    background: var(--bad-soft);
    color: var(--bad);
  }

  .notice.good {
    background: var(--good-soft);
    color: var(--good);
  }

  .report {
    flex: none;
    padding: 0;
    border: 0;
    background: none;
    color: inherit;
    font: inherit;
    text-decoration: underline;
    text-underline-offset: 2px;
    cursor: pointer;
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

  /* The tab and whatever is open over it. */
  .stage {
    position: relative;
    display: flex;
    flex: 1;
    flex-direction: column;
    min-height: 0;
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

  .tool {
    display: grid;
    align-content: start;
    padding: 10px 0;
  }

  /* The drop log brings no margins of its own: it was made for the training log's wide page. */
  .tool :global(.split),
  .tool :global(.empty),
  .tool :global(.problem) {
    margin-right: 10px;
    margin-left: 10px;
  }

  .tool-note {
    margin: 4px 14px;
    color: var(--muted);
    font-size: var(--text-sm);
    line-height: 1.5;
  }
</style>
