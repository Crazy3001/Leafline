<script lang="ts">
  // Tools → Route planner: the fewest portals from one map to another. It starts from the map
  // you're on; you can start from any other. "Go" follows the route in the Go here bar.
  import { MapPin } from "@lucide/svelte";
  import { untrack } from "svelte";

  import MapDataPanel from "$lib/components/map/MapDataPanel.svelte";
  import MapPicker from "$lib/components/map/MapPicker.svelte";
  import RoutePlanner from "$lib/components/map/RoutePlanner.svelte";
  import { backend } from "$lib/api";
  import { mapData, watchMapData } from "$lib/mapdata.svelte";
  import { live } from "$lib/readings.svelte";
  import type { MapRef } from "$lib/types";

  watchMapData();

  const ready = $derived(mapData.status?.ready ?? false);
  /** The map the minimap shows, matched in the data. */
  let here = $state<{ map: MapRef; region: string | null } | null>(null);
  /** Another start the player picked. */
  let picked = $state<MapRef | null>(null);

  const name = $derived(live.analysis?.map?.name ?? null);
  const twin = $derived(live.analysis?.map?.twin ?? null);
  $effect(() => {
    const asked = name;
    const askedTwin = twin;
    if (!asked || !ready) return;
    backend
      .findMap(asked, untrack(() => live.analysis?.map?.street ?? null), askedTwin)
      .then((match) => {
        if (match && name === asked) here = { map: match.card.map, region: match.card.region ?? null };
      })
      .catch(() => {});
  });

  const from = $derived(picked ?? here?.map ?? null);
</script>

<section class="card page">
  {#if !ready}
    {#if mapData.status}<MapDataPanel compact />{/if}
  {:else}
    <div class="from">
      <MapPin size={14} />
      {#if from}
        <span>From <strong>{from.name}</strong>{#if !picked && here}<span class="quiet"> · where you are</span>{/if}</span>
        {#if picked && here}<button class="link" onclick={() => (picked = null)}>From where you are</button>{/if}
      {:else}
        <span class="quiet">The minimap isn't in view. Pick where you start.</span>
      {/if}
    </div>
    <MapPicker label="Start from another map" placeholder={from ? "Start somewhere else…" : "Where you start…"} onpick={(m) => (picked = m)} />
    {#if from}
      <div class="plan">
        {#key from.id}
          <RoutePlanner {from} region={picked ? null : (here?.region ?? null)} />
        {/key}
      </div>
    {/if}
  {/if}
</section>

<style>
  .page {
    display: grid;
    gap: 10px;
    margin: 0 10px 10px;
    padding: 12px 14px 14px;
  }

  .from {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px 8px;
    color: var(--map);
  }

  .from span {
    color: var(--text);
  }

  .from strong {
    color: var(--ref-map);
    font-weight: 600;
  }

  .quiet,
  .from .quiet {
    color: var(--faint);
    font-size: var(--text-sm);
  }

  .link {
    margin-left: auto;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--accent-text);
    font: inherit;
    font-size: var(--text-sm);
    font-weight: 600;
    cursor: pointer;
  }

  .plan {
    padding-top: 10px;
    border-top: 1px solid var(--border);
  }
</style>
