<script lang="ts">
  // The map a quest step happens on, with the way there from where the character is.
  import { Route } from "@lucide/svelte";

  import RouteSteps from "$lib/components/map/RouteSteps.svelte";
  import { backend } from "$lib/api";
  import { travelKey } from "$lib/settings.svelte";
  import type { MapRef, MapRoute } from "$lib/types";

  let { maps, from }: { maps: MapRef[]; from: MapRef | null } = $props();

  let open = $state(false);
  let route = $state<MapRoute | null>(null);
  let routing = $state(false);

  const target = $derived(maps[0]?.id ?? null);
  const start = $derived(from?.id ?? null);
  const here = $derived(start != null && maps.some((m) => m.id === start));

  $effect(() => {
    const [to, source] = [target, start];
    void travelKey();
    route = null;
    if (!open || to == null || source == null || to === source) return;
    routing = true;
    backend
      .mapRoute(source, to)
      .then((found) => {
        if (target === to && start === source) route = found;
      })
      .finally(() => (routing = false));
  });
</script>

{#if maps.length}
  <p class="place">
    <span>{maps.map((m) => m.name).join(", ")}</span>
    {#if here}
      <span class="here">you're here</span>
    {:else if from}
      <button aria-expanded={open} onclick={() => (open = !open)}><Route size={11} />{open ? "Hide route" : "Route"}</button>
    {/if}
  </p>
  {#if open && !here}
    {#if route}
      <div class="route"><RouteSteps {route} /></div>
    {:else if !routing}
      <p class="none">No route through portals is known from {from?.name}.</p>
    {/if}
  {/if}
{/if}

<style>
  .place {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 2px 8px;
    margin: 2px 0 0;
    color: var(--muted);
    font-size: var(--text-sm);
  }

  .here {
    color: var(--good);
    font-size: var(--text-xs);
  }

  button {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--accent);
    font-size: var(--text-xs);
    cursor: pointer;
  }

  button:hover,
  button:focus-visible {
    color: var(--accent-strong);
    outline: none;
  }

  .route {
    margin-top: 5px;
  }

  .none {
    margin: 4px 0 0;
    color: var(--faint);
    font-size: var(--text-xs);
  }
</style>
