<script lang="ts">
  // A route's steps as the player walks them, and how many portals it takes.
  import { EyeOff } from "@lucide/svelte";

  import type { Direction, MapRoute } from "$lib/types";

  let {
    route,
    onnavigate,
  }: {
    route: MapRoute;
    /** Opens a map named in a step; without it the names are plain text. */
    onnavigate?: (id: number) => void;
  } = $props();

  const verbs: Record<Direction, string> = {
    west: "West portal",
    east: "East portal",
    in: "Go in",
    out: "Go out",
    other: "Portal",
  };
</script>

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
          {#if onnavigate}
            <button class="map" onclick={() => onnavigate(step.to.id)}>{step.to.name}</button>
          {:else}
            <span class="map">{step.to.name}</span>
          {/if}
        </span>
      </li>
    {/each}
  </ol>
  <p class="note">
    {route.steps.length} portal{route.steps.length === 1 ? "" : "s"}. Portal level or quest requirements are not checked.
  </p>
{/if}

<style>
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
  }

  button.map {
    cursor: pointer;
  }

  button.map:hover {
    color: var(--map);
    text-decoration: underline;
  }

  .note {
    margin: 6px 0 0;
    font-size: 11px;
    color: var(--faint);
  }
</style>
