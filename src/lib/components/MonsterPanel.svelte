<script lang="ts">
  // The Monsters half of the Lookup tab: what is on this map and near the character's level, and
  // a search of every monster in the map data by name, drop or map.
  import { ArrowLeft, LoaderCircle, Search, Skull, SlidersHorizontal, X } from "@lucide/svelte";
  import { untrack } from "svelte";

  import Card from "./Card.svelte";
  import MapDataPanel from "./map/MapDataPanel.svelte";
  import MonsterCard from "./MonsterCard.svelte";
  import Section from "./map/Section.svelte";
  import { backend } from "$lib/api";
  import { formatCompact, formatNumber } from "$lib/format";
  import { OSMS_CREDIT, OSMS_URL, mapData, watchMapData } from "$lib/mapdata.svelte";
  import { backFromMonster, blankMonsterQuery, monsterView, openMonster } from "$lib/monsters.svelte";
  import type { MapMatch, MapReading, MonsterFilters, MonsterPage, MonsterQuery, MonsterRow, MobRow, QuestReading, SessionSummary } from "$lib/types";

  let {
    map = null,
    level = null,
    quest = null,
    session = null,
    pinned = false,
  }: {
    map?: MapReading | null;
    level?: number | null;
    quest?: QuestReading | null;
    session?: SessionSummary | null;
    pinned?: boolean;
  } = $props();

  const KINDS = [
    { id: null, label: "All" },
    { id: "field", label: "Field" },
    { id: "boss", label: "Bosses" },
    { id: "undead", label: "Undead" },
  ];
  const SORTS = [
    { id: "", label: "Level" },
    { id: "name", label: "Name" },
    { id: "hp", label: "Most HP" },
    { id: "exp", label: "Most EXP" },
    { id: "efficiency", label: "Most EXP for its HP" },
  ];
  /** Monsters this many levels from the character count as near it. */
  const NEAR_LEVELS = 5;
  const NEAR_SHOWN = 6;
  /** Long enough to skip the keys between letters, short enough to feel immediate. */
  const SEARCH_DELAY = 120;

  watchMapData();

  const ready = $derived(mapData.status?.ready ?? false);
  const query = $derived(monsterView.query);

  let filters = $state<MonsterFilters | null>(null);
  let showFilters = $state(false);
  let page = $state<MonsterPage | null>(null);
  let searching = $state(false);
  let here = $state<MapMatch | null>(null);
  let near = $state<MonsterRow[]>([]);

  const narrowed = $derived([query.weakTo, query.region, query.levelMin, query.levelMax, query.sort].filter((v) => v != null).length);
  const browsing = $derived(query.text.trim() !== "" || query.kind != null || narrowed > 0);
  const asked = $derived(JSON.stringify(query));
  const mapName = $derived(map?.name ?? null);
  const mapStreet = $derived(map?.street ?? null);
  const mapTwin = $derived(map?.twin ?? null);

  $effect(() => {
    void mapData.status?.dataUpdatedAt;
    if (!ready) return;
    backend.monsterFilters().then((f) => (filters = f));
    // So a monster can be found by what it drops. Nothing is sent, and it renews weekly at most.
    backend.downloadDrops().catch(() => {});
  });

  // Follows the query alone: every live reading brings new objects with the same content.
  $effect(() => {
    const key = asked;
    if (!ready) {
      page = null;
      return;
    }
    searching = true;
    const timer = setTimeout(() => {
      untrack(() => backend.searchMonsters({ ...query, offset: 0 }))
        .then((result) => {
          if (key === asked) page = result;
        })
        .finally(() => {
          if (key === asked) searching = false;
        });
    }, SEARCH_DELAY);
    return () => clearTimeout(timer);
  });

  $effect(() => {
    const name = mapName;
    const twin = mapTwin;
    here = null;
    if (!name || !ready) return;
    backend.findMap(name, untrack(() => mapStreet), twin).then((match) => {
      if (mapName === name && mapTwin === twin) here = match;
    });
  });

  $effect(() => {
    const at = level;
    near = [];
    if (at == null || !ready) return;
    const around = { levelMin: Math.max(1, at - NEAR_LEVELS), levelMax: at + NEAR_LEVELS, near: at, kind: "field" };
    backend.searchMonsters({ ...blankMonsterQuery(), ...around }).then((result) => {
      if (level === at) near = result.monsters.slice(0, NEAR_SHOWN);
    });
  });

  async function more() {
    if (!page) return;
    const key = asked;
    const next = await backend.searchMonsters({ ...query, offset: page.monsters.length });
    if (key === asked && page) page.monsters.push(...next.monsters);
  }

  function setQuery(change: Partial<MonsterQuery>) {
    monsterView.query = { ...monsterView.query, ...change, offset: 0 };
    monsterView.id = null;
    monsterView.trail = [];
  }

  function clear() {
    setQuery(blankMonsterQuery());
    showFilters = false;
  }

  /** The column it sorts by, with Level as the way back. */
  function sortBy(sort: string | null) {
    setQuery({ sort: query.sort === sort ? null : sort });
  }

  const number = (value: string) => (value === "" ? null : Math.max(0, Math.floor(Number(value) || 0)));
</script>

{#snippet name(mob: { name: string; boss: boolean; undead: boolean })}
  <span class="name">
    <span class="label">{mob.name}</span>
    {#if mob.boss}<span class="boss" title="Boss"><Skull size={11} /></span>{/if}
    {#if mob.undead}<span class="tag" title="Undead">undead</span>{/if}
  </span>
{/snippet}

{#snippet numbers(mob: { level: number; hp: number; exp: number })}
  <span class="tabular">{mob.level}</span>
  <span class="tabular">{formatCompact(mob.hp)}</span>
  <span class="tabular">{formatNumber(mob.exp)}</span>
{/snippet}

{#snippet head(sortable: boolean)}
  <div class="row head">
    <span>Monster</span>
    {#if sortable}
      <button class:on={query.sort == null} title="Lowest level first" onclick={() => sortBy(null)}>Lv</button>
      <button class:on={query.sort === "hp"} title="Most HP first" onclick={() => sortBy("hp")}>HP</button>
      <button class:on={query.sort === "exp"} title="Most EXP first" onclick={() => sortBy("exp")}>EXP</button>
    {:else}
      <span>Lv</span><span>HP</span><span>EXP</span>
    {/if}
  </div>
{/snippet}

{#snippet rows(list: MonsterRow[], sortable: boolean)}
  {@render head(sortable)}
  <ul class="rows">
    {#each list as mob (mob.id)}
      <li>
        <button class="row" onclick={() => openMonster(mob.id)}>
          {@render name(mob)}
          {@render numbers(mob)}
          {#if mob.note}<span class="note">{mob.note}</span>{/if}
        </button>
      </li>
    {/each}
  </ul>
{/snippet}

{#snippet onMap(list: MobRow[])}
  {@render head(false)}
  <ul class="rows">
    {#each list as mob (mob.id)}
      <li>
        <button class="row" onclick={() => openMonster(mob.id)}>
          {@render name(mob)}
          {@render numbers(mob)}
        </button>
      </li>
    {/each}
  </ul>
{/snippet}

{#if !ready}
  <Card title="Monster search" plain={pinned}>
    <p class="lead">Search every monster once the map data is downloaded.</p>
    <MapDataPanel compact />
  </Card>
{:else}
  <div class="controls" class:pinned>
    <div class="search">
      <label class="field">
        <Search size={13} />
        <input
          type="search"
          placeholder="Monster, what it drops, or a map"
          aria-label="Search monsters"
          value={query.text}
          oninput={(e) => setQuery({ text: e.currentTarget.value })}
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
      {#if browsing || monsterView.id != null}
        <button class="tool" title="Clear the search" aria-label="Clear the search" onclick={clear}><X size={13} /></button>
      {/if}
    </div>

    {#if monsterView.id == null}
      <div class="kinds" role="radiogroup" aria-label="Kind of monster">
        {#each KINDS as kind (kind.label)}
          <button role="radio" aria-checked={query.kind === kind.id} onclick={() => setQuery({ kind: kind.id })}>{kind.label}</button>
        {/each}
      </div>
    {/if}

    {#if showFilters}
      <div class="filters">
        <label>
          <span>Weak to</span>
          <select value={query.weakTo ?? ""} onchange={(e) => setQuery({ weakTo: e.currentTarget.value || null })}>
            <option value="">Any</option>
            {#each filters?.elements ?? [] as element (element)}<option value={element}>{element}</option>{/each}
          </select>
        </label>
        <label>
          <span>Region</span>
          <select value={query.region ?? ""} onchange={(e) => setQuery({ region: e.currentTarget.value || null })}>
            <option value="">Any</option>
            {#each filters?.regions ?? [] as region (region)}<option value={region}>{region}</option>{/each}
          </select>
        </label>
        <div class="levels">
          <span>Level</span>
          <input class="tabular" type="number" min="0" aria-label="Lowest level" value={query.levelMin ?? ""} oninput={(e) => setQuery({ levelMin: number(e.currentTarget.value) })} />
          <span class="to">to</span>
          <input class="tabular" type="number" min="0" aria-label="Highest level" value={query.levelMax ?? ""} oninput={(e) => setQuery({ levelMax: number(e.currentTarget.value) })} />
        </div>
        <label>
          <span>Sort</span>
          <select value={query.sort ?? ""} onchange={(e) => setQuery({ sort: e.currentTarget.value || null })}>
            {#each SORTS as sort (sort.id)}<option value={sort.id}>{sort.label}</option>{/each}
          </select>
        </label>
      </div>
    {/if}
  </div>

  {#if monsterView.id != null}
    <button class="back" class:pinned onclick={backFromMonster}><ArrowLeft size={12} />Back</button>
    <MonsterCard id={monsterView.id} {map} {level} {quest} {session} plain={pinned} />
  {:else}
    <div class="list" class:pinned>
      {#if !browsing}
        {#if here?.card.mobs.length}
          <Section title="On this map" count={here.card.mobs.length}>
            {#snippet aside()}<span class="place">{here?.card.map.name}</span>{/snippet}
            {@render onMap(here.card.mobs)}
          </Section>
        {/if}
        {#if near.length}
          <Section title="Near your level" count={near.length}>
            {#snippet aside()}<span class="tabular">Lv {level}</span>{/snippet}
            {@render rows(near, false)}
          </Section>
        {/if}
      {/if}
      {#if page}
        {#if page.closest}<p class="quiet">Nothing by that name. The closest:</p>{/if}
        {#if page.monsters.length}
          {#if !page.closest}
            <p class="count tabular">{page.total.toLocaleString()} {page.total === 1 ? "monster" : "monsters"}</p>
          {/if}
          {@render rows(page.monsters, true)}
          {#if page.monsters.length < page.total}
            <button class="back inline" onclick={more}>Show more</button>
          {/if}
        {:else}
          <p class="quiet">No monsters match.</p>
        {/if}
        <button class="credit" onclick={() => backend.openLink(OSMS_URL)}>{OSMS_CREDIT}</button>
      {:else if searching}
        <p class="quiet"><span class="spin"><LoaderCircle size={13} /></span> Searching…</p>
      {/if}
    </div>
  {/if}
{/if}

<style>
  .lead {
    margin: 0 0 10px;
    color: var(--muted);
    font-size: 12px;
  }

  /* A card pads itself; these sit beside cards, so they match it. A pinned window pads its body. */
  .controls {
    padding: 8px 12px 0;
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
    margin-bottom: 6px;
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
    font-size: 12px;
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
    font-size: 11px;
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

  .kinds {
    display: grid;
    grid-template-columns: repeat(4, auto);
    gap: 0;
    margin-bottom: 8px;
    border-bottom: 1px solid var(--border);
  }

  .kinds button {
    padding: 3px 2px 5px;
    border: 0;
    border-bottom: 2px solid transparent;
    background: transparent;
    color: var(--muted);
    font-size: 11.5px;
    cursor: pointer;
  }

  .kinds button:hover,
  .kinds button:focus-visible {
    color: var(--text);
    outline: none;
  }

  .kinds [aria-checked="true"] {
    border-bottom-color: var(--accent);
    color: var(--accent-strong);
  }

  .filters {
    display: grid;
    gap: 5px;
    margin-bottom: 8px;
    padding: 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    font-size: 11.5px;
  }

  .filters label,
  .levels {
    display: grid;
    grid-template-columns: 48px 1fr;
    align-items: center;
    gap: 6px;
    color: var(--muted);
  }

  .levels {
    grid-template-columns: 48px 1fr auto 1fr;
  }

  .to {
    color: var(--faint);
  }

  .filters select,
  .levels input {
    min-width: 0;
    height: 24px;
    padding: 0 4px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--text);
    font: inherit;
  }

  .count {
    margin: 10px 0 4px;
    color: var(--faint);
    font-size: 11px;
  }

  .place {
    display: block;
    max-width: 170px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .rows {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  /* One grid for the head and the rows, so the numbers line up in columns. */
  .row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 26px 46px 44px;
    column-gap: 6px;
    align-items: baseline;
    width: 100%;
    padding: 4px;
    border: 0;
    background: transparent;
    color: var(--muted);
    font-size: 11.5px;
    text-align: right;
  }

  button.row {
    cursor: pointer;
  }

  button.row:hover,
  button.row:focus-visible {
    background: var(--surface-3);
    outline: none;
  }

  .rows li {
    border-top: 1px solid var(--border);
  }

  .head {
    padding-block: 0 3px;
    color: var(--faint);
    font-size: 10px;
    font-weight: 500;
  }

  .head > :first-child {
    text-align: left;
  }

  .head button {
    padding: 0;
    border: 0;
    background: transparent;
    color: inherit;
    font-size: 10px;
    text-align: right;
    cursor: pointer;
  }

  .head button:hover,
  .head button:focus-visible {
    color: var(--text);
    outline: none;
  }

  .head button.on {
    color: var(--accent-strong);
  }

  .name {
    display: flex;
    align-items: baseline;
    gap: 3px;
    min-width: 0;
    color: var(--text);
    font-size: 12px;
    font-weight: 500;
    text-align: left;
  }

  .label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .boss {
    display: inline-grid;
    flex: none;
    align-self: center;
    color: var(--bad);
  }

  .tag {
    flex: none;
    padding: 0 4px;
    border-radius: var(--radius-sm);
    background: rgba(255, 255, 255, 0.06);
    color: var(--faint);
    font-size: 9.5px;
    font-weight: 400;
  }

  .note {
    grid-column: 1 / -1;
    overflow: hidden;
    color: var(--faint);
    font-size: 10.5px;
    text-align: left;
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
    font-size: 11.5px;
    cursor: pointer;
  }

  .back:hover,
  .back:focus-visible {
    color: var(--text);
    outline: none;
  }

  .back.inline,
  .back.pinned {
    margin-inline: 0;
  }

  .back.inline {
    margin-top: 6px;
  }

  .quiet {
    margin: 10px 0 6px;
    color: var(--faint);
    font-size: 11.5px;
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

  .credit {
    display: block;
    margin-top: 8px;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--faint);
    font-size: 10.5px;
    text-align: left;
    cursor: pointer;
  }

  .credit:hover,
  .credit:focus-visible {
    color: var(--muted);
    outline: none;
  }
</style>
