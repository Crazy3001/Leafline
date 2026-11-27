<script lang="ts" module>
  import type { MapDetails, MapPicture } from "$lib/types";

  /** Maps' cards and pictures already fetched, so a map read again never blanks the compass. */
  const cards = new Map<number, MapDetails | null>();
  const pictures = new Map<number, MapPicture | null>();
</script>

<script lang="ts">
  // The desk's compass, centred under the top row on every view: the map you're on with its
  // portals, as the map card's Portals section shows them (left and right portals beside it, doors
  // above or below, each with a small box showing where it stands). With a place set by Go here,
  // the portal to take next is lit and the map's box says where it leads and how far is left;
  // without one, each portal opens its map's card, whose Go sets it. Clicking the map's box opens
  // the route under it. On a phone it is one band, with every portal under the map's box. It
  // follows the map the wayfinder last placed you on (wayfinder.svelte.ts). Information only:
  // Leafline never draws in the game, and the player walks the way themselves.
  import { tip } from "$lib/tip.svelte";
  import { Check, ChevronDown, Navigation, X } from "@lucide/svelte";

  import ConnectionCompass from "$lib/components/map/ConnectionCompass.svelte";
  import RouteSteps from "$lib/components/map/RouteSteps.svelte";
  import Ref from "$lib/components/Ref.svelte";
  import { backend } from "$lib/api";
  import { nav, watchDestination } from "$lib/destination.svelte";
  import { formatNumber } from "$lib/format";
  import { openPeek } from "$lib/peek.svelte";
  import { can } from "$lib/platform";
  import { live } from "$lib/readings.svelte";
  import { changeSettings, display } from "$lib/settings.svelte";
  import Switch from "$lib/ui/Switch.svelte";
  import { endTrip, trip } from "$lib/wayfinder.svelte";
  import { walkTime } from "$lib/wayfinding";

  let {
    onpick,
    line = false,
  }: {
    /** Opens where a place is picked (the Map view's Go to…). */
    onpick: () => void;
    /** For a phone: one portal each side. */
    line?: boolean;
  } = $props();

  watchDestination();

  /** How long the map's name must be out of view before the compass says it's the last map seen. */
  const LAST_SEEN_MS = 4000;

  const dest = $derived(nav.destination);
  const here = $derived(trip.here);

  // The map's portals (its card) and where each stands (its picture's marks), fetched once a map.
  let card = $state<MapDetails | null>(null);
  let picture = $state<MapPicture | null>(null);
  const hereId = $derived(here?.id ?? null);
  $effect(() => {
    const id = hereId;
    if (id == null) {
      card = null;
      picture = null;
      return;
    }
    card = cards.get(id) ?? null;
    picture = pictures.get(id) ?? null;
    if (!cards.has(id))
      backend
        .mapCard(id)
        .catch(() => null)
        .then((c) => {
          cards.set(id, c);
          if (hereId === id) card = c;
        });
    if (!pictures.has(id))
      backend
        .mapPicture(id)
        .catch(() => null)
        .then((p) => {
          pictures.set(id, p);
          if (hereId === id) picture = p;
        });
  });

  const route = $derived(dest ? trip.route : null);
  const step = $derived(route?.steps[trip.next] ?? null);
  const left = $derived(route ? route.steps.length - trip.next : 0);
  /** The portal to take from this map, when the next step is a portal from here. */
  const next = $derived(step && !step.travel && step.from.id === hereId ? { portal: step.portal, to: step.to.id } : null);

  /**
   * The map's name is in view; otherwise the map shown is where you were last seen. A map loading
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

  /** With no map placed yet, what the compass is waiting for. */
  const waiting = $derived(
    notReading ? (can.liveReading ? "Not reading the game" : "Share the game window to see this map's portals") : "Waiting to read the minimap",
  );

  /**
   * On a phone the band shows a map's portals when it has two at most; otherwise only the next
   * one, with the rest counted either side.
   */
  const perSide = $derived((card?.exits.length ?? 0) <= 2 ? 1 : 0);

  let open = $state(false);
  let root = $state<HTMLElement>();
  const drawerId = "compass-view";
  function outside(event: MouseEvent) {
    if (open && root && !root.contains(event.target as Node)) open = false;
  }
  function keys(event: KeyboardEvent) {
    if (open && event.key === "Escape") {
      open = false;
      root?.querySelector<HTMLElement>(".hub-btn")?.focus();
    }
  }

  /** A portal clicked: its map's card, whose Go heads there. */
  function portalCard(id: number, from: Element) {
    const exit = card?.exits.find((e) => e.to.id === id);
    openPeek({ kind: "map", id, name: exit?.to.name ?? null }, from);
  }

  const fares = $derived(route?.steps.slice(trip.next).reduce((sum, s) => sum + (s.travel?.fare ?? 0), 0) ?? 0);
</script>

<svelte:window onclick={outside} onkeydown={keys} />

{#snippet hub()}
  <button
    class="hub-btn"
    aria-expanded={open}
    aria-controls={drawerId}
    use:tip={open ? null : line ? (dest ? "The whole route and every portal here" : "Every portal here") : dest ? "The whole route" : "Where to head for"}
    onclick={() => (open = !open)}
  >
    <span class="here-name">
      {here?.name ?? waiting}{#if here && !reading}<span class="seen">last seen</span>{:else if here && trip.unsure}<span
          class="seen">unclear</span
        >{/if}
    </span>
    {#if trip.arrived}
      <span class="sub good"><Check size={11} strokeWidth={3} />{trip.arrived}</span>
    {:else if dest}
      <span class="sub" class:warn={trip.lost}>
        {#if step && line && next}
          <!-- The lit portal beside it says where. -->
          {left} {left === 1 ? "map" : "maps"} to {dest.name}
        {:else if step}
          <b>Next:</b>
          {step.travel ? `${step.travel.service} →` : "→"}
          {step.to.name}<span class="then">{line ? " · " : ""}{left} {left === 1 ? "map" : "maps"} to {dest.name}</span>
        {:else if trip.lost}
          No known way to {dest.name}
        {:else if here}
          Working out the way to {dest.name}…
        {:else}
          Heading for {dest.name}
        {/if}
      </span>
    {:else if here?.street}
      <span class="sub faint">{here.street}</span>
    {/if}
    <span class="chev" class:open aria-hidden="true"><ChevronDown size={11} /></span>
  </button>
{/snippet}

<div class="compass" class:line class:going={dest && step} bind:this={root}>
  {#if here}
    <!-- The map card's Portals view on the desk, which has the height for it; a band on a phone. -->
    <ConnectionCompass
      layout={line ? "strip" : "compass"}
      exits={card?.exits ?? []}
      map={here}
      current={reading ? here : null}
      points={picture?.points ?? null}
      {next}
      {perSide}
      doors={0}
      {hub}
      onmore={() => (open = true)}
      onnavigate={portalCard}
    />
  {:else}
    <div class="lone">{@render hub()}</div>
  {/if}
  {#if dest}
    <button class="end" aria-label={`Stop heading for ${dest.name}`} use:tip={"Stop heading there"} onclick={() => endTrip()}
      ><X size={13} /></button
    >
  {:else if !line}
    <button class="end" aria-label="Pick a place to head for" use:tip={"Pick a place to head for"} onclick={onpick}
      ><Navigation size={12} /></button
    >
  {/if}

  {#if open}
    <div class="drawer" id={drawerId}>
      {#if here && line}
        <p class="d-head">
          Portals on <Ref kind="map" id={here.id} name={here.name} />{#if !reading}<span class="seen"> · last seen</span>{/if}
        </p>
        <ConnectionCompass
          exits={card?.exits ?? []}
          map={here}
          current={reading ? here : null}
          points={picture?.points ?? null}
          {next}
          onnavigate={portalCard}
        />
        <p class="note">Boxes show where each portal stands on the map, from the map data. Leafline doesn't know where you stand.</p>
      {/if}
      {#if dest}
        <p class="d-head route-head">
          To <Ref kind={dest.kind === "npc" ? "npc" : "map"} id={dest.kind === "npc" ? null : dest.maps[0].id} name={dest.name} />{#if dest.kind === "npc" && route?.to}<span
              class="faint">{` · on ${route.to.name}`}</span
            >{/if}
        </p>
        {#if route && route.steps.length}
          <RouteSteps {route} current={trip.next} note={false} />
          <p class="note tabular">
            {route.steps.length} {route.steps.length === 1 ? "step" : "steps"}{#if route.walkSeconds != null && !route.steps.some((s) => s.travel)}, about
              {walkTime(route.walkSeconds)} on foot{/if}{#if fares} · {formatNumber(fares)} mesos in fares{/if}
          </p>
        {:else if trip.lost}
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
      {:else}
        <p class="note">Click a portal for its map, and Go to head there, or <button class="link" onclick={onpick}>pick a place</button> in the Map view.</p>
      {/if}
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
    max-width: 1040px;
    min-width: 0;
    padding: 5px var(--space-1) 5px var(--space-1);
    border: var(--line-w) solid var(--control-line);
    border-radius: var(--radius);
    background: var(--surface);
  }

  .compass.going {
    border-color: color-mix(in srgb, var(--accent) 50%, var(--control-line));
  }

  /* The portals view (the card's layout, or the phone's band) fills the box. */
  .compass > :global(.compass),
  .compass > :global(.strip) {
    flex: 1;
    min-width: 0;
  }

  .lone {
    display: flex;
    flex: 1;
    justify-content: center;
    min-width: 0;
  }

  .hub-btn {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    max-width: 100%;
    padding: 0 var(--space-3);
    border: 0;
    background: none;
    color: var(--text);
    font: inherit;
    line-height: 1.25;
    cursor: pointer;
  }

  .hub-btn:focus-visible,
  .end:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .here-name,
  .sub {
    max-width: 100%;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .here-name {
    color: var(--map);
    font-family: var(--font-display);
    font-size: var(--text-md);
    font-weight: var(--weight-strong);
  }

  .lone .here-name {
    color: var(--muted);
    font-weight: var(--weight-body);
  }

  .seen {
    margin-left: var(--space-1);
    color: var(--warn);
    font-family: var(--font-sans);
    font-size: var(--text-xs);
    font-weight: var(--weight-body);
  }

  .sub {
    display: block;
    color: var(--muted);
    font-size: var(--text-xs);
  }

  .sub :global(svg) {
    margin-right: 3px;
    vertical-align: -1px;
  }

  /* The desk has the height: how far is left goes on a line of its own. */
  .compass:not(.line) .then {
    display: block;
  }

  .sub b {
    color: var(--accent-text-strong);
    font-weight: var(--weight-strong);
  }

  .sub.good {
    color: var(--good);
  }

  .sub.warn {
    color: var(--warn);
  }

  .faint {
    color: var(--faint);
  }

  .chev {
    position: absolute;
    right: 0;
    top: 50%;
    display: grid;
    color: var(--faint);
    transform: translateY(-50%);
    transition: transform 0.14s ease;
  }

  .chev.open {
    transform: translateY(-50%) rotate(180deg);
  }

  .end {
    display: inline-grid;
    flex: none;
    place-items: center;
    width: 24px;
    height: 24px;
    border: var(--line-w) solid var(--control-line);
    border-radius: 50%;
    background: var(--surface);
    color: var(--muted);
    cursor: pointer;
  }

  .end:hover {
    background: var(--surface-2);
    color: var(--text);
  }

  /* The whole view, opened under the compass over the page. */
  .drawer {
    position: absolute;
    top: calc(100% + var(--space-1-5));
    left: 50%;
    z-index: 30;
    width: min(560px, calc(100vw - 2 * var(--space-3)));
    max-height: min(72vh, 620px);
    overflow-y: auto;
    padding: var(--space-3);
    border: var(--line-w) solid var(--line);
    border-radius: var(--radius-lg);
    background: var(--surface);
    box-shadow: var(--card-shadow), 0 12px 32px rgb(0 0 0 / 0.18);
    transform: translateX(-50%);
  }

  .d-head {
    margin: 0 0 var(--space-1);
    font-size: var(--text-md);
  }

  .route-head:not(:first-child) {
    margin-top: var(--space-3);
    padding-top: var(--space-2);
    border-top: var(--line-w) solid var(--line);
  }

  .note {
    margin: var(--space-1-5) 0 0;
    color: var(--faint);
    font-size: var(--text-xs);
    line-height: 1.4;
  }

  .link {
    padding: 0;
    border: 0;
    background: none;
    color: var(--accent-text);
    font: inherit;
    text-decoration: underline;
    cursor: pointer;
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

  /* A phone: the map's street and the arrow labels make way for the names. */
  .line {
    padding: 3px var(--space-1);
  }

  /* Only the arrows say which side; the boxes and words make way for the names. */
  .line :global(.strip .locator) {
    display: none;
  }

  .line :global(.strip .dir) {
    font-size: 0;
  }

  .line :global(.strip .slot.west),
  .line :global(.strip .slot.east),
  .line :global(.strip .slot.doors) {
    flex: 0 1 auto;
  }

  .line :global(.strip .node) {
    min-width: 44px;
    max-width: 92px;
  }

  .line :global(.strip .hub) {
    flex: 1 1 0;
    min-width: 112px;
  }


  .line .hub-btn {
    padding: 0 var(--space-2);
  }
</style>
