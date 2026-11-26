<script lang="ts">
  // The desk's compass, in the middle of the top row on every view: where you're heading and the
  // one thing to do next from the map you're on, as a sat-nav shows the next turn. It heads for
  // the place set with Go here; with none set, for a quest just finished or the first Next up step
  // that has a place, which one click sets. The way is the route bar's own (wayfinder.svelte.ts),
  // and it moves on as the minimap's name reads. Clicking it opens the whole route under it.
  // Information only: the player walks the way themselves.
  import { tip } from "$lib/tip.svelte";
  import {
    ArrowUp,
    CarTaxiFront,
    Check,
    CircleDot,
    Compass as CompassIcon,
    DoorOpen,
    LogIn,
    LogOut,
    Navigation,
    Plane,
    ScrollText,
    TriangleAlert,
    X,
  } from "@lucide/svelte";
  import { untrack } from "svelte";

  import PortalStrip from "$lib/components/map/PortalStrip.svelte";
  import RouteSteps from "$lib/components/map/RouteSteps.svelte";
  import Ref from "$lib/components/Ref.svelte";
  import { backend } from "$lib/api";
  import { goTo, nav, watchDestination } from "$lib/destination.svelte";
  import { formatNumber } from "$lib/format";
  import { nextSteps, watchNextUp } from "$lib/nextup.svelte";
  import { can } from "$lib/platform";
  import { live } from "$lib/readings.svelte";
  import { changeSettings, display, travelKey } from "$lib/settings.svelte";
  import type { Destination, MapRoute, RouteStep } from "$lib/types";
  import Switch from "$lib/ui/Switch.svelte";
  import { endTrip, trip } from "$lib/wayfinder.svelte";
  import { BEARINGS, HIDDEN_PORTAL_HINT, heading, shortest, walkTime } from "$lib/wayfinding";

  let {
    onpick,
    line = false,
  }: {
    /** Opens where a place is picked (the Map view's Go to…). */
    onpick: () => void;
    /** One line high, for a phone. */
    line?: boolean;
  } = $props();

  watchDestination();
  watchNextUp();

  /** How long the map's name must be out of view before the compass says it's from the last map. */
  const LAST_SEEN_MS = 4000;

  /** The place set with Go here. */
  const set = $derived(nav.destination);
  /** With none set: a quest just finished, else the first Next up step with somewhere to go. */
  const suggestion = $derived.by<{ place: Destination; why: string } | null>(() => {
    if (set) return null;
    if (trip.offer) return { place: trip.offer.destination, why: `${trip.offer.quest} is done` };
    const first = nextSteps().find((s) => s.place);
    return first?.place ? { place: first.place, why: first.title } : null;
  });
  const target = $derived(set ?? suggestion?.place ?? null);

  // A suggestion isn't followed, so its way is worked out here, by the same router, from the map
  // the wayfinder last placed you on.
  let suggested = $state<MapRoute | null>(null);
  let asks = 0;
  const suggestKey = $derived(
    suggestion && trip.here ? JSON.stringify([trip.here.id, suggestion.place.maps.map((m) => m.id), travelKey()]) : "",
  );
  $effect(() => {
    const key = suggestKey;
    const ask = ++asks;
    if (!key) {
      suggested = null;
      return;
    }
    untrack(() => {
      const from = trip.here!.id;
      const maps = suggestion!.place.maps;
      void Promise.all(maps.map((m) => backend.mapRoute(from, m.id).catch(() => null))).then((routes) => {
        if (ask === asks) suggested = shortest(routes);
      });
    });
  });

  const route = $derived(set ? trip.route : suggested);
  const next = $derived(set ? trip.next : 0);
  const step = $derived(route?.steps[next] ?? null);
  const way = $derived(step ? heading(step) : null);
  const left = $derived(route ? route.steps.length - next : 0);
  const rides = $derived(route ? route.steps.slice(next).filter((s) => s.travel).length : 0);
  /** A suggestion for the map you're on. */
  const atIt = $derived(!set && route != null && route.steps.length === 0);

  /**
   * The map's name is in view; otherwise the way is from where you were last seen. A map loading
   * hides the name for a moment, so it counts as gone only after a few seconds.
   */
  const inView = $derived(Boolean(live.analysis?.map?.name));
  let reading = $state(true);
  $effect(() => {
    if (inView) {
      reading = true;
      return;
    }
    const timer = setTimeout(() => (reading = false), LAST_SEEN_MS);
    return () => clearTimeout(timer);
  });
  const notReading = $derived(live.liveStatus.state === "off" || live.liveStatus.state === "failed");

  function rideIcon(s: RouteStep) {
    return s.travel?.kind === "scroll" ? ScrollText : s.travel?.kind === "npc" ? Plane : CarTaxiFront;
  }
  function stillIcon(s: RouteStep) {
    if (s.travel) return rideIcon(s);
    if (way?.near || s.side === "centre") return CircleDot;
    if (s.direction === "in") return LogIn;
    if (s.direction === "out") return LogOut;
    return DoorOpen;
  }
  const turn = $derived(way?.bearing ? BEARINGS.indexOf(way.bearing) * 45 : null);

  /** What stands in for the step when there isn't one. */
  const waiting = $derived.by(() => {
    if (atIt) return target?.kind === "npc" ? `${target.name} is on this map` : "You're on it";
    if (set && trip.lost) return trip.here ? `No known way from ${trip.here.name}` : "No known way";
    if (trip.here) return "Working out the way…";
    if (notReading) return can.liveReading ? "Not reading the game" : "Share the game window to follow the way";
    return "Waiting to read the minimap";
  });

  let open = $state(false);
  let root = $state<HTMLElement>();
  const drawerId = "compass-route";
  function toggle() {
    open = !open;
  }
  function outside(event: MouseEvent) {
    if (open && root && !root.contains(event.target as Node)) open = false;
  }
  function keys(event: KeyboardEvent) {
    if (open && event.key === "Escape") {
      open = false;
      root?.querySelector<HTMLElement>(".face")?.focus();
    }
  }
  // Nothing to show: close the drawer with it.
  $effect(() => {
    if (!target) open = false;
  });

  const showPictures = $derived(display.showPictures);
  const fares = $derived(route?.steps.slice(next).reduce((sum, s) => sum + (s.travel?.fare ?? 0), 0) ?? 0);
</script>

<svelte:window onclick={outside} onkeydown={keys} />

<div
  class="compass"
  class:line
  class:idle={!target && !trip.arrived}
  class:suggested={!set && target}
  class:lost={set && trip.lost}
  bind:this={root}
>
  {#if trip.arrived}
    <div class="face arrived" role="status">
      <span class="dial good"><Check size={16} strokeWidth={2.5} /></span>
      <span class="lines"><span class="main">{trip.arrived}</span></span>
    </div>
  {:else if target}
    <button
      class="face"
      aria-expanded={open}
      aria-controls={drawerId}
      aria-label={`Heading for ${target.name}. ${step && way ? `Next: ${way.words} to ${step.to.name}` : waiting}. Show the whole route`}
      onclick={toggle}
    >
      <span class="dial" class:ride={step?.travel} class:warn={set && trip.lost} class:still={!step}>
        {#if step && turn != null}
          <span class="needle" style:transform="rotate({turn}deg)"><ArrowUp size={17} strokeWidth={2.5} /></span>
        {:else if step}
          {@const Still = stillIcon(step)}<Still size={16} strokeWidth={2.25} />
        {:else if set && trip.lost}
          <TriangleAlert size={15} />
        {:else}
          <CompassIcon size={16} />
        {/if}
      </span>
      <span class="lines">
        <span class="top">
          {#if !set}<span class="chip">{trip.offer ? "Done" : "Next up"}</span>{/if}
          <span class="to">To <b>{target.name}</b></span>
          {#if step}
            <span class="count tabular">· {left} {left === 1 ? "map" : "maps"} to go{#if rides}, {rides} {rides === 1 ? "ride" : "rides"}{/if}</span>
          {/if}
          {#if !reading && trip.here}
            <span class="seen" use:tip={"The minimap's name isn't in view, so the way is from the map you were last seen on"}
              >· from {trip.here.name}, last seen</span
            >
          {:else if trip.unsure}
            <span class="seen">· minimap unclear</span>
          {/if}
        </span>
        <span class="main">
          {#if step && way}
            <span class="say"
              ><span class="verb" class:ride={step.travel}>{way.words}</span><span class="arrow">→</span><span class="map">{step.to.name}</span
              >{#if step.hidden}<span class="hidden">hidden</span>{/if}</span
            >
            <!-- At phone width the line above hides, and the arrow says which way: how far, to where, comes here. -->
            <span class="tail tabular"
              >· {left} to {target.name}{#if !reading && trip.here}<span class="seen">, last seen {trip.here.name}</span>{/if}</span
            >
          {:else}
            <span class="say wait">{waiting}</span><span class="tail">· to {target.name}</span>
          {/if}
        </span>
      </span>
    </button>
    {#if set}
      <button class="end" aria-label={`Stop heading for ${target.name}`} use:tip={"Stop heading there"} onclick={() => endTrip()}
        ><X size={13} /></button
      >
    {:else if !atIt}
      <button class="go" aria-label={`Go to ${target.name}`} use:tip={suggestion?.why ?? null} onclick={() => goTo(target)}
        ><Navigation size={11} />Go</button
      >
    {/if}
  {:else}
    <button class="face idle" use:tip={"Pick a map or NPC to head for, in the Map view"} onclick={onpick}>
      <span class="dial still"><CompassIcon size={16} /></span>
      <span class="lines"><span class="main"><span class="wait">Pick a place</span></span></span>
    </button>
  {/if}

  {#if open && target}
    <div class="drawer" id={drawerId}>
      <p class="d-head">
        {#if !set && suggestion}<span class="why">{suggestion.why}.</span>{/if}
        To <Ref kind={target.kind === "npc" ? "npc" : "map"} id={target.kind === "npc" ? null : target.maps[0].id} name={target.name} />{#if target.kind === "npc" && route?.to}<span class="faint">{` · on ${route.to.name}`}</span>{/if}
      </p>
      {#if step && !step.travel && showPictures}
        <PortalStrip from={step.from} portal={step.portal} to={step.to.id} height={96} />
      {/if}
      {#if step && way}
        <p class="basis">
          {#if step.hidden}<span class="warn-text">{HIDDEN_PORTAL_HINT}.</span>{/if}
          The arrow: {way.basis.toLowerCase()}.{#if next === 0 && !step.travel && !step.toward} Leafline doesn't know where you stand on the map.{/if}
        </p>
      {/if}
      {#if route && route.steps.length}
        <div class="steps"><RouteSteps {route} current={next} note={false} /></div>
        <p class="note tabular">
          {route.steps.length} {route.steps.length === 1 ? "step" : "steps"}{#if route.walkSeconds != null && !route.steps.some((s) => s.travel)}, about
            {walkTime(route.walkSeconds)} on foot{/if}{#if fares} · {formatNumber(fares)} mesos in fares{/if}
        </p>
      {:else if set && trip.lost}
        <p class="note">{display.travel.enabled ? "Some trips need a ship or an NPC." : "Taxis and scrolls may find a way."}</p>
      {/if}
      <div class="switch-row">
        <span class="s-name">Taxis and return scrolls</span>
        <Switch
          checked={display.travel.enabled}
          label="Taxis and return scrolls"
          onchange={(on) => void changeSettings({ travel: { ...display.travel, enabled: on } })}
        />
      </div>
    </div>
  {/if}
</div>

<style>
  .compass {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--space-1);
    width: 100%;
    max-width: 560px;
    min-width: 0;
    padding: 3px var(--space-1) 3px 3px;
    border: var(--line-w) solid color-mix(in srgb, var(--accent) 45%, var(--control-line));
    border-radius: var(--radius-pill);
    background: color-mix(in srgb, var(--accent) 8%, var(--surface));
    box-shadow: var(--card-shadow);
  }

  /* Nowhere to head for: a plain small button, not a bar. */
  .compass.idle {
    width: auto;
    border-color: var(--control-line);
    background: var(--surface);
    box-shadow: none;
  }

  .idle .face {
    padding-right: var(--space-2);
  }

  .idle .dial {
    width: 30px;
    height: 30px;
  }

  .compass.suggested {
    border-style: dashed;
    background: var(--surface);
  }

  .compass.lost {
    border-color: color-mix(in srgb, var(--warn) 55%, var(--control-line));
  }

  .face {
    display: flex;
    flex: 1;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
    padding: 0;
    border: 0;
    border-radius: var(--radius-pill);
    background: none;
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .face.arrived {
    cursor: default;
  }

  .face:focus-visible,
  .end:focus-visible,
  .go:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  /* The dial: the one bold shape, pointing the way. */
  .dial {
    display: grid;
    flex: none;
    place-items: center;
    width: 38px;
    height: 38px;
    border-radius: 50%;
    background: var(--accent);
    color: var(--accent-ink);
  }

  .dial.still {
    background: var(--accent-soft);
    color: var(--accent-text-strong);
  }

  .dial.ride {
    background: var(--travel);
    color: var(--surface);
  }

  .dial.warn {
    background: var(--warn-soft);
    color: var(--warn);
  }

  .dial.good {
    background: color-mix(in srgb, var(--good) 18%, transparent);
    color: var(--good);
  }

  .suggested .dial:not(.still) {
    background: var(--accent-soft);
    color: var(--accent-text-strong);
  }

  .needle {
    display: grid;
    transition: transform 0.3s ease;
  }

  @media (prefers-reduced-motion: reduce) {
    .needle {
      transition: none;
    }
  }

  .lines {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-width: 0;
    line-height: 1.25;
  }

  .top,
  .main {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .top {
    color: var(--muted);
    font-size: var(--text-xs);
  }

  .top b {
    color: var(--text);
    font-weight: var(--weight-strong);
  }

  .chip {
    display: inline-block;
    margin-right: var(--space-1);
    padding: 0 var(--space-1);
    border-radius: var(--radius-sm);
    background: var(--accent-soft);
    color: var(--accent-text-strong);
    font-size: var(--text-xs);
    font-weight: var(--weight-strong);
  }

  .seen {
    color: var(--warn);
  }

  .main {
    display: flex;
    align-items: baseline;
    font-family: var(--font-display);
    font-size: var(--text-md);
    font-weight: var(--weight-strong);
  }

  .say {
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .verb.ride {
    color: var(--travel);
  }

  .arrow {
    margin: 0 var(--space-1);
    color: var(--accent-text);
    font-weight: var(--weight-body);
  }

  .hidden {
    margin-left: var(--space-1);
    color: var(--warn);
    font-size: var(--text-xs);
    font-weight: var(--weight-body);
  }

  .tail {
    display: none;
    flex: none;
    max-width: 65%;
    overflow: hidden;
    text-overflow: ellipsis;
    margin-left: var(--space-1);
    color: var(--muted);
    font-family: var(--font-sans);
    font-size: var(--text-xs);
    font-weight: var(--weight-body);
  }

  .wait {
    color: var(--muted);
    font-weight: var(--weight-body);
  }

  .end,
  .go {
    display: inline-flex;
    flex: none;
    align-items: center;
    justify-content: center;
    gap: var(--space-1);
    border: var(--line-w) solid var(--control-line);
    background: var(--surface);
    color: var(--muted);
    cursor: pointer;
  }

  .end {
    width: 26px;
    height: 26px;
    border-radius: 50%;
  }

  .go {
    height: 26px;
    padding: 0 var(--space-2-5, 10px);
    border-color: color-mix(in srgb, var(--accent) 55%, transparent);
    border-radius: var(--radius-pill);
    background: var(--accent-soft);
    color: var(--accent-text-strong);
    font-size: var(--text-xs);
    font-weight: var(--weight-strong);
  }

  .end:hover {
    background: var(--surface-2);
    color: var(--text);
  }

  .go:hover {
    background: var(--accent);
    color: var(--accent-ink);
  }

  /* The whole route, opened under the compass over the view. */
  .drawer {
    position: absolute;
    top: calc(100% + var(--space-1-5));
    left: 50%;
    z-index: 30;
    width: min(480px, calc(100cqw - 2 * var(--space-4)));
    max-height: min(70vh, 560px);
    overflow-y: auto;
    padding: var(--space-3);
    border: var(--line-w) solid var(--line);
    border-radius: var(--radius-lg);
    background: var(--surface);
    box-shadow: var(--card-shadow), 0 12px 32px rgb(0 0 0 / 0.18);
    transform: translateX(-50%);
  }

  .d-head {
    margin: 0 0 var(--space-2);
    font-size: var(--text-md);
  }

  .why {
    margin-right: var(--space-1);
    color: var(--muted);
  }

  .basis,
  .note {
    margin: var(--space-1-5) 0 0;
    color: var(--faint);
    font-size: var(--text-xs);
    line-height: 1.4;
  }

  .warn-text {
    color: var(--warn);
  }

  .faint {
    color: var(--faint);
  }

  .steps :global(.steps) {
    margin-top: var(--space-2);
  }

  .switch-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    margin-top: var(--space-2);
    padding-top: var(--space-2);
    border-top: var(--line-w) solid var(--line);
  }

  .s-name {
    color: var(--muted);
    font-size: var(--text-xs);
  }

  /* One line high: on a phone, and in a narrow desk window, where it has a row of its own. */
  .compass.line {
    max-width: none;
    padding: 2px var(--space-1) 2px 2px;
  }

  .line .dial {
    width: 28px;
    height: 28px;
  }

  .line .top {
    display: none;
  }

  .line .tail {
    display: inline;
  }

  /* The arrow says which way; a ride still says what it is. */
  .line .verb:not(.ride),
  .line .verb:not(.ride) + .arrow {
    display: none;
  }

  .line .say {
    flex: 0 1 auto;
  }

  @container desk-main (max-width: 640px) {
    .compass {
      max-width: none;
      padding: 2px var(--space-1) 2px 2px;
    }

    .dial {
      width: 28px;
      height: 28px;
    }

    .top {
      display: none;
    }

    .tail {
      display: inline;
    }

    .verb:not(.ride),
    .verb:not(.ride) + .arrow {
      display: none;
    }
  }
</style>
