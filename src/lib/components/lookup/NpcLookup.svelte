<script lang="ts">
  // The NPCs half of the Lookup tab: who is on this map, and a search of every NPC in the data by
  // name, quest or map.
  import { ArrowLeft, LoaderCircle } from "@lucide/svelte";
  import { untrack } from "svelte";

  import Chips from "./Chips.svelte";
  import FilterBox from "./FilterBox.svelte";
  import KindTabs from "./KindTabs.svelte";
  import LookupRow from "./LookupRow.svelte";
  import NpcCard from "./NpcCard.svelte";
  import SearchBar from "./SearchBar.svelte";
  import Card from "../Card.svelte";
  import MapDataPanel from "../map/MapDataPanel.svelte";
  import { backend } from "$lib/api";
  import { OSMS_CREDIT, OSMS_URL, mapData, watchMapData } from "$lib/mapdata.svelte";
  import { blankNpcQuery, npcView } from "$lib/places.svelte";
  import type { MapMatch, MapReading, NpcPage, NpcQuery, NpcRow } from "$lib/types";

  let {
    here = null,
    map = null,
    pinned = false,
  }: { here?: MapMatch | null; map?: MapReading | null; pinned?: boolean } = $props();

  const KINDS: { id: string | null; label: string }[] = [
    { id: null, label: "All" },
    { id: "quests", label: "Gives quests" },
  ];
  /** Long enough to skip the keys between letters, short enough to feel immediate. */
  const SEARCH_DELAY = 120;

  watchMapData();

  const ready = $derived(mapData.status?.ready ?? false);
  const query = $derived(npcView.query);

  let regions = $state<string[]>([]);
  let areas = $state<string[]>([]);
  let showFilters = $state(false);
  let page = $state<NpcPage | null>(null);
  let searching = $state(false);

  const narrowed = $derived(query.region != null ? 1 : 0);
  const browsing = $derived(query.text.trim() !== "" || query.kind != null || query.area != null || narrowed > 0);
  const asked = $derived(JSON.stringify(query));

  $effect(() => {
    void mapData.status?.dataUpdatedAt;
    if (!ready) return;
    backend.mapRegions().then((r) => (regions = r));
    backend.mapAreas().then((a) => (areas = a));
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
      untrack(() => backend.searchNpcs({ ...query, offset: 0 }))
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
    const next = await backend.searchNpcs({ ...query, offset: page.npcs.length });
    if (key === asked && page) page.npcs.push(...next.npcs);
  }

  function setQuery(change: Partial<NpcQuery>) {
    npcView.query = { ...npcView.query, ...change, offset: 0 };
    npcView.id = null;
  }

  function clear() {
    setQuery(blankNpcQuery());
    showFilters = false;
  }

  const quests = (count: number) => (count ? `${count} quest${count === 1 ? "" : "s"}` : null);

  function line(row: NpcRow) {
    if (row.note) return row.note;
    if (!row.map) return "Not on any map";
    return row.maps > 1 ? `${row.map} and ${row.maps - 1} more` : row.map;
  }
</script>

{#if !ready}
  <Card title="NPC search" plain={pinned}>
    <p class="lead">Search every NPC once the map data is downloaded.</p>
    <MapDataPanel compact />
  </Card>
{:else}
  <div class="controls" class:pinned>
    <SearchBar
      value={query.text}
      placeholder="NPC, a quest they give, or a map"
      label="Search NPCs"
      {narrowed}
      bind:filters={showFilters}
      clearable={browsing || npcView.id != null}
      oninput={(text) => setQuery({ text })}
      onclear={clear}
    />
    {#if npcView.id == null}
      <KindTabs label="Kind of NPC" options={KINDS} value={query.kind} onselect={(kind) => setQuery({ kind })} />
      <Chips label="Area" options={areas} value={query.area} onselect={(area) => setQuery({ area })} />
    {/if}
    {#if showFilters}
      <FilterBox>
        <label>
          <span>Region</span>
          <select value={query.region ?? ""} onchange={(e) => setQuery({ region: e.currentTarget.value || null })}>
            <option value="">Any</option>
            {#each regions as region (region)}<option value={region}>{region}</option>{/each}
          </select>
        </label>
      </FilterBox>
    {/if}
  </div>

  {#if npcView.id != null}
    <button class="back" class:pinned onclick={() => (npcView.id = null)}><ArrowLeft size={12} />Back</button>
    <NpcCard id={npcView.id} {map} plain={pinned} />
  {:else}
    <div class="list" class:pinned>
      {#if !browsing && here?.card.npcs.length}
        <p class="heading">On this map</p>
        <ul class="rows">
          {#each here.card.npcs as npc (npc.id)}
            <li><LookupRow name={npc.name} aside={quests(npc.quests.length)} onclick={() => (npcView.id = npc.id)} /></li>
          {/each}
        </ul>
        <p class="heading">Everyone</p>
      {/if}
      {#if page}
        {#if page.closest}<p class="quiet">Nothing by that name. The closest:</p>{/if}
        {#if page.npcs.length}
          {#if !page.closest}<p class="count tabular">{page.total.toLocaleString()} {page.total === 1 ? "NPC" : "NPCs"}</p>{/if}
          <ul class="rows">
            {#each page.npcs as row (row.id)}
              <li><LookupRow name={row.name} line={line(row)} aside={quests(row.quests)} onclick={() => (npcView.id = row.id)} /></li>
            {/each}
          </ul>
          {#if page.npcs.length < page.total}<button class="back inline" onclick={more}>Show more</button>{/if}
        {:else}
          <p class="quiet">No NPCs match.</p>
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
    font-size: var(--text-sm);
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

  .heading {
    margin: 8px 0 3px;
    color: var(--muted);
    font-family: var(--font-display);
    font-size: var(--text-md);
  }

  .count {
    margin: 0 0 4px;
    color: var(--faint);
    font-size: var(--text-xs);
  }

  .rows {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .rows li + li {
    border-top: 1px solid var(--border);
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
