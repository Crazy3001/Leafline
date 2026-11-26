<script lang="ts">
  // The Quests tab: every quest in the map data, region by region, with where each stands for the
  // character as the game's Quest window and Quest Helper last showed it, what is ready to hand in
  // first and questlines folded into one row. A quest picked opens beside the list (in place of it
  // in a narrow window). The list needs nothing from the game, so it works with live reading off.
  // A pinned window shows only what you're on now.
  import { can } from "$lib/platform";
  import { clipped, tip } from "$lib/tip.svelte";
  import { FilterBoxState } from "$lib/filterbox.svelte";
  import { ArrowLeft, LoaderCircle, Search, SlidersHorizontal, X } from "@lucide/svelte";
  import { untrack } from "svelte";

  import Card from "./Card.svelte";
  import LookupSplit from "./lookup/LookupSplit.svelte";
  import MapDataPanel from "./map/MapDataPanel.svelte";
  import QuestCard from "./QuestCard.svelte";
  import QuestSheetCard from "./QuestSheetCard.svelte";
  import FollowedQuests from "./quest/FollowedQuests.svelte";
  import LoggedQuests from "./quest/LoggedQuests.svelte";
  import QuestLogCard from "./quest/QuestLogCard.svelte";
  import QuestBook from "./quest/QuestBook.svelte";
  import QuestLine from "./quest/QuestLine.svelte";
  import QuestMark from "./quest/QuestMark.svelte";
  import { backend } from "$lib/api";
  import { characters } from "$lib/characters.svelte";
  import { formatCompact } from "$lib/format";
  import { mapData, watchMapData } from "$lib/mapdata.svelte";
  import { blankSearch, followedQuests, following, noteCharacter, questView, watchQuests } from "$lib/quests.svelte";
  import { doneIds, hasLog, noteSeriesSteps, questLog, refreshRegionTotals, watchQuestLog } from "$lib/questlog.svelte";
  import type { MapReading, QuestPage, QuestReading, QuestRow, QuestSearch, SessionSummary, WholeLog } from "$lib/types";

  let {
    quest = null,
    map = null,
    session = null,
    level = null,
    character = null,
    pinned = false,
  }: {
    quest?: QuestReading | null;
    /** The character read now, whose followed quests show. */
    character?: string | null;
    map?: MapReading | null;
    session?: SessionSummary | null;
    level?: number | null;
    pinned?: boolean;
  } = $props();

  const REPEATS = [
    { id: "daily", label: "Daily" },
    { id: "weekly", label: "Weekly" },
  ];
  /** Long enough to skip the keys between letters, short enough to feel immediate. */
  const SEARCH_DELAY = 120;

  watchQuests();
  watchMapData();
  watchQuestLog();

  $effect(() => noteCharacter(character));

  const ready = $derived(mapData.status?.ready ?? false);
  const search = $derived(questView.search);

  let regions = $state<string[]>([]);
  const filterBox = new FilterBoxState("quests");
  let page = $state<QuestPage | null>(null);
  let searching = $state(false);

  const narrowed = $derived(
    [search.region, search.repeat, search.atLevel, search.noPrior || null, search.hideDone || null].filter((v) => v != null)
      .length,
  );
  /** The quests the log shows done, as a key: a search asks again when it changes. */
  const done = $derived(doneIds());
  const doneKey = $derived(done.join(","));
  const browsing = $derived(search.text.trim() !== "" || narrowed > 0);
  const asked = $derived(JSON.stringify(search));

  $effect(() => {
    void mapData.status?.dataUpdatedAt;
    if (ready) {
      backend.questRegions().then((r) => (regions = r));
      refreshRegionTotals();
    }
  });

  $effect(() => {
    const key = asked;
    void doneKey;
    if (!browsing || !ready) {
      page = null;
      return;
    }
    searching = true;
    const timer = setTimeout(() => {
      untrack(() => backend.searchQuests({ ...search, done, offset: 0 }))
        .then((result) => {
          if (key === asked) page = result;
        })
        .finally(() => {
          if (key === asked) searching = false;
        });
    }, SEARCH_DELAY);
    return () => clearTimeout(timer);
  });

  // Every quest with where it stands, asked again whenever the log, the level, the Quest Helper
  // or the map data changes. Whose: the character looked at, as the quest log goes by.
  let whole = $state<WholeLog | null>(null);
  const helperTitles = $derived((quest?.quests ?? []).map((q) => q.title).filter(Boolean));
  const wholeKey = $derived(`${characters.active ?? following.character}|${level}|${helperTitles.join("\n")}`);
  $effect(() => {
    void questLog.log;
    void mapData.status?.dataUpdatedAt;
    const asked = wholeKey;
    if (pinned || !ready) return;
    untrack(() =>
      backend
        .wholeQuestLog(characters.active ?? following.character ?? null, level, helperTitles)
        .then((found) => {
          if (asked !== wholeKey) return;
          whole = found;
          noteSeriesSteps(found);
        })
        .catch(() => {}),
    );
  });

  function pick(id: number) {
    questView.id = id;
  }

  async function more() {
    if (!page) return;
    const key = asked;
    const next = await backend.searchQuests({ ...search, done, offset: page.quests.length });
    if (key === asked && page) page.quests.push(...next.quests);
  }

  function setSearch(change: Partial<QuestSearch>) {
    questView.search = { ...questView.search, ...change, offset: 0 };
    questView.id = null;
  }

  function clear() {
    questView.search = blankSearch();
    questView.id = null;
  }

  /** What a row says under the name. */
  function line(row: QuestRow) {
    const parts = [
      row.levelMin ? `Lv ${row.levelMin}` : null,
      row.giver,
      row.region,
      row.repeat ? row.repeat[0].toUpperCase() + row.repeat.slice(1) : null,
    ];
    return parts.filter(Boolean).join(" · ");
  }
</script>

{#snippet rows(list: QuestRow[])}
  <ul class="results">
    {#each list as row (row.id)}
      <li class:picked={row.id === questView.id}>
        <button aria-current={row.id === questView.id || undefined} onclick={() => (questView.id = row.id)}>
          <span class="row-name">{row.name}<QuestMark id={row.id} label /></span>
          <span use:clipped class="row-line">{line(row)}{#if row.exp}{line(row) ? " · " : ""}<span class="exp tabular">{formatCompact(row.exp)} EXP</span>{/if}</span>
        </button>
      </li>
    {/each}
  </ul>
{/snippet}

<!-- A pinned window shows what is tracked, and a quest opened from a link; the book is in the tab. -->
{#if ready && !pinned}
  <div class="controls" class:pinned>
    <div class="search">
      <label class="field">
        <Search size={13} />
        <input
          type="search"
          placeholder="Search quests, people, monsters"
          aria-label="Search quests"
          value={search.text}
          oninput={(e) => setSearch({ text: e.currentTarget.value })}
        />
      </label>
      <button
        class="tool"
        class:active={filterBox.open || narrowed > 0}
        use:tip={"Filters"}
        aria-label="Filters"
        aria-expanded={filterBox.open}
        onclick={() => (filterBox.open = !filterBox.open)}
      >
        <SlidersHorizontal size={13} />{#if narrowed}<span class="tabular">{narrowed}</span>{/if}
      </button>
      {#if browsing || questView.id != null}
        <button class="tool" use:tip={"Clear the search"} aria-label="Clear the search" onclick={clear}><X size={13} /></button>
      {/if}
    </div>

    {#if filterBox.open}
      <div class="filters">
        <label>
          <span>Region</span>
          <select value={search.region ?? ""} onchange={(e) => setSearch({ region: e.currentTarget.value || null })}>
            <option value="">Any</option>
            {#each regions as region (region)}<option value={region}>{region}</option>{/each}
          </select>
        </label>
        <label>
          <span>Repeats</span>
          <select value={search.repeat ?? ""} onchange={(e) => setSearch({ repeat: e.currentTarget.value || null })}>
            <option value="">Any</option>
            {#each REPEATS as repeat (repeat.id)}<option value={repeat.id}>{repeat.label}</option>{/each}
          </select>
        </label>
        {#if level != null}
          <label class="check">
            <input type="checkbox" checked={search.atLevel != null} onchange={(e) => setSearch({ atLevel: e.currentTarget.checked ? level : null })} />
            <span>Only what Lv {level} can start</span>
          </label>
        {/if}
        <label class="check">
          <input type="checkbox" checked={!!search.noPrior} onchange={(e) => setSearch({ noPrior: e.currentTarget.checked || undefined })} />
          <span>{hasLog() ? "Only quests you can start: none needed first, or only done ones" : "Only quests that need no other first"}</span>
        </label>
        {#if hasLog()}
          <label class="check">
            <input type="checkbox" checked={!!search.hideDone} onchange={(e) => setSearch({ hideDone: e.currentTarget.checked || undefined })} />
            <span>Only quests not done</span>
          </label>
        {/if}
      </div>
    {/if}
  </div>
{/if}

{#if ready && !pinned}
  <LookupSplit open={questView.id != null} onback={() => (questView.id = null)}>
    {#snippet list()}
      {#if browsing}
        <div class="list">
          {#if page}
            {#if page.quests.length}
              <p class="count tabular">{page.total.toLocaleString()} {page.total === 1 ? "quest" : "quests"}</p>
              {@render rows(page.quests)}
              {#if page.quests.length < page.total}<button class="back inline" onclick={more}>Show more</button>{/if}
            {:else}
              <p class="quiet">No quests match.</p>
            {/if}
          {:else if searching}
            <p class="quiet"><span class="spin"><LoaderCircle size={13} /></span> Searching…</p>
          {/if}
        </div>
      {:else}
        <QuestBook {whole} helper={quest} selected={questView.id} onpick={pick} />
      {/if}
    {/snippet}
    {#snippet detail()}
      {#if questView.id != null}
        <!-- The line first: it is short, and its steps open in the card under it. -->
        <QuestLine {whole} id={questView.id} helper={quest} onpick={pick} />
        <QuestSheetCard id={questView.id} {map} {quest} />
      {/if}
    {/snippet}
    {#snippet empty()}
      <p class="empty">Pick a quest: what it asks, where to go and what it pays show here.</p>
    {/snippet}
  </LookupSplit>
{:else if ready && questView.id != null}
  <button class="back" class:pinned onclick={() => (questView.id = null)}><ArrowLeft size={12} />Back</button>
  <QuestSheetCard id={questView.id} {map} {quest} plain={pinned} />
{:else}
  <!-- A pinned window: what can be handed in now first, then what the Quest Helper tracks. -->
  {#if ready}<LoggedQuests handIn tracked={quest} {map} {session} plain={pinned} />{/if}
  {#if quest}
    <QuestCard {quest} {map} {session} plain={pinned} />
  {:else if !(ready && (followedQuests().length || questLog.log?.quests.some((q) => q.state === "inProgress")))}
    <p class="empty">
      {#if can.liveReading}
        Open the Quest Helper in game to track quest counters here, or search for a quest and follow it.
      {:else}
        Search for a quest and follow it: its steps and where to go show here.
      {/if}
    </p>
  {/if}
  {#if ready}<FollowedQuests tracked={quest} {map} {session} plain={pinned} />{/if}
  {#if ready}<LoggedQuests handIn={false} tracked={quest} {map} {session} plain={pinned} />{/if}
  {#if !pinned && !ready}<QuestLogCard />{/if}
  {#if !ready && !pinned}
    <Card title="Quest search" plain={pinned}>
      <p class="lead">Search every quest once the map data is downloaded.</p>
      <MapDataPanel compact />
    </Card>
  {/if}
{/if}

<style>
  .row-line .exp {
    color: var(--exp-text);
  }

  .lead {
    margin: 0 0 var(--space-2);
    color: var(--muted);
    font-size: var(--text-md);
  }

  /* A card pads itself; these sit beside cards, so they match it. A pinned window pads its body. */
  .controls {
    padding: var(--space-2) var(--space-3) 0;
  }

  .list {
    padding: 0 var(--space-3) var(--space-2);
  }

  .controls.pinned {
    padding: 0;
  }

  .search {
    display: flex;
    gap: var(--space-1);
    margin-bottom: var(--space-2);
  }

  .field {
    display: flex;
    flex: 1;
    align-items: center;
    gap: var(--space-1-5);
    min-width: 0;
    height: 28px;
    padding: 0 var(--space-2);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--faint);
  }

  .field:focus-within {
    border-color: color-mix(in srgb, var(--accent) 60%, transparent);
  }

  .field input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: 0;
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: var(--text-md);
  }

  .field input::-webkit-search-cancel-button {
    display: none;
  }

  .tool {
    display: inline-flex;
    flex: none;
    align-items: center;
    justify-content: center;
    gap: var(--space-0-5);
    min-width: 28px;
    height: 28px;
    padding: 0 var(--space-1-5);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--muted);
    font-size: var(--text-xs);
    cursor: pointer;
  }

  .tool:hover,
  .tool:focus-visible {
    color: var(--text);
    outline: none;
  }

  .tool.active {
    border-color: color-mix(in srgb, var(--accent) 45%, transparent);
    color: var(--accent-text);
  }

  .filters {
    display: grid;
    gap: var(--space-1);
    margin-bottom: var(--space-2);
    padding: var(--space-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    font-size: var(--text-md);
  }

  .filters label {
    display: grid;
    grid-template-columns: 48px 1fr;
    align-items: center;
    gap: var(--space-1-5);
    color: var(--muted);
  }

  .filters select {
    min-width: 0;
    height: 24px;
    padding: 0 var(--space-1);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--text);
    font: inherit;
  }

  .filters .check {
    display: flex;
  }

  .count {
    margin: 0 0 var(--space-1);
    color: var(--faint);
    font-size: var(--text-xs);
  }

  .results {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .results li + li {
    border-top: 1px solid var(--border);
  }

  .results button {
    display: grid;
    gap: 1px;
    width: 100%;
    padding: var(--space-1) var(--space-1);
    border: 0;
    background: transparent;
    text-align: left;
    cursor: pointer;
  }

  .results li.picked button {
    background: color-mix(in srgb, var(--accent) 14%, transparent);
  }

  .results button:hover,
  .results button:focus-visible {
    background: var(--surface-3);
    outline: none;
  }

  .row-name {
    font-size: var(--text-md);
    font-weight: var(--weight-strong);
  }

  .row-line {
    overflow: hidden;
    color: var(--muted);
    font-size: var(--text-xs);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .back {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    margin: var(--space-0-5) var(--space-3) var(--space-1-5);
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--muted);
    font-size: var(--text-md);
    cursor: pointer;
  }

  .back.pinned,
  .back.inline {
    margin-inline: 0;
  }

  .back:hover,
  .back:focus-visible {
    color: var(--text);
    outline: none;
  }

  .quiet {
    margin: 0 0 var(--space-1-5);
    color: var(--faint);
    font-size: var(--text-md);
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

  .empty {
    margin: 0;
    padding: var(--space-4) var(--space-3);
    color: var(--faint);
    font-size: var(--text-md);
    text-align: center;
  }

</style>
