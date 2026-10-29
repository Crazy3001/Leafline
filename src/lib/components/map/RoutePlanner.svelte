<script lang="ts">
  // "Route to…": the fewest portals from this map to another, as steps the player walks.
  import { EyeOff, Flag, X } from "@lucide/svelte";

  import { backend } from "$lib/api";
  import type { Direction, MapRef, MapRoute } from "$lib/types";
  import MapPicker from "./MapPicker.svelte";

  let { from, onnavigate }: { from: MapRef; onnavigate: (id: number) => void } = $props();

  let target = $state<MapRef | null>(null);
  let route = $state<MapRoute | null>(null);
  let loading = $state(false);

  // Recompute when the card moves to another map.
  $effect(() => {
    const start = from.id;
    const to = target;
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

  const verbs: Record<Direction, string> = {
    west: "West portal",
    east: "East portal",
    in: "Go in",
    out: "Go out",
    other: "Portal",
  };
</script>

{#if target}
  <div class="target">
    <Flag size={12} />
    <span class="to">To <strong>{target.name}</strong></span>
    <button class="clear" title="Clear the route" onclick={() => (target = null)}><X size={12} /></button>
  </div>
  {#if route}
    {#if route.steps.length === 0}
      <p class="note">You are already there.</p>
    {:else}
      <ol class="steps">
        {#each route.steps as step, i}
          <li>
            <span class="n tabular">{i + 1}</span>
            <span class="text">
              <span class="verb">{verbs[step.direction]}</span>
              {#if step.portal}<code title="Portal name in the map data">{step.portal}</code>{/if}
              {#if step.hidden}<span class="hidden" title="The portal's name marks it as hidden"><EyeOff size={11} /></span>{/if}
              <span class="arrow">→</span>
              <button class="map" onclick={() => onnavigate(step.to.id)}>{step.to.name}</button>
            </span>
          </li>
        {/each}
      </ol>
      <p class="note">
        {route.steps.length} portal{route.steps.length === 1 ? "" : "s"}. Portal level or quest requirements are not
        checked.
      </p>
    {/if}
  {:else if !loading}
    <p class="note">
      No route through portals is known. Some trips need a ship, taxi or return scroll, which the data doesn't list.
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
    font-size: 12px;
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
    border-radius: 4px;
    background: transparent;
    color: var(--faint);
    cursor: pointer;
  }

  .clear:hover {
    background: rgba(255, 255, 255, 0.07);
    color: var(--text);
  }

  .steps {
    display: grid;
    gap: 2px;
    margin: 8px 0 0;
    padding: 0;
    list-style: none;
  }

  .steps li {
    display: flex;
    align-items: baseline;
    gap: 8px;
    padding: 3px 0;
    font-size: 12px;
  }

  .n {
    flex: none;
    display: grid;
    place-items: center;
    width: 17px;
    height: 17px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--map) 16%, transparent);
    color: var(--map);
    font-size: 10px;
    font-weight: 600;
  }

  .text {
    display: inline-flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 4px;
    min-width: 0;
  }

  .verb {
    color: var(--muted);
  }

  code {
    font-family: var(--font-mono);
    font-size: 10.5px;
    color: var(--faint);
  }

  .hidden {
    color: var(--warn);
    align-self: center;
    display: inline-grid;
  }

  .arrow {
    color: var(--faint);
  }

  .map {
    padding: 0;
    border: 0;
    background: transparent;
    font-weight: 500;
    text-align: left;
    cursor: pointer;
  }

  .map:hover {
    color: var(--map);
    text-decoration: underline;
  }

  .note {
    margin: 6px 0 0;
    font-size: 11px;
    color: var(--faint);
  }
</style>
