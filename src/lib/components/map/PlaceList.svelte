<script lang="ts">
  // The maps something can be found on, the best first, and the route to the one picked.
  import { backend } from "$lib/api";
  import RouteSteps from "./RouteSteps.svelte";
  import { formatCompact, groundDistance } from "$lib/format";
  import { travelKey } from "$lib/settings.svelte";
  import type { MapRef, MapRoute, Pace, Shortcut } from "$lib/types";

  /** A map, how far it is, and for a monster, how many live there. */
  interface Place {
    map: MapRef;
    hops: number | null;
    travel: Shortcut | null;
    count?: number;
    mapTotal?: number;
    pace?: Pace | null;
  }

  let { places, from }: { places: Place[]; from: MapRef | null } = $props();

  /** Enough to choose from without a long list pushing the drops off the panel. */
  const SHOWN = 5;

  let all = $state(false);
  /** The closest maps first, instead of the ones with the most. */
  let nearest = $state(false);
  /** The map whose route is shown: the best one until another is picked. */
  let picked = $state<number | null>(null);
  let route = $state<MapRoute | null>(null);
  let routing = $state(false);

  const steps = (m: Place) => Math.min(m.hops ?? Infinity, m.travel?.hops ?? Infinity);
  const ordered = $derived(nearest ? [...places].sort((a, b) => steps(a) - steps(b)) : places);
  const counted = $derived(places.some((m) => m.count != null));
  const shown = $derived(all ? ordered : ordered.slice(0, SHOWN));
  const target = $derived(picked ?? ordered[0]?.map.id ?? null);
  const ground = $derived(places.find((m) => m.map.id === target) ?? null);
  const fromId = $derived(from?.id);

  $effect(() => {
    const to = target;
    const start = fromId;
    void travelKey();
    route = null;
    if (to == null || start == null) return;
    routing = true;
    backend
      .mapRoute(start, to)
      .then((r) => {
        if (target === to && fromId === start) route = r;
      })
      .finally(() => (routing = false));
  });

  const share = (m: Place) => (m.mapTotal! > m.count! ? `of ${m.mapTotal} on the map` : "the only monster on the map");
</script>

{#if places.length}
  <div class="legend">
    <span>Map</span>
    {#if counted}<span title="How many of it spawn, out of every monster on the map">It / all</span>{/if}
    {#if from && places.length > 1}
      <button class:on={nearest} title={nearest ? "Show the best maps first" : "Show the closest maps first"} aria-pressed={nearest} onclick={() => (nearest = !nearest)}>Nearest</button>
    {/if}
  </div>
  <ul class="maps">
    {#each shown as map (map.map.id)}
      <li>
        <button class="ground" class:selected={map.map.id === target} aria-pressed={map.map.id === target} onclick={() => (picked = map.map.id)}>
          <span class="name">{map.map.name}</span>
          {#if map.count != null}
            <span class="count tabular" title="{map.count} spawn, {share(map)}">{map.count}<small>/{map.mapTotal}</small></span>
          {/if}
          <span
            class="dist tabular"
            class:shortcut={map.travel}
            title={map.travel ? `With the travel you turned on in Settings, via ${map.travel.via.name}` : undefined}>{groundDistance(map, from != null)}</span
          >
        </button>
      </li>
    {/each}
  </ul>
  {#if places.length > SHOWN}
    <button class="more" onclick={() => (all = !all)}>{all ? "Show fewer" : `Show all ${places.length}`}</button>
  {/if}

  {#if ground?.pace}
    <p class="note" title="From the EXP you measured here, assuming you defeat its monsters in proportion to how many spawn, at the map data's base EXP.">
      About {formatCompact(ground.pace.killsPerHour)} an hour at your measured pace here
    </p>
  {/if}

  {#if !from}
    <p class="note">Routes appear when the minimap is in view.</p>
  {:else if route}
    <div class="route"><RouteSteps {route} /></div>
  {:else if !routing}
    <p class="note">No route through portals is known from {from.name}.</p>
  {/if}
  {#if ground}
    <button class="more" title="See everything on this map" onclick={() => backend.showPlace("maps", ground.map.id)}>Open {ground.map.name}</button>
  {/if}
{/if}

<style>
  .legend {
    display: flex;
    align-items: baseline;
    gap: 8px;
    padding: 0 6px 3px;
    color: var(--faint);
    font-size: 10px;
  }

  .legend span:first-child {
    flex: 1;
  }

  .legend button {
    min-width: 56px;
    padding: 0;
    border: 0;
    background: transparent;
    color: inherit;
    font-size: 10px;
    text-align: right;
    cursor: pointer;
  }

  .legend button:hover,
  .legend button:focus-visible {
    color: var(--text);
    outline: none;
  }

  .legend button.on {
    color: var(--accent-strong);
  }

  .maps {
    display: grid;
    gap: 1px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .ground {
    display: flex;
    align-items: baseline;
    gap: 8px;
    width: 100%;
    padding: 3px 6px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--muted);
    font-size: 11.5px;
    text-align: left;
    cursor: pointer;
  }

  .ground:hover {
    background: rgba(255, 255, 255, 0.04);
    color: var(--text);
  }

  .ground:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -1px;
  }

  .ground.selected {
    background: color-mix(in srgb, var(--map) 12%, transparent);
    color: var(--text);
  }

  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .count,
  .dist {
    flex: none;
    color: var(--faint);
  }

  .count {
    color: var(--muted);
  }

  .count small {
    color: var(--faint);
    font-size: 10px;
  }

  .dist {
    min-width: 56px;
    text-align: right;
  }

  .dist.shortcut {
    color: var(--travel);
  }

  .more {
    margin-top: 3px;
    padding: 0 6px;
    border: 0;
    background: transparent;
    color: var(--muted);
    font-size: 11px;
    cursor: pointer;
  }

  .more:hover,
  .more:focus-visible {
    color: var(--text);
    outline: none;
  }

  .route {
    margin-top: 2px;
  }

  .note {
    margin: 6px 0 0;
    color: var(--faint);
    font-size: 11px;
  }
</style>
