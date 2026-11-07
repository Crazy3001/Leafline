<script lang="ts">
  // "Go to…": a search over maps and NPCs that heads for the one picked, with the places headed
  // for lately beside it for a second trip.
  import { Navigation, X } from "@lucide/svelte";

  import { backend } from "$lib/api";
  import { forgetPlace, goTo, isHeadingFor, mapPlace, nav, npcPlace, watchDestination } from "$lib/destination.svelte";
  import type { Destination, MapRef, NpcRow } from "$lib/types";

  watchDestination();

  let text = $state("");
  let maps = $state<MapRef[]>([]);
  let npcs = $state<NpcRow[]>([]);
  let active = $state(0);
  let open = $state(false);
  let error = $state<string | null>(null);
  let asked = 0;

  /** Results in the order shown: maps, then NPCs. */
  const results = $derived([
    ...maps.map((m) => ({ kind: "map" as const, key: `m${m.id}`, map: m })),
    ...npcs.map((n) => ({ kind: "npc" as const, key: `n${n.id}`, npc: n })),
  ]);
  const recent = $derived(nav.recent.filter((d) => !isHeadingFor(d)).slice(0, 4));

  async function search() {
    const ask = ++asked;
    const query = text.trim();
    if (!query) {
      maps = [];
      npcs = [];
      return;
    }
    const [foundMaps, foundNpcs] = await Promise.all([
      backend.searchMaps(query).catch(() => []),
      backend.searchNpcs({ text: query, kind: null, region: null, area: null, offset: 0 }).catch(() => null),
    ]);
    if (ask !== asked) return;
    maps = foundMaps.slice(0, 5);
    npcs = foundNpcs && !foundNpcs.closest ? foundNpcs.npcs.slice(0, 4) : [];
    active = 0;
    open = true;
  }

  async function pick(result: (typeof results)[number]) {
    text = "";
    maps = [];
    npcs = [];
    open = false;
    error = null;
    if (result.kind === "map") return goTo(mapPlace(result.map));
    // An NPC search row names one map; the sheet has every map they stand on.
    const where = await backend.npcSheet(result.npc.id, null, null, null).catch(() => null);
    const place = where && npcPlace(where.sheet.name, where.sheet.maps.map((m) => m.map));
    if (place) await goTo(place);
    else error = `${result.npc.name} isn't on any map in the data.`;
  }

  function onkeydown(event: KeyboardEvent) {
    if (!results.length) return;
    if (event.key === "ArrowDown") active = (active + 1) % results.length;
    else if (event.key === "ArrowUp") active = (active - 1 + results.length) % results.length;
    else if (event.key === "Enter") void pick(results[active]);
    else if (event.key === "Escape") open = false;
    else return;
    event.preventDefault();
  }

  const where = (d: Destination) => (d.kind === "npc" ? d.maps[0]?.name : d.maps[0]?.street) ?? null;
</script>

<div class="goto">
  <label class="field">
    <Navigation size={12} />
    <input
      type="search"
      aria-label="Go to a map or NPC"
      placeholder="Go to a map or NPC…"
      bind:value={text}
      oninput={search}
      {onkeydown}
      onfocus={() => (open = true)}
      onblur={() => setTimeout(() => (open = false), 120)}
    />
  </label>
  {#if open && text.trim()}
    <ul class="results" role="listbox" aria-label="Places to go">
      {#each results as result, i (result.key)}
        <li role="option" aria-selected={i === active}>
          <button onmousedown={(e) => e.preventDefault()} onclick={() => pick(result)} class:active={i === active}>
            {#if result.kind === "map"}
              <span class="name map">{result.map.name}</span>
              {#if result.map.street}<span class="sub">{result.map.street}</span>{/if}
            {:else}
              <span class="name npc">{result.npc.name}</span>
              {#if result.npc.map}<span class="sub">{result.npc.map}{result.npc.maps > 1 ? ` and ${result.npc.maps - 1} more` : ""}</span>{/if}
            {/if}
          </button>
        </li>
      {:else}
        <li class="empty">No map or NPC by that name.</li>
      {/each}
    </ul>
  {/if}
  {#if error}<p class="error">{error}</p>{/if}
  {#if recent.length}
    <ul class="recent" aria-label="Places headed for lately">
      {#each recent as place (place.kind + place.name + place.maps[0]?.id)}
        <li>
          <button class="ref {place.kind === 'npc' ? 'ref-npc' : 'ref-map'}" onclick={() => goTo(place)}>{place.name}</button>
          {#if where(place)}<span class="sub">{where(place)}</span>{/if}
          <button class="forget" aria-label="Forget {place.name}" onclick={() => forgetPlace(place)}><X size={10} /></button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .goto {
    position: relative;
  }

  .field {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 26px;
    padding: 0 8px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--accent);
  }

  .field:focus-within {
    border-color: var(--accent);
  }

  input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: 0;
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
  }

  input::-webkit-search-cancel-button {
    filter: invert(0.6);
  }

  .results {
    position: absolute;
    z-index: 5;
    top: 30px;
    left: 0;
    right: 0;
    max-height: 240px;
    margin: 0;
    padding: 4px;
    overflow-y: auto;
    list-style: none;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface-3);
    box-shadow: var(--shadow);
  }

  .results button {
    display: grid;
    width: 100%;
    padding: 4px 8px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    text-align: left;
    cursor: pointer;
  }

  .results button.active,
  .results button:hover {
    background: rgba(255, 255, 255, 0.06);
  }

  .name {
    font-size: var(--text-sm);
  }

  .name.map {
    color: var(--ref-map);
  }

  .name.npc {
    color: var(--ref-npc);
  }

  .sub {
    color: var(--faint);
    font-size: var(--text-xs);
  }

  .empty {
    padding: 6px 8px;
    color: var(--faint);
    font-size: var(--text-sm);
  }

  .error {
    margin: 4px 0 0;
    color: var(--warn);
    font-size: var(--text-xs);
  }

  .recent {
    display: flex;
    flex-wrap: wrap;
    gap: 2px 12px;
    margin: 5px 0 0;
    padding: 0;
    list-style: none;
    font-size: var(--text-sm);
  }

  .recent li {
    display: inline-flex;
    align-items: baseline;
    gap: 4px;
    min-width: 0;
  }

  .forget {
    display: inline-grid;
    align-self: center;
    place-items: center;
    width: 14px;
    height: 14px;
    padding: 0;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--faint);
    cursor: pointer;
    opacity: 0;
  }

  .recent li:hover .forget,
  .forget:focus-visible {
    opacity: 1;
  }

  .forget:hover {
    color: var(--text);
  }
</style>
