<script lang="ts">
  // The Quests tab: what the Quest Helper is tracking, and a book of every quest in the map data.
  // The book needs nothing from the game, so it works with live reading off.
  import { ArrowLeft, LoaderCircle, Search, SlidersHorizontal, X } from "@lucide/svelte";
  import { untrack } from "svelte";

  import Card from "./Card.svelte";
  import MapDataPanel from "./map/MapDataPanel.svelte";
  import QuestCard from "./QuestCard.svelte";
  import QuestSheetCard from "./QuestSheetCard.svelte";
  import { backend } from "$lib/api";
  import { formatCompact } from "$lib/format";
  import { OSMS_CREDIT, OSMS_URL, mapData, watchMapData } from "$lib/mapdata.svelte";
  import { blankSearch, questView, watchQuests } from "$lib/quests.svelte";
  import type { MapReading, QuestPage, QuestReading, QuestRow, QuestSearch, SessionSummary } from "$lib/types";

  let {
    quest = null,
    map = null,
    session = null,
    level = null,
    pinned = false,
  }: {
    quest?: QuestReading | null;
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

  const ready = $derived(mapData.status?.ready ?? false);
  const search = $derived(questView.search);

  let regions = $state<string[]>([]);
  let showFilters = $state(false);
  let page = $state<QuestPage | null>(null);
  let searching = $state(false);

  const narrowed = $derived([search.region, search.repeat, search.atLevel].filter((v) => v != null).length);
  const browsing = $derived(search.text.trim() !== "" || narrowed > 0);
  const asked = $derived(JSON.stringify(search));

  $effect(() => {
    void mapData.status?.dataUpdatedAt;
    if (ready) backend.questRegions().then((r) => (regions = r));
  });

  $effect(() => {
    const key = asked;
    if (!browsing || !ready) {
      page = null;
      return;
    }
    searching = true;
    const timer = setTimeout(() => {
      untrack(() => backend.searchQuests({ ...search, offset: 0 }))
        .then((result) => {
          if (key === asked) page = result;
        })
        .finally(() => {
          if (key === asked) searching = false;
        });
    }, SEARCH_DELAY);
    return () => clearTimeout(timer);
  });

  async function more() {
    if (!page) return;
    const key = asked;
    const next = await backend.searchQuests({ ...search, offset: page.quests.length });
    if (key === asked && page) page.quests.push(...next.quests);
  }

  function setSearch(change: Partial<QuestSearch>) {
    questView.search = { ...questView.search, ...change, offset: 0 };
    questView.id = null;
  }

  function clear() {
    questView.search = blankSearch();
    questView.id = null;
    showFilters = false;
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
      <li>
        <button onclick={() => (questView.id = row.id)}>
          <span class="row-name">{row.name}</span>
          <span class="row-line">{line(row)}{#if row.exp}{line(row) ? " · " : ""}<span class="exp tabular">{formatCompact(row.exp)} EXP</span>{/if}</span>
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
        class:active={showFilters || narrowed > 0}
        title="Filters"
        aria-label="Filters"
        aria-expanded={showFilters}
        onclick={() => (showFilters = !showFilters)}
      >
        <SlidersHorizontal size={13} />{#if narrowed}<span class="tabular">{narrowed}</span>{/if}
      </button>
      {#if browsing || questView.id != null}
        <button class="tool" title="Clear the search" aria-label="Clear the search" onclick={clear}><X size={13} /></button>
      {/if}
    </div>

    {#if showFilters}
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
      </div>
    {/if}
  </div>
{/if}

{#if ready && questView.id != null}
  <button class="back" class:pinned onclick={() => (questView.id = null)}><ArrowLeft size={12} />Back</button>
  <QuestSheetCard id={questView.id} {map} plain={pinned} />
{:else if ready && browsing}
  <div class="list" class:pinned>
    {#if page}
      {#if page.quests.length}
        <p class="count tabular">{page.total.toLocaleString()} {page.total === 1 ? "quest" : "quests"}</p>
        {@render rows(page.quests)}
        {#if page.quests.length < page.total}<button class="back inline" onclick={more}>Show more</button>{/if}
      {:else}
        <p class="quiet">No quests match.</p>
      {/if}
      <button class="credit" onclick={() => backend.openLink(OSMS_URL)}>{OSMS_CREDIT}</button>
    {:else if searching}
      <p class="quiet"><span class="spin"><LoaderCircle size={13} /></span> Searching…</p>
    {/if}
  </div>
{:else}
  {#if quest}
    <QuestCard {quest} {map} {session} plain={pinned} />
  {:else}
    <p class="empty">Open the Quest Helper in game to track quest counters here.</p>
  {/if}
  {#if !ready && !pinned}
    <Card title="Quest search" plain={pinned}>
      <p class="lead">Search every quest once the map data is downloaded.</p>
      <MapDataPanel compact />
    </Card>
  {/if}
{/if}

<style>
  .row-line .exp {
    color: var(--exp);
  }

  .lead {
    margin: 0 0 10px;
    color: var(--muted);
    font-size: var(--text-sm);
  }

  /* A card pads itself; these sit beside cards, so they match it. A pinned window pads its body. */
  .controls {
    padding: 10px 12px 0;
  }

  .list {
    padding: 0 12px 10px;
  }

  .controls.pinned,
  .list.pinned {
    padding: 0;
  }

  .search {
    display: flex;
    gap: 4px;
    margin-bottom: 8px;
  }

  .field {
    display: flex;
    flex: 1;
    align-items: center;
    gap: 6px;
    min-width: 0;
    height: 28px;
    padding: 0 8px;
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
    font-size: var(--text-sm);
  }

  .field input::-webkit-search-cancel-button {
    display: none;
  }

  .tool {
    display: inline-flex;
    flex: none;
    align-items: center;
    justify-content: center;
    gap: 3px;
    min-width: 28px;
    height: 28px;
    padding: 0 6px;
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
    color: var(--accent);
  }

  .filters {
    display: grid;
    gap: 5px;
    margin-bottom: 8px;
    padding: 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    font-size: var(--text-sm);
  }

  .filters label {
    display: grid;
    grid-template-columns: 48px 1fr;
    align-items: center;
    gap: 6px;
    color: var(--muted);
  }

  .filters select {
    min-width: 0;
    height: 24px;
    padding: 0 4px;
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
    margin: 0 0 4px;
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
    padding: 5px 4px;
    border: 0;
    background: transparent;
    text-align: left;
    cursor: pointer;
  }

  .results button:hover,
  .results button:focus-visible {
    background: var(--surface-3);
    outline: none;
  }

  .row-name {
    font-size: var(--text-md);
    font-weight: 600;
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
    gap: 4px;
    margin: 2px 12px 6px;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--muted);
    font-size: var(--text-sm);
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
    margin: 0 0 6px;
    color: var(--faint);
    font-size: var(--text-sm);
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
    padding: 18px 10px;
    color: var(--faint);
    font-size: var(--text-sm);
    text-align: center;
  }

  .credit {
    display: block;
    margin-top: 8px;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--faint);
    font-size: var(--text-xs);
    text-align: left;
    cursor: pointer;
  }

  .credit:hover,
  .credit:focus-visible {
    color: var(--muted);
    outline: none;
  }
</style>
