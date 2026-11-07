<script lang="ts">
  // "Route to…": the fewest portals from this map to another, as steps the player walks.
  import { Flag, X } from "@lucide/svelte";

  import GoButton from "$lib/components/GoButton.svelte";
  import { mapPlace } from "$lib/destination.svelte";
  import { backend } from "$lib/api";
  import { display, travelKey } from "$lib/settings.svelte";
  import type { MapRef, MapRoute } from "$lib/types";
  import MapPicker from "./MapPicker.svelte";
  import RouteSteps from "./RouteSteps.svelte";

  let {
    from,
    region = null,
    onnavigate,
  }: { from: MapRef; region?: string | null; onnavigate: (id: number) => void } = $props();

  let target = $state<MapRef | null>(null);
  let route = $state<MapRoute | null>(null);
  let loading = $state(false);

  // Recompute when the card moves to another map.
  $effect(() => {
    const start = from.id;
    const to = target;
    void travelKey();
    route = null;
    if (!to) return;
    loading = true;
    backend
      .mapRoute(start, to.id)
      .then((r) => {
        if (target === to && from.id === start) route = r;
      })
      .finally(() => (loading = false));
  });
</script>

{#if target}
  <div class="target">
    <Flag size={12} />
    <span class="to">To <strong>{target.name}</strong></span>
    <GoButton place={mapPlace(target)} />
    <button class="clear" title="Clear the route" onclick={() => (target = null)}><X size={12} /></button>
  </div>
  {#if route}
    <RouteSteps {route} {onnavigate} />
  {:else if !loading}
    <p class="note">
      {#if region === "Maple Island"}
        No portal leaves Maple Island. To reach Victoria Island, talk to Shanks in Southperry. That trip is one way, so
        finish what you want to do here first.
      {:else if display.travel.enabled}
        No route is known, even with the travel you turned on. Some trips need a ship or an NPC the data doesn't list.
      {:else}
        No route through portals is known. Some trips need a ship, taxi or return scroll: turn on travel in Settings to
        count taxis and scrolls.
      {/if}
    </p>
  {/if}
{:else}
  <MapPicker label="Route to a map" placeholder="Route to…" onpick={(map) => (target = map)} />
{/if}

<style>
  .target {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--muted);
    font-size: var(--text-sm);
  }

  .to {
    flex: 1;
    min-width: 0;
  }

  .to strong {
    color: var(--text);
    font-weight: 600;
  }

  .clear {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--faint);
    cursor: pointer;
  }

  .clear:hover {
    background: rgba(255, 255, 255, 0.07);
    color: var(--text);
  }

  .note {
    margin: 6px 0 0;
    font-size: var(--text-xs);
    color: var(--faint);
  }
</style>
