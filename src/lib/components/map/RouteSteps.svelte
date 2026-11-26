<script lang="ts">
  // A route's steps as the player takes them, and how many portals it takes. Rides and return
  // scrolls, when the player turned them on, stand apart from portal steps. On a trip being
  // followed, the steps taken are ticked off and the one to take now stands out.
  import { tip } from "$lib/tip.svelte";
  import { CarTaxiFront, Check, EyeOff, Plane, ScrollText } from "@lucide/svelte";

  import Ref from "$lib/components/Ref.svelte";
  import { formatNumber } from "$lib/format";
  import type { MapRoute } from "$lib/types";

  import { HIDDEN_PORTAL_HINT, stepVerb as verb, walkTime } from "$lib/wayfinding";

  let {
    route,
    current = null,
    note = true,
  }: {
    route: MapRoute;
    /** On a trip being followed, the step to take now. */
    current?: number | null;
    /** The line under the steps saying how many portals and how long; off where it is said elsewhere. */
    note?: boolean;
  } = $props();

  const rides = $derived(route.steps.filter((s) => s.travel).length);
  const portals = $derived(route.steps.length - rides);
</script>

{#if route.steps.length === 0}
  <p class="note">You are already there.</p>
{:else}
  <ol class="steps" class:following={current != null}>
    {#each route.steps as step, i}
      {@const done = current != null && i < current}
      <li
        class:ride={step.travel}
        class:end={i === route.steps.length - 1}
        class:done
        class:now={current === i}
        aria-current={current === i ? "step" : undefined}
      >
        <span class="n tabular">{#if done}<Check size={11} strokeWidth={3} />{:else}{i + 1}{/if}</span>
        <span class="text">
          {#if step.travel}
            <span class="service">
              {#if step.travel.kind === "scroll"}<ScrollText size={12} />{:else if step.travel.kind === "npc"}<Plane size={12} />{:else}<CarTaxiFront size={12} />{/if}
              {step.travel.service}
            </span>
          {:else}
            <span class="verb">{verb(step)}</span>
            {#if step.hidden}<span class="hidden" use:tip={"The portal's name marks it as hidden: the game draws nothing there"}
                ><EyeOff size={11} /></span
              >{/if}
          {/if}
          <span class="arrow">→</span>
          {#if step.to.known}<Ref kind="map" id={step.to.id} name={step.to.name} class="step-map" />{:else}<span class="map"
              >{step.to.name}</span
            >{/if}
          {#if step.hidden && !step.travel}<span class="hidden-hint">· {HIDDEN_PORTAL_HINT}</span>{/if}
          {#if step.travel?.fare}
            <span class="fare tabular" use:tip={"Fare at level 20 and up, as players recorded it on mapleclassic.wiki"}
              >{formatNumber(step.travel.fare)} mesos</span
            >
          {/if}
        </span>
      </li>
    {/each}
  </ol>
  {#if !note}
    <!-- Said by whoever shows the steps. -->
  {:else if rides}
    <p class="note">
      {portals} portal{portals === 1 ? "" : "s"} and {rides}
      {rides === 1 ? "ride or scroll" : "rides or scrolls"}{#if route.walking != null}, instead of {route.walking} portals on foot{:else}; no portals lead
        there on foot{/if}. Taxis and scrolls are from mapleclassic.wiki: check them in game.
    </p>
  {:else}
    <p class="note">
      {route.steps.length} portal{route.steps.length === 1 ? "" : "s"}{#if route.walkSeconds != null}, about {walkTime(route.walkSeconds)} on foot{/if}.
      The quickest walk, counting the way across each map. Portal level or quest requirements are not checked.
    </p>
  {/if}
{/if}

<style>
  /* A vertical line of stops: each step a dot on the rail, the last one filled, where it ends. */
  .steps {
    display: grid;
    margin: var(--space-2) 0 0;
    padding: 0;
    list-style: none;
  }

  .steps li {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-height: 28px;
    padding: var(--space-0-5) 0;
    font-size: var(--text-md);
  }

  .steps li::before {
    content: "";
    position: absolute;
    top: 0;
    bottom: 0;
    left: 9px;
    width: 2px;
    border-radius: 1px;
    background: color-mix(in srgb, var(--map) 28%, transparent);
  }

  .steps li:first-child::before {
    top: 50%;
  }

  .steps li:last-child::before {
    bottom: 50%;
  }

  .steps li:only-child::before {
    display: none;
  }

  .n {
    position: relative;
    z-index: 1;
    flex: none;
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    border: 1.5px solid color-mix(in srgb, var(--map) 70%, transparent);
    border-radius: 50%;
    background: var(--surface-2);
    color: var(--map);
    font-size: var(--text-xs);
    font-weight: var(--weight-strong);
  }

  .end .n {
    border-color: var(--map);
    background: var(--map);
    box-shadow: 0 0 10px color-mix(in srgb, var(--map) 45%, transparent);
    color: var(--accent-ink);
  }

  /* On a trip being followed the end is only outlined, so the step to take now is the one that stands out. */
  .following .end:not(.now) .n {
    background: var(--surface-2);
    box-shadow: none;
    color: var(--map);
  }

  /* Steps already taken fade back; the one to take now gets the accent. */
  .done .n {
    border-color: color-mix(in srgb, var(--map) 35%, transparent);
    color: color-mix(in srgb, var(--map) 70%, transparent);
  }

  .done .text {
    opacity: 0.55;
  }

  .now .n {
    border-color: var(--accent-strong);
    background: var(--accent-strong);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 25%, transparent);
    color: var(--accent-ink);
  }

  .now .text {
    font-weight: var(--weight-strong);
  }

  .text {
    display: inline-flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: var(--space-1);
    min-width: 0;
  }

  .verb {
    color: var(--muted);
  }

  .ride .n {
    border-color: color-mix(in srgb, var(--travel) 70%, transparent);
    color: var(--travel);
  }

  .ride.end .n {
    background: var(--travel);
    color: var(--accent-ink);
  }

  .service {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    color: var(--travel);
    font-weight: var(--weight-body);
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

  .hidden-hint {
    color: var(--warn);
    font-size: var(--text-xs);
  }

  .arrow {
    color: var(--faint);
  }

  .map,
  .text :global(.step-map) {
    font-weight: var(--weight-body);
  }

  .note {
    margin: var(--space-1-5) 0 0;
    font-size: var(--text-xs);
    color: var(--faint);
  }
</style>
