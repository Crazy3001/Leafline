<script lang="ts">
  // The Monsters half of the Lookup tab: what is on this map and near the character's level, and
  // a search of every monster in the map data by name, drop or map.
  import { clipped, tip } from "$lib/tip.svelte";
  import { FilterBoxState } from "$lib/filterbox.svelte";
  import { LoaderCircle, Skull } from "@lucide/svelte";
  import { untrack } from "svelte";

  import Card from "./Card.svelte";
  import Chips from "./lookup/Chips.svelte";
  import FilterBox from "./lookup/FilterBox.svelte";
  import KindTabs from "./lookup/KindTabs.svelte";
  import SearchBar from "./lookup/SearchBar.svelte";
  import MapDataPanel from "./map/MapDataPanel.svelte";
  import MonsterCard from "./MonsterCard.svelte";
  import LookupSplit from "./lookup/LookupSplit.svelte";
  import Section from "./map/Section.svelte";
  import { backend } from "$lib/api";
  import Pic from "$lib/ui/Pic.svelte";
  import { formatCompact, formatNumber, monsterLabel } from "$lib/format";
  import { mapData, watchMapData } from "$lib/mapdata.svelte";
  import { backFromMonster, blankMonsterQuery, monsterView, openMonster } from "$lib/monsters.svelte";
  import type { MapMatch, MapReading, MonsterFilters, MonsterPage, MonsterQuery, MonsterRow, MobRow, QuestReading, SessionSummary } from "$lib/types";

  let {
    here = null,
    map = null,
    level = null,
    quest = null,
    session = null,
    pinned = false,
  }: {
    /** The map the minimap header matched. */
    here?: MapMatch | null;
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
  let areas = $state<string[]>([]);
  const filterBox = new FilterBoxState("monsters", true);
  let page = $state<MonsterPage | null>(null);
  let searching = $state(false);
  let near = $state<MonsterRow[]>([]);

  const narrowed = $derived([query.weakTo, query.region, query.levelMin, query.levelMax, query.sort].filter((v) => v != null).length);
  const browsing = $derived(query.text.trim() !== "" || query.kind != null || query.area != null || narrowed > 0);
  const asked = $derived(JSON.stringify(query));

  $effect(() => {
    void mapData.status?.dataUpdatedAt;
    if (!ready) return;
    backend.monsterFilters().then((f) => (filters = f));
    backend.mapAreas().then((a) => (areas = a));
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
  }

  /** The column it sorts by, with Level as the way back. */
  function sortBy(sort: string | null) {
    setQuery({ sort: query.sort === sort ? null : sort });
  }

  const number = (value: string) => (value === "" ? null : Math.max(0, Math.floor(Number(value) || 0)));
</script>

{#snippet name(mob: { id: number; name: string; boss: boolean; undead: boolean })}
  <span class="name">
    <Pic kind="mob" id={mob.id} size={20} />
    <span use:clipped class="label">{monsterLabel(mob.name)}</span>
    {#if mob.boss}<span class="boss" use:tip={"Boss"}><Skull size={11} /></span>{/if}
    {#if mob.undead}<span class="badge tag">undead</span>{/if}
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
      <button class:on={query.sort == null} onclick={() => sortBy(null)}>Lv</button>
      <button class:on={query.sort === "hp"} onclick={() => sortBy("hp")}>HP</button>
      <button class:on={query.sort === "exp"} onclick={() => sortBy("exp")}>EXP</button>
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
        <button class="row nav-row" class:selected={monsterView.id === mob.id} aria-current={monsterView.id === mob.id} onclick={() => openMonster(mob.id)}>
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
        <button class="row nav-row" class:selected={monsterView.id === mob.id} aria-current={monsterView.id === mob.id} onclick={() => openMonster(mob.id)}>
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
    <SearchBar
      value={query.text}
      placeholder="Monster, what it drops, or a map"
      label="Search monsters"
      {narrowed}
      bind:filters={filterBox.open}
      clearable={browsing || monsterView.id != null}
      oninput={(text) => setQuery({ text })}
      onclear={clear}
    />

    {#if monsterView.id == null}
      <KindTabs label="Kind of monster" options={KINDS} value={query.kind} onselect={(kind) => setQuery({ kind })} />
      <Chips label="Area" options={areas} value={query.area} onselect={(area) => setQuery({ area })} />
    {/if}

    {#if filterBox.open}
      <FilterBox>
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
          <input class="tabular" type="number" min="0" placeholder="any" aria-label="Lowest level" value={query.levelMin ?? ""} oninput={(e) => setQuery({ levelMin: number(e.currentTarget.value) })} />
          <span class="to">to</span>
          <input class="tabular" type="number" min="0" placeholder="any" aria-label="Highest level" value={query.levelMax ?? ""} oninput={(e) => setQuery({ levelMax: number(e.currentTarget.value) })} />
        </div>
        <label>
          <span>Sort</span>
          <select value={query.sort ?? ""} onchange={(e) => setQuery({ sort: e.currentTarget.value || null })}>
            {#each SORTS as sort (sort.id)}<option value={sort.id}>{sort.label}</option>{/each}
          </select>
        </label>
      </FilterBox>
    {/if}
  </div>

  <LookupSplit open={monsterView.id != null} onback={backFromMonster} {pinned}>
    {#snippet detail()}
      {#if monsterView.id != null}<MonsterCard id={monsterView.id} {map} {level} {quest} {session} plain={pinned} />{/if}
    {/snippet}
    {#snippet empty()}<p class="pick">Pick a monster to see it here.</p>{/snippet}
    {#snippet list()}
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
      {:else if searching}
        <p class="quiet"><span class="spin"><LoaderCircle size={13} /></span> Searching…</p>
      {/if}
    </div>
    {/snippet}
  </LookupSplit>
{/if}

<style>
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

  .controls.pinned,
  .list.pinned {
    padding: 0;
  }

  .count {
    margin: var(--space-2) 0 var(--space-1);
    color: var(--faint);
    font-size: var(--text-xs);
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
    column-gap: var(--space-1-5);
    align-items: baseline;
    width: 100%;
    padding: var(--space-1) var(--space-6) var(--space-1) var(--space-1);
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--muted);
    font-size: var(--text-md);
    text-align: right;
  }

  .row.selected {
    background: var(--accent-soft);
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
    padding-block: 0 var(--space-0-5);
    color: var(--faint);
    font-size: var(--text-xs);
    font-weight: var(--weight-body);
  }

  .head > :first-child {
    text-align: left;
  }

  .head button {
    padding: 0;
    border: 0;
    background: transparent;
    color: inherit;
    font-size: var(--text-xs);
    text-align: right;
    cursor: pointer;
  }

  .head button:hover,
  .head button:focus-visible {
    color: var(--text);
    outline: none;
  }

  .head button.on {
    color: var(--accent-text-strong);
  }

  .name {
    display: flex;
    align-items: center;
    gap: var(--space-1-5);
    min-width: 0;
    color: var(--text);
    font-size: var(--text-md);
    font-weight: var(--weight-body);
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

  .note {
    grid-column: 1 / -1;
    overflow: hidden;
    color: var(--faint);
    font-size: var(--text-xs);
    text-align: left;
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

  .back:hover,
  .back:focus-visible {
    color: var(--text);
    outline: none;
  }

  .back.inline {
    margin-inline: 0;
  }

  .back.inline {
    margin-top: var(--space-1-5);
  }

  .quiet {
    margin: var(--space-2) 0 var(--space-1-5);
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

  .pick {
    margin: 0;
    padding: var(--space-8) var(--space-3);
    color: var(--faint);
    font-size: var(--text-md);
    text-align: center;
  }
</style>
