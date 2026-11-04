<script lang="ts">
  // The Items half of the Lookup tab: the last price check, and a search of every item in the map
  // data. Search needs nothing from the game, so it works with live reading off.
  import { ArrowLeft, LoaderCircle } from "@lucide/svelte";
  import { untrack } from "svelte";

  import Card from "./Card.svelte";
  import ItemCard from "./ItemCard.svelte";
  import FilterBox from "./lookup/FilterBox.svelte";
  import GearFinder from "./lookup/GearFinder.svelte";
  import KindTabs from "./lookup/KindTabs.svelte";
  import SearchBar from "./lookup/SearchBar.svelte";
  import Chips from "./lookup/Chips.svelte";
  import MapDataPanel from "./map/MapDataPanel.svelte";
  import { backend } from "$lib/api";
  import { JOBS, blankQuery, gearView, gives, itemSlot, itemView, lastLine, watchItems } from "$lib/items.svelte";
  import { OSMS_CREDIT, OSMS_URL, mapData, watchMapData } from "$lib/mapdata.svelte";
  import { hotkey } from "$lib/settings.svelte";
  import type { ItemFilters, ItemKind, ItemPage, ItemRecord, ItemTooltip, MapReading } from "$lib/types";

  let {
    tooltips,
    level = null,
    character = null,
    job = null,
    map = null,
    pinned = false,
  }: {
    tooltips: ItemTooltip[];
    level?: number | null;
    character?: string | null;
    /** The job the HUD shows. */
    job?: string | null;
    map?: MapReading | null;
    pinned?: boolean;
  } = $props();

  const KINDS: { id: ItemKind | null; label: string }[] = [
    { id: null, label: "All" },
    { id: "equip", label: "Equip" },
    { id: "use", label: "Use" },
    { id: "scroll", label: "Scrolls" },
    { id: "setup", label: "Set-up" },
    { id: "etc", label: "Etc" },
  ];
  const SORTS = ["Attack", "Magic", "STR", "DEX", "INT", "LUK", "Weapon Def.", "Magic Def."];
  /** Long enough to skip the keys between letters, short enough to feel immediate. */
  const SEARCH_DELAY = 120;

  watchItems();
  watchMapData();

  const ready = $derived(mapData.status?.ready ?? false);
  const query = $derived(itemView.query);

  let filters = $state<ItemFilters | null>(null);
  let showFilters = $state(false);
  let page = $state<ItemPage | null>(null);
  let searching = $state(false);
  let record = $state<ItemRecord | null>(null);

  const narrowed = $derived(
    [query.slot, query.class, query.levelMin, query.levelMax, query.sort, query.mine || null].filter((v) => v != null).length,
  );
  const browsing = $derived(query.text.trim() !== "" || query.kind != null || narrowed > 0);
  /** Equipment is shown for the character until they search by name or ask for everything. */
  const finding = $derived(query.kind === "equip" && gearView.forMe && query.text.trim() === "");
  const equipment = $derived(query.kind == null || query.kind === "equip");
  const slotted = $derived(equipment || query.kind === "scroll");
  const asked = $derived(JSON.stringify([query, level, character]));

  $effect(() => {
    void mapData.status?.dataUpdatedAt;
    if (ready) backend.itemFilters().then((f) => (filters = f));
  });

  // Follows the query alone: every live reading brings new objects with the same content.
  $effect(() => {
    const key = asked;
    if (!browsing || finding || !ready) {
      page = null;
      return;
    }
    searching = true;
    const timer = setTimeout(() => {
      untrack(() => backend.searchItems({ ...query, offset: 0 }, level, character))
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
    const id = itemView.id;
    record = null;
    if (id != null) {
      backend.itemRecord(id).then((found) => {
        if (itemView.id === id) record = found;
      });
    }
  });

  async function more() {
    if (!page) return;
    const key = asked;
    const next = await backend.searchItems({ ...query, offset: page.items.length }, level, character);
    if (key === asked && page) page.items.push(...next.items);
  }

  function setQuery(change: Partial<typeof query>) {
    itemView.query = { ...itemView.query, ...change, offset: 0 };
    itemView.id = null;
  }

  function clear() {
    itemView.query = blankQuery();
    itemView.id = null;
    showFilters = false;
  }

  const number = (value: string) => (value === "" ? null : Math.max(0, Math.floor(Number(value) || 0)));

  /** What a row says under the name. */
  function line(item: ItemRecord) {
    if (item.kind === "equip") {
      const parts = [item.weaponType ?? item.slot, item.reqLevel ? `Lv ${item.reqLevel}` : null, gives(item)];
      return parts.filter(Boolean).join(" · ");
    }
    if (item.scroll) return lastLine(item).replace("Success rate: ", "");
    return lastLine(item) || itemSlot(item);
  }

  const gender = (item: ItemRecord) => (item.gender ? item.gender[0].toUpperCase() : null);
</script>

{#if !ready}
  {#each tooltips as item, i}
    <ItemCard {item} title={i === 0 ? "Item" : "Compared item"} plain={pinned && i === 0} />
  {/each}
  <Card title="Item search" plain={pinned}>
    <p class="lead">Search every item once the map data is downloaded.</p>
    <MapDataPanel compact />
  </Card>
{:else}
  <div class="controls" class:pinned>
  <SearchBar
    value={query.text}
    placeholder="Search items"
    label="Search items"
    {narrowed}
    bind:filters={showFilters}
    filterable={!finding}
    clearable={browsing || itemView.id != null}
    oninput={(text) => setQuery({ text })}
    onclear={clear}
  />

  {#if itemView.id == null}
    <KindTabs label="Kind of item" options={KINDS} value={query.kind} onselect={(kind) => setQuery({ kind, slot: null })} />
    {#if query.kind === "equip" && !finding}
      <button class="for-me" onclick={() => (gearView.forMe = true)}>Show only gear for me</button>
      <Chips label="Slot" options={filters?.slots ?? []} value={query.slot} onselect={(slot) => setQuery({ slot })} />
    {/if}
  {/if}

  {#if showFilters && !finding}
    <FilterBox>
      {#if slotted}
        <label>
          <span>Slot</span>
          <select value={query.slot ?? ""} onchange={(e) => setQuery({ slot: e.currentTarget.value || null })}>
            <option value="">Any</option>
            {#each filters?.slots ?? [] as slot (slot)}<option value={slot}>{slot}</option>{/each}
            <optgroup label="Weapon type">
              {#each filters?.weapons ?? [] as weapon (weapon)}<option value={weapon}>{weapon}</option>{/each}
            </optgroup>
          </select>
        </label>
      {/if}
      {#if equipment}
        <label>
          <span>Job</span>
          <select value={query.class ?? ""} onchange={(e) => setQuery({ class: e.currentTarget.value || null })}>
            <option value="">Any</option>
            {#each JOBS as job (job)}<option value={job}>{job}</option>{/each}
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
            <option value="">Level</option>
            <option value="name">Name</option>
            {#each SORTS as label (label)}<option value={label}>Highest {label}</option>{/each}
          </select>
        </label>
        <label class="check">
          <input type="checkbox" checked={query.mine} onchange={(e) => setQuery({ mine: e.currentTarget.checked })} />
          <span>Only what my build can wear now</span>
        </label>
      {/if}
    </FilterBox>
  {/if}
  </div>

  {#if itemView.id != null}
    {#if browsing}
      <button class="back" class:pinned onclick={() => (itemView.id = null)}><ArrowLeft size={12} />Back to results</button>
    {/if}
    {#if record}
      <ItemCard item={null} {record} {map} plain={pinned} />
    {/if}
  {:else if finding}
    <div class="list" class:pinned>
      <GearFinder {level} {character} {job} slots={filters?.slots ?? []} slot={query.slot} onslot={(slot) => setQuery({ slot })} />
    </div>
  {:else if browsing}
    <div class="list" class:pinned>
    {#if page}
      {#if page.closest}<p class="quiet">Nothing by that name. The closest:</p>{/if}
      {#if page.items.length}
        <p class="count tabular">{page.total.toLocaleString()} {page.total === 1 ? "item" : "items"}</p>
        <ul class="results">
          {#each page.items as item (item.id)}
            <li>
              <button onclick={() => (itemView.id = item.id)}>
                <span class="row-name">{item.name}{#if gender(item)}<small>{gender(item)}</small>{/if}</span>
                <span class="row-line">{line(item)}</span>
              </button>
            </li>
          {/each}
        </ul>
        {#if page.items.length < page.total}
          <button class="back" onclick={more}>Show more</button>
        {/if}
      {:else}
        <p class="quiet">No items match.</p>
      {/if}
      <button class="credit" onclick={() => backend.openLink(OSMS_URL)}>{OSMS_CREDIT}</button>
    {:else if searching}
      <p class="quiet"><span class="spin"><LoaderCircle size={13} /></span> Searching…</p>
    {/if}
    </div>
  {:else}
    {#each tooltips as item, i}
      <ItemCard {item} {map} title={i === 0 ? "Item" : "Compared item"} plain={pinned && i === 0} />
    {:else}
      <p class="empty">
        Search above, or hover a piece of equipment in game and {hotkey("ReadItem") ? `press ${hotkey("ReadItem")}` : "use the Read item hotkey"} to check that copy.
      </p>
    {/each}
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
    padding: 10px 12px 0;
  }

  .list {
    padding: 0 12px 10px;
  }

  .controls.pinned,
  .list.pinned {
    padding: 0;
  }

  .for-me {
    margin: -2px 0 6px;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--accent-strong);
    font-size: 11.5px;
    cursor: pointer;
  }

  .for-me:hover,
  .for-me:focus-visible {
    text-decoration: underline;
    outline: none;
  }

  .count {
    margin: 0 0 4px;
    color: var(--faint);
    font-size: 11px;
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
    font-size: 12.5px;
    font-weight: 600;
  }

  .row-name small {
    margin-left: 5px;
    color: var(--faint);
    font-size: 10px;
    font-weight: 500;
  }

  .row-line {
    overflow: hidden;
    color: var(--muted);
    font-size: 11px;
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

  .list .back,
  .back.pinned {
    margin-inline: 0;
  }

  .quiet {
    margin: 0 0 6px;
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

  .empty {
    margin: 0;
    padding: 18px 10px;
    color: var(--faint);
    font-size: 12px;
    text-align: center;
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
