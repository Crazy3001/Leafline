<script lang="ts">
  // The slim "next step" bar for "Go here", under the overlay's header, in the compact HUD and
  // in the Route pin (which shows the trip the overlay follows, see followTrip):
  // the step to take from the map the player is on, how many follow, and a way to cancel. It
  // moves on by itself as the minimap reading changes (see wayfinder.svelte.ts). Also offers
  // the turn-in place of a quest whose counters just filled.
  import {
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    CarTaxiFront,
    Check,
    DoorOpen,
    LogIn,
    LogOut,
    Navigation,
    Pin,
    ScrollText,
    X,
  } from "@lucide/svelte";

  import GoButton from "./GoButton.svelte";
  import { backend } from "$lib/api";
  import { nav } from "$lib/destination.svelte";
  import { formatNumber } from "$lib/format";
  import { showNpcNamed } from "$lib/places.svelte";
  import { live } from "$lib/readings.svelte";
  import { display } from "$lib/settings.svelte";
  import type { Destination, RouteStep } from "$lib/types";
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
  const total = $derived(trip.route?.steps.length ?? 0);
  /** Past this many steps the bar of segments gets too fine to read, and the count says it. */
  const MAX_SEGMENTS = 16;

  /** The picture for a step: the way to walk for a portal, the ride or scroll otherwise. */
  function turnIcon(step: RouteStep) {
    if (step.travel) return step.travel.kind === "scroll" ? ScrollText : CarTaxiFront;
    if (step.side === "left" || (!step.side && step.direction === "west")) return ArrowLeft;
    if (step.side === "right" || (!step.side && step.direction === "east")) return ArrowRight;
    if (step.side === "centre") return ArrowUp;
    if (step.direction === "in") return LogIn;
    if (step.direction === "out") return LogOut;
    return DoorOpen;
  }
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
  {@const Turn = step ? turnIcon(step) : Navigation}
  <div class="bar trip" class:hud class:pin class:lost={trip.lost}>
    <!-- Which way to go, big enough to take in at a glance, as a sat-nav shows the next turn. -->
    <span class="turn" class:ride={step?.travel}><Turn size={hud ? 13 : 18} strokeWidth={2.25} /></span>
    <div class="body">
      {#if step && !hud}
        <p class="head">
          <span class="label">Next step</span>
          {#if total > 1}<span class="faint tabular">{trip.next + 1} of {total}</span>{/if}
        </p>
      {/if}
      <p class="next">
        {#if step}
          {#if step.travel}
            <span class="ride">{step.travel.service}</span>
          {:else}
            <span class="verb">{stepVerb(step)}</span>
          {/if}
          <span class="arrow">→</span>
          <span class="map">{step.to.name}</span>
          {#if step.travel?.fare}<span class="faint tabular fare">{formatNumber(step.travel.fare)} mesos</span>{/if}
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
      {#if step && total > 1 && total <= MAX_SEGMENTS && !hud}
        <!-- The trip so far: steps taken, the one to take now, and those after it. -->
        <div class="progress" aria-hidden="true">
          {#each { length: total } as _, i}<span class:done={i < trip.next} class:now={i === trip.next}></span>{/each}
        </div>
      {/if}
    </div>
    {#if canPin}
      <button class="close" title="Pin the route" aria-label="Pin the route" onclick={pinRoute}
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
    align-items: center;
    gap: 10px;
    padding: 8px 6px 8px 8px;
    border-left: 3px solid var(--accent);
    background: color-mix(in srgb, var(--accent) 7%, var(--surface-2));
  }

  .trip.lost {
    border-left-color: var(--warn);
  }

  .trip.hud {
    gap: 7px;
    padding: 4px 4px 4px 6px;
    border-left-width: 2px;
  }

  /* The next turn, as a square tile: the one big shape on the bar. */
  .turn {
    display: grid;
    flex: none;
    place-items: center;
    width: 34px;
    height: 34px;
    border: 1px solid color-mix(in srgb, var(--accent) 55%, transparent);
    border-radius: var(--radius-sm);
    background: var(--accent-soft);
    color: var(--accent-strong);
  }

  .turn.ride {
    border-color: color-mix(in srgb, var(--travel) 55%, transparent);
    background: color-mix(in srgb, var(--travel) 14%, transparent);
    color: var(--travel);
  }

  .lost .turn {
    border-color: color-mix(in srgb, var(--warn) 55%, transparent);
    background: var(--warn-soft);
    color: var(--warn);
  }

  .hud .turn {
    width: 22px;
    height: 22px;
  }

  .head {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    margin: 0 0 1px;
    font-size: var(--text-xs);
    line-height: 1.2;
  }

  .progress {
    display: flex;
    gap: 2px;
    margin-top: 5px;
  }

  .progress span {
    flex: 1;
    height: 3px;
    background: var(--border-strong);
  }

  .progress .done {
    background: color-mix(in srgb, var(--accent) 45%, transparent);
  }

  .progress .now {
    background: var(--accent-strong);
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
    font-size: var(--text-lg);
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
    gap: 0 6px;
    font-size: var(--text-md);
    font-weight: 600;
    line-height: 1.3;
  }

  .hud .next {
    font-size: var(--text-sm);
  }

  .label {
    color: var(--accent-strong);
    font-family: var(--font-display);
    font-weight: 400;
  }

  .verb {
    color: var(--text);
  }

  .ride {
    color: var(--travel);
  }

  .arrow {
    color: var(--accent);
    font-weight: 400;
  }

  .map {
    color: var(--ref-map);
  }

  .fare {
    font-size: var(--text-xs);
    font-weight: 400;
  }

  .rest {
    margin-top: 1px;
    color: var(--muted);
    font-size: var(--text-xs);
  }

  .rest .ref {
    font-weight: 600;
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
