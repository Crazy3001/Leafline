<script lang="ts">
  // The Lookup tab: items, monsters, maps and NPCs, each searched from the map data. None needs
  // anything from the game, so all work with live reading off.
  import { untrack } from "svelte";

  import ItemPanel from "./ItemPanel.svelte";
  import MapLookup from "./lookup/MapLookup.svelte";
  import NpcLookup from "./lookup/NpcLookup.svelte";
  import MonsterPanel from "./MonsterPanel.svelte";
  import { backend } from "$lib/api";
  import { lookup } from "$lib/items.svelte";
  import { mapData, watchMapData } from "$lib/mapdata.svelte";
  import { watchMonsters } from "$lib/monsters.svelte";
  import { watchPlaces } from "$lib/places.svelte";
  import type { ItemTooltip, MapMatch, MapReading, QuestReading, SessionSummary } from "$lib/types";

  let {
    tooltips,
    level = null,
    character = null,
    job = null,
    map = null,
    quest = null,
    session = null,
    pinned = false,
  }: {
    tooltips: ItemTooltip[];
    level?: number | null;
    character?: string | null;
    /** The job the HUD shows. */
    job?: string | null;
    map?: MapReading | null;
    quest?: QuestReading | null;
    session?: SessionSummary | null;
    pinned?: boolean;
  } = $props();

  const MODES = [
    { id: "items", label: "Items" },
    { id: "monsters", label: "Monsters" },
    { id: "maps", label: "Maps" },
    { id: "npcs", label: "NPCs" },
  ] as const;

  watchMonsters();
  watchPlaces();
  watchMapData();

  /** The map the minimap header matched. */
  let here = $state<MapMatch | null>(null);

  const ready = $derived(mapData.status?.ready ?? false);
  const mapName = $derived(map?.name ?? null);
  const mapTwin = $derived(map?.twin ?? null);

  // Follows the text alone: every live reading brings new objects with the same content.
  $effect(() => {
    const name = mapName;
    const twin = mapTwin;
    here = null;
    if (!name || !ready) return;
    backend.findMap(name, untrack(() => map?.street ?? null), twin).then((match) => {
      if (mapName === name && mapTwin === twin) here = match;
    });
  });
</script>

<div class="modes" class:pinned role="radiogroup" aria-label="What to look up">
  {#each MODES as mode (mode.id)}
    <button role="radio" aria-checked={lookup.mode === mode.id} onclick={() => (lookup.mode = mode.id)}>{mode.label}</button>
  {/each}
</div>

{#if lookup.mode === "monsters"}
  <MonsterPanel {here} {map} {level} {quest} {session} {pinned} />
{:else if lookup.mode === "maps"}
  <MapLookup {here} {pinned} />
{:else if lookup.mode === "npcs"}
  <NpcLookup {here} {map} {pinned} />
{:else}
  <ItemPanel {tooltips} {level} {character} {job} {map} {pinned} />
{/if}

<style>
  .modes {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 0;
    margin: 10px 12px 0;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
  }

  .modes.pinned {
    margin: 0 0 8px;
  }

  .modes button {
    height: 26px;
    padding: 0 8px;
    border: 0;
    background: transparent;
    color: var(--muted);
    font-size: 12px;
    cursor: pointer;
  }

  .modes button + button {
    border-left: 1px solid var(--border-strong);
  }

  .modes button:hover,
  .modes button:focus-visible {
    color: var(--text);
    outline: none;
  }

  .modes [aria-checked="true"] {
    background: var(--accent-soft);
    color: var(--accent-strong);
    font-weight: 600;
  }
</style>
