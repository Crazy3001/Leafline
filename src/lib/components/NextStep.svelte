<script lang="ts">
  // The slim "next step" bar for "Go here", under the overlay's header, in the compact HUD and
  // in the Route pin (which shows the trip the overlay follows, see followTrip):
  // the step to take from the map the player is on, how many follow, and a way to cancel. It
  // moves on by itself as the minimap reading changes (see wayfinder.svelte.ts). Also offers
  // the turn-in place of a quest whose counters just filled.
  import { CarTaxiFront, Check, Navigation, Pin, ScrollText, X } from "@lucide/svelte";

  import GoButton from "./GoButton.svelte";
  import { backend } from "$lib/api";
  import { nav } from "$lib/destination.svelte";
  import { formatNumber } from "$lib/format";
  import { showNpcNamed } from "$lib/places.svelte";
  import { live } from "$lib/readings.svelte";
  import { display } from "$lib/settings.svelte";
  import type { Destination } from "$lib/types";
  import { dismissOffer, endTrip, trip } from "$lib/wayfinder.svelte";
  import { stepVerb } from "$lib/wayfinding";

  let { variant = "overlay" }: { variant?: "overlay" | "hud" | "pin" } = $props();

  const hud = $derived(variant === "hud");
  const pin = $derived(variant === "pin");
  /** The overlay's bar offers the Route pin, a window of its own to keep in view. */
  const canPin = $derived(variant === "overlay" && !live.pins.pinned.includes("route"));

  async function pinRoute() {
    live.pins = await backend.pinPanel("route");
  }
  const destination = $derived(nav.destination);
  const step = $derived(trip.route?.steps[trip.next] ?? null);
  const after = $derived(trip.route ? trip.route.steps.length - trip.next - 1 : 0);
  /** The map the route ends on, which says where an NPC stands. */
  const end = $derived(trip.route?.to ?? null);

  function open(place: Destination) {
    if (place.kind === "npc") void showNpcNamed(place.name);
    else void backend.showPlace("maps", place.maps[0].id);
  }
</script>

{#snippet name(place: Destination)}
  {#if hud}
    <span class="who" class:npc={place.kind === "npc"}>{place.name}</span>
  {:else}
    <button class="ref {place.kind === 'npc' ? 'ref-npc' : 'ref-map'}" onclick={() => open(place)}>{place.name}</button>
  {/if}
{/snippet}

{#if trip.offer}
  {@const offer = trip.offer}
  <div class="bar offer" class:hud class:pin role="status">
    <span class="icon good"><Check size={11} /></span>
    <span class="line">
      <span class="quest">{offer.quest}</span> is done.
      {offer.giver ? "Back to" : "Turn in to"}
      {@render name(offer.destination)}{#if offer.giver}<span class="faint">, who gave it</span>{/if}
    </span>
    <GoButton place={offer.destination} />
    <button class="close" aria-label="Not now" onclick={dismissOffer}><X size={11} /></button>
  </div>
{/if}

{#if trip.arrived}
  <div class="bar arrived" class:hud class:pin role="status">
    <span class="icon good"><Check size={11} /></span>
    <span class="line">{trip.arrived}</span>
  </div>
{:else if destination}
  <div class="bar trip" class:hud class:pin>
    <span class="icon"><Navigation size={11} /></span>
    <div class="body">
      <p class="next">
        {#if step}
          <span class="label">Next</span>
          {#if step.travel}
            <span class="ride">
              {#if step.travel.kind === "scroll"}<ScrollText size={11} />{:else}<CarTaxiFront size={11} />{/if}
              {step.travel.service}
            </span>
          {:else}
            <span class="verb">{stepVerb(step)}</span>
          {/if}
          <span class="arrow">→</span>
          <span class="map">{step.to.name}</span>
          {#if step.travel?.fare}<span class="faint tabular">{formatNumber(step.travel.fare)} mesos</span>{/if}
        {:else if trip.lost}
          <span class="label">No known way</span>
          {#if trip.here}<span class="faint">from {trip.here.name}</span>{/if}
        {:else if trip.here}
          <span class="faint">Working out the way…</span>
        {:else}
          <span class="faint">Waiting to read the minimap</span>
        {/if}
      </p>
      <p class="rest">
        {#if trip.rerouted && trip.here}<span class="warn">New route from {trip.here.name} ·</span>{/if}
        {#if step && !after && destination.kind === "map"}
          <!-- The step already names the map. -->
          Last step
        {:else}
          {#if step}
            {after ? `${after} more to` : "Then you're at"}
          {:else}
            To
          {/if}
          {@render name(destination)}{#if destination.kind === "npc" && end}<span class="faint">{` · ${end.name}`}</span>{/if}
        {/if}
        {#if trip.lost && !hud}
          <span class="faint"
            >· {display.travel.enabled ? "some trips need a ship or an NPC" : "turn on travel in Settings for taxis and scrolls"}</span
          >
        {/if}
        {#if trip.unsure && !trip.rerouted}<span class="faint">{" · minimap unclear"}</span>{/if}
      </p>
    </div>
    {#if canPin}
      <button class="close" title="Pin the route: keep the next step in a window of its own" aria-label="Pin the route" onclick={pinRoute}
        ><Pin size={11} /></button
      >
    {/if}
    <button class="close" aria-label="Cancel the trip" onclick={() => endTrip()}><X size={11} /></button>
  </div>
{:else if pin && !trip.offer}
  <p class="idle">Not heading anywhere. Pick <strong>Go</strong> beside a map or NPC, or <strong>Go to…</strong> in the Map tab.</p>
{/if}

<style>
  .bar {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    padding: 5px 6px 5px 10px;
    border-bottom: 1px solid var(--border);
    background: var(--surface-2);
    font-size: var(--text-sm);
  }

  .bar.hud {
    padding: 4px 4px 4px 10px;
    border-top: 1px solid var(--border);
    border-bottom: 0;
    background: transparent;
    font-size: var(--text-xs);
  }

  .trip {
    border-left: 2px solid var(--accent);
    padding-left: 8px;
  }

  /* The pin's window frames it, and the next step is all it shows, so it reads larger. */
  .bar.pin {
    padding: 6px 2px 6px 8px;
    border-bottom: 0;
  }

  .bar.pin + .bar.pin {
    margin-top: 6px;
  }

  .pin .next {
    font-size: var(--text-md);
  }

  .idle {
    margin: 0;
    padding: 4px 0;
    color: var(--faint);
    font-size: var(--text-sm);
  }

  .idle strong {
    color: var(--muted);
    font-weight: 600;
  }

  .icon {
    display: inline-grid;
    flex: none;
    margin-top: 3px;
    color: var(--accent);
  }

  .icon.good {
    color: var(--good);
  }

  .body {
    flex: 1;
    min-width: 0;
  }

  .next,
  .rest {
    margin: 0;
  }

  .next {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 0 5px;
    font-weight: 500;
  }

  .label {
    color: var(--accent-strong);
    font-family: var(--font-display);
    font-weight: 400;
  }

  .verb {
    color: var(--muted);
  }

  .ride {
    display: inline-flex;
    align-self: center;
    align-items: center;
    gap: 3px;
    color: var(--travel);
  }

  .arrow {
    color: var(--faint);
  }

  .map {
    color: var(--ref-map);
  }

  .rest {
    color: var(--muted);
    font-size: var(--text-xs);
  }

  .line {
    flex: 1;
    min-width: 0;
    line-height: 1.4;
  }

  .arrived .line {
    color: var(--good);
    font-weight: 500;
  }

  .quest {
    font-weight: 600;
  }

  .offer {
    align-items: center;
  }

  .offer .icon {
    margin-top: 0;
  }

  .who {
    color: var(--ref-map);
  }

  .who.npc {
    color: var(--ref-npc);
  }

  .faint {
    color: var(--faint);
  }

  .warn {
    color: var(--warn);
  }

  .close {
    display: grid;
    flex: none;
    place-items: center;
    width: 20px;
    height: 20px;
    padding: 0;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--faint);
    cursor: pointer;
  }

  .close:hover,
  .close:focus-visible {
    background: rgba(255, 255, 255, 0.07);
    color: var(--text);
    outline: none;
  }
</style>
