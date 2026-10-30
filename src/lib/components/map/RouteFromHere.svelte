<script lang="ts">
  // The way from the map the player is on to the one they are looking at.
  import { backend } from "$lib/api";
  import { display, travelKey } from "$lib/settings.svelte";
  import type { MapRef, MapRoute } from "$lib/types";
  import RouteSteps from "./RouteSteps.svelte";

  let { from, to, onnavigate }: { from: MapRef; to: MapRef; onnavigate: (id: number) => void } = $props();

  let route = $state<MapRoute | null>(null);
  let loading = $state(true);

  $effect(() => {
    const [start, end] = [from.id, to.id];
    void travelKey();
    loading = true;
    backend
      .mapRoute(start, end)
      .then((r) => {
        if (from.id === start && to.id === end) route = r;
      })
      .finally(() => (loading = false));
  });
</script>

{#if route}
  <RouteSteps {route} {onnavigate} />
{:else if !loading}
  <p class="note">
    {#if display.travel.enabled}
      No route from your map is known, even with the travel you turned on.
    {:else}
      No route through portals is known. Some trips need a ship, taxi or return scroll: turn on travel in Settings to
      count taxis and scrolls.
    {/if}
  </p>
{/if}

<style>
  .note {
    margin: 0;
    color: var(--faint);
    font-size: 11.5px;
    line-height: 1.4;
  }
</style>
