<script lang="ts">
  // The Lookup tab: items, monsters, maps, NPCs and crafting recipes, each searched from the map data. None needs
  // anything from the game, so all work with live reading off.
  import { untrack } from "svelte";

  import CraftingPanel from "./item/CraftingPanel.svelte";
  import ItemPanel from "./ItemPanel.svelte";
  import MapLookup from "./lookup/MapLookup.svelte";
  import NpcLookup from "./lookup/NpcLookup.svelte";
  import SkillLookup from "./lookup/SkillLookup.svelte";
  import MonsterPanel from "./MonsterPanel.svelte";
  import Segmented from "$lib/ui/Segmented.svelte";
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
    { id: "skills", label: "Skills" },
    { id: "crafting", label: "Crafting" },
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

<div class="modes" class:pinned>
  <Segmented label="What to look up" options={[...MODES]} value={lookup.mode} stretch onchange={(mode) => (lookup.mode = mode)} />
</div>

{#if lookup.mode === "monsters"}
  <MonsterPanel {here} {map} {level} {quest} {session} {pinned} />
{:else if lookup.mode === "maps"}
  <MapLookup {here} {pinned} />
{:else if lookup.mode === "npcs"}
  <NpcLookup {here} {map} {pinned} />
{:else if lookup.mode === "skills"}
  <SkillLookup {pinned} />
{:else if lookup.mode === "crafting"}
  <CraftingPanel {pinned} />
{:else}
  <ItemPanel {tooltips} {level} {character} {job} {map} {pinned} />
{/if}

<style>
  .modes {
    display: flex;
    margin: var(--space-3) var(--space-3) 0;
  }

  .modes.pinned {
    margin: 0 0 var(--space-2);
  }
</style>
