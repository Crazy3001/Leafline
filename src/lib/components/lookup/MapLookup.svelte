<script lang="ts">
  // The Maps half of the Lookup tab: the map the player is on, and a search of every map in the
  // data by name, area, monster or NPC, down to a map's full card.
  import GoButton from "$lib/components/GoButton.svelte";
  import { FilterBoxState } from "$lib/filterbox.svelte";
  import SaveSpot from "$lib/components/SaveSpot.svelte";
  import { mapPlace } from "$lib/destination.svelte";
  import { ArrowLeft, LoaderCircle } from "@lucide/svelte";
  import { untrack } from "svelte";

  import Chips from "./Chips.svelte";
  import FilterBox from "./FilterBox.svelte";
  import KindTabs from "./KindTabs.svelte";
  import LookupRow from "./LookupRow.svelte";
  import SearchBar from "./SearchBar.svelte";
  import Card from "../Card.svelte";
  import MapDataPanel from "../map/MapDataPanel.svelte";
  import MapDetailsView from "../map/MapDetailsView.svelte";
  import { backend } from "$lib/api";
  import { OSMS_CREDIT, OSMS_URL, mapData, watchMapData } from "$lib/mapdata.svelte";
  import { blankMapQuery, mapView } from "$lib/places.svelte";
  import type { MapDetails, MapMatch, MapPage, MapQuery, MapRow } from "$lib/types";

  let { here = null, pinned = false }: { here?: MapMatch | null; pinned?: boolean } = $props();

  const KINDS: { id: string | null; label: string }[] = [
    { id: null, label: "All" },
    { id: "field", label: "Hunting" },
    { id: "town", label: "Towns" },
  ];
  /** Long enough to skip the keys between letters, short enough to feel immediate. */
  const SEARCH_DELAY = 120;

  watchMapData();

  const ready = $derived(mapData.status?.ready ?? false);
  const query = $derived(mapView.query);

  let regions = $state<string[]>([]);
  let areas = $state<string[]>([]);
  const filterBox = new FilterBoxState("maps");
  let page = $state<MapPage | null>(null);
  let searching = $state(false);
  let details = $state<MapDetails | null>(null);

  const narrowed = $derived([query.region, query.levelMin, query.levelMax].filter((v) => v != null).length);
  const browsing = $derived(query.text.trim() !== "" || query.kind != null || query.area != null || query.sort != null || narrowed > 0);
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
      untrack(() => backend.browseMaps({ ...query, offset: 0 }))
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
    const id = mapView.id;
    details = null;
    if (id != null) {
      backend.mapCard(id).then((card) => {
        if (mapView.id === id) details = card;
      });
    }
  });

  async function more() {
    if (!page) return;
    const key = asked;
    const next = await backend.browseMaps({ ...query, offset: page.maps.length });
    if (key === asked && page) page.maps.push(...next.maps);
  }

  function setQuery(change: Partial<MapQuery>) {
    mapView.query = { ...mapView.query, ...change, offset: 0 };
    mapView.id = null;
    mapView.trail = [];
  }

  function clear() {
    setQuery(blankMapQuery());
  }

  function open(id: number) {
    if (mapView.id != null && mapView.id !== id) mapView.trail.push(mapView.id);
    mapView.id = id;
  }

  const back = () => (mapView.id = mapView.trail.pop() ?? null);
  const number = (value: string) => (value === "" ? null : Math.max(0, Math.floor(Number(value) || 0)));
  const levels = (range: [number, number] | null) => (range ? (range[0] === range[1] ? `Lv ${range[0]}` : `Lv ${range[0]}–${range[1]}`) : null);

  function line(row: MapRow) {
    return row.note ?? [row.map.street !== row.map.name ? row.map.street : null, row.isTown ? "Town" : null].filter(Boolean).join(" · ");
  }
</script>

{#if !ready}
  <Card title="Map search" plain={pinned}>
    <p class="lead">Search every map once the map data is downloaded.</p>
    <MapDataPanel compact />
  </Card>
{:else}
  <div class="controls" class:pinned>
    <SearchBar
      value={query.text}
      placeholder="Map, area, monster or NPC"
      label="Search maps"
      {narrowed}
      bind:filters={filterBox.open}
      clearable={browsing || mapView.id != null}
      oninput={(text) => setQuery({ text })}
      onclear={clear}
    />
    {#if mapView.id == null}
      <KindTabs label="Kind of map" options={KINDS} value={query.kind} onselect={(kind) => setQuery({ kind })} />
      <Chips label="Area" options={areas} value={query.area} onselect={(area) => setQuery({ area })} />
    {/if}
    {#if filterBox.open}
      <FilterBox>
        <label>
          <span>Region</span>
          <select value={query.region ?? ""} onchange={(e) => setQuery({ region: e.currentTarget.value || null })}>
            <option value="">Any</option>
            {#each regions as region (region)}<option value={region}>{region}</option>{/each}
          </select>
        </label>
        <div class="levels">
          <span>Monsters</span>
          <input class="tabular" type="number" min="0" placeholder="any" aria-label="Lowest monster level" value={query.levelMin ?? ""} oninput={(e) => setQuery({ levelMin: number(e.currentTarget.value) })} />
          <span class="to">to</span>
          <input class="tabular" type="number" min="0" placeholder="any" aria-label="Highest monster level" value={query.levelMax ?? ""} oninput={(e) => setQuery({ levelMax: number(e.currentTarget.value) })} />
        </div>
        <label>
          <span>Sort</span>
          <select value={query.sort ?? ""} onchange={(e) => setQuery({ sort: e.currentTarget.value || null })}>
            <option value="">Name</option>
            <option value="level">Lowest monsters</option>
          </select>
        </label>
      </FilterBox>
    {/if}
  </div>

  {#if mapView.id != null}
    <button class="back" class:pinned onclick={back}><ArrowLeft size={12} />Back</button>
    {#if details}
      <Card title="Map" plain={pinned}>
        {#if details.map.street}<p class="street">{details.map.street}</p>{/if}
        <h3 class="name">{details.map.name} <GoButton place={mapPlace(details.map)} /></h3>
        <p class="save-row"><SaveSpot mapId={details.map.id} name={details.map.name} text /></p>
        {#key details.map.id}
          <MapDetailsView {details} current={here?.card.map ?? null} variant="compact" onnavigate={open} />
        {/key}
      </Card>
    {/if}
  {:else}
    <div class="list" class:pinned>
      {#if !browsing && here}
        <p class="heading">You're here</p>
        <ul class="rows">
          <li>
            <LookupRow name={here.card.map.name} picture={{ kind: "mark", id: here.card.map.id }} line={here.card.map.street} aside={levels(here.card.levelRange)} onclick={() => open(here!.card.map.id)} />
          </li>
        </ul>
        <p class="heading">Every map</p>
      {/if}
      {#if page}
        {#if page.closest}<p class="quiet">Nothing by that name. The closest:</p>{/if}
        {#if page.maps.length}
          {#if !page.closest}<p class="count tabular">{page.total.toLocaleString()} {page.total === 1 ? "map" : "maps"}</p>{/if}
          <ul class="rows">
            {#each page.maps as row (row.map.id)}
              <li><LookupRow name={row.map.name} picture={{ kind: "mark", id: row.map.id }} line={line(row)} aside={levels(row.levelRange)} onclick={() => open(row.map.id)} /></li>
            {/each}
          </ul>
          {#if page.maps.length < page.total}<button class="back inline" onclick={more}>Show more</button>{/if}
        {:else}
          <p class="quiet">No maps match.</p>
        {/if}
        <button class="credit" onclick={() => backend.openLink(OSMS_URL)}>{OSMS_CREDIT}</button>
      {:else if searching}
        <p class="quiet"><span class="spin"><LoaderCircle size={13} /></span> Searching…</p>
      {/if}
    </div>
  {/if}
{/if}

<style>
  .save-row {
    margin: 2px 0 8px;
  }

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

  .street {
    margin: 0;
    color: var(--muted);
    font-size: var(--text-sm);
  }

  .name {
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--text-lg);
    font-weight: 600;
    line-height: 1.25;
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
