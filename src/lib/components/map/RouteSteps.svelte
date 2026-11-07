<script lang="ts">
  // A route's steps as the player takes them, and how many portals it takes. Rides and return
  // scrolls, when the player turned them on, stand apart from portal steps.
  import { CarTaxiFront, EyeOff, ScrollText } from "@lucide/svelte";

  import { formatNumber } from "$lib/format";
  import type { MapRoute } from "$lib/types";
  import { stepVerb as verb } from "$lib/wayfinding";

  let {
    route,
    onnavigate,
  }: {
    route: MapRoute;
    /** Opens a map named in a step; without it the names are plain text. */
    onnavigate?: (id: number) => void;
  } = $props();

  const rides =$derived(route.steps.filter((s) => s.travel).length);
  const portals = $derived(route.steps.length - rides);
</script>

{#if route.steps.length === 0}
  <p class="note">You are already there.</p>
{:else}
  <ol class="steps">
    {#each route.steps as step, i}
      <li class:ride={step.travel}>
        <span class="n tabular">{i + 1}</span>
        <span class="text">
          {#if step.travel}
            <span class="service">
              {#if step.travel.kind === "scroll"}<ScrollText size={12} />{:else}<CarTaxiFront size={12} />{/if}
              {step.travel.service}
            </span>
          {:else}
            <span class="verb">{verb(step)}</span>
            {#if step.hidden}<span class="hidden" title="The portal's name marks it as hidden"><EyeOff size={11} /></span>{/if}
          {/if}
          <span class="arrow">→</span>
          {#if onnavigate}
            <button class="map" onclick={() => onnavigate(step.to.id)}>{step.to.name}</button>
          {:else}
            <span class="map">{step.to.name}</span>
          {/if}
          {#if step.travel?.fare}
            <span class="fare tabular" title="Fare at level 20 and up, as players recorded it on mapleclassic.wiki"
              >{formatNumber(step.travel.fare)} mesos</span
            >
          {/if}
        </span>
      </li>
    {/each}
  </ol>
  {#if rides}
    <p class="note">
      {portals} portal{portals === 1 ? "" : "s"} and {rides}
      {rides === 1 ? "ride or scroll" : "rides or scrolls"}{#if route.walking != null}, instead of {route.walking} portals on foot{:else}; no portals lead
        there on foot{/if}. Taxis and scrolls are from mapleclassic.wiki: check them in game.
    </p>
  {:else}
    <p class="note">
      {route.steps.length} portal{route.steps.length === 1 ? "" : "s"}. Portal level or quest requirements are not checked.
    </p>
  {/if}
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
    font-size: var(--text-sm);
  }

  .n {
    flex: none;
    display: grid;
    place-items: center;
    width: 17px;
    height: 17px;
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--map) 16%, transparent);
    color: var(--map);
    font-size: var(--text-xs);
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

  .ride .n {
    background: color-mix(in srgb, var(--travel) 18%, transparent);
    color: var(--travel);
  }

  .service {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: var(--travel);
    font-weight: 500;
  }

  .service :global(svg) {
    align-self: center;
  }

  .fare {
    color: var(--faint);
    font-size: var(--text-xs);
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
    font-size: var(--text-xs);
    color: var(--faint);
  }
</style>
