<script lang="ts">
  import { tip } from "$lib/tip.svelte";
  import Ref from "$lib/components/Ref.svelte";
  // The slim "next step" bar for "Go here", under the overlay's header, in the compact HUD and
  // in the Route pin (which shows the trip the overlay follows, see followTrip):
  // the step to take from the map the player is on, how many follow, and a way to cancel. It
  // moves on by itself as the minimap reading changes (see wayfinder.svelte.ts). Also offers
  // the turn-in place of a quest whose counters just filled. Under the step, outside the HUD:
  // the map the player is on with the portal to take lit up, every step of the trip, and the
  // taxis and scrolls the route may use, the same switches as in Settings.
  import {
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    CarTaxiFront,
    ChevronDown,
    Plane,
    Check,
    DoorOpen,
    ListOrdered,
    LoaderCircle,
    LogIn,
    LogOut,
    Map as MapIcon,
    Navigation,
    Pin,
    ScrollText,
    X,
  } from "@lucide/svelte";

  import GoButton from "./GoButton.svelte";
  import PortalStrip from "./map/PortalStrip.svelte";
  import RouteSteps from "./map/RouteSteps.svelte";
  import { backend } from "$lib/api";
  import { nav } from "$lib/destination.svelte";
  import { formatNumber } from "$lib/format";
  import { live } from "$lib/readings.svelte";
  import { changeSettings, display } from "$lib/settings.svelte";
  import type { Destination, RouteStep, TravelChoice, TravelInfo } from "$lib/types";
  import Switch from "$lib/ui/Switch.svelte";
  import { dismissOffer, endTrip, rerouteTrip, trip } from "$lib/wayfinder.svelte";
  import { HIDDEN_PORTAL_HINT, stepVerb, walkTime } from "$lib/wayfinding";

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
    if (step.travel) return step.travel.kind === "scroll" ? ScrollText : step.travel.kind === "npc" ? Plane : CarTaxiFront;
    if (step.side === "left" || (!step.side && step.direction === "west")) return ArrowLeft;
    if (step.side === "right" || (!step.side && step.direction === "east")) return ArrowRight;
    if (step.side === "centre") return ArrowUp;
    if (step.direction === "in") return LogIn;
    if (step.direction === "out") return LogOut;
    return DoorOpen;
  }
  /** The map the route ends on, which says where an NPC stands. */
  const end = $derived(trip.route?.to ?? null);

  // What opens under the step: the list of steps or the travel switches, one at a time, and
  // whether the map strip shows. Remembered per kind of bar, as a convenience.
  type Drawer = "steps" | "travel" | null;
  function recall<T>(key: string, fallback: T): T {
    try {
      const value = localStorage.getItem(key);
      return value === null ? fallback : (JSON.parse(value) as T);
    } catch {
      return fallback;
    }
  }
  function keep(key: string, value: unknown) {
    try {
      localStorage.setItem(key, JSON.stringify(value));
    } catch {
      // A convenience only.
    }
  }
  // svelte-ignore state_referenced_locally
  const store = `leafline.nextStep.${variant}`;
  let drawer = $state<Drawer>(recall<Drawer>(`${store}.drawer`, null));
  let showMap = $state(recall(`${store}.map`, true));
  function toggle(which: Exclude<Drawer, null>) {
    drawer = drawer === which ? null : which;
    keep(`${store}.drawer`, drawer);
  }
  function toggleMap() {
    showMap = !showMap;
    keep(`${store}.map`, showMap);
  }

  const travelOn = $derived(display.travel.enabled);
  const KINDS: { key: keyof Omit<TravelChoice, "enabled">; label: string; short: string; scroll?: boolean }[] = [
    { key: "cabs", label: "Taxis", short: "Taxis" },
    { key: "vipCabs", label: "VIP Cab", short: "VIP" },
    { key: "scrolls", label: "Return scrolls", short: "Scrolls", scroll: true },
  ];
  /** Which of the kinds are on, said in a few words for the closed button. */
  const travelWords = $derived.by(() => {
    if (!travelOn) return "off";
    const on = KINDS.filter((k) => display.travel[k.key]);
    if (!on.length) return "none picked";
    return on.map((k) => k.short).join(", ");
  });

  let travel = $state<TravelInfo | null>(null);
  let downloading = $state(false);
  let travelError = $state<string | null>(null);
  // The travel drawer says when the taxis and scrolls aren't downloaded yet.
  $effect(() => {
    if (drawer !== "travel" || hud) return;
    backend
      .travelInfo()
      .then((t) => (travel = t))
      .catch(() => (travel = null));
  });
  function setTravel(change: Partial<TravelChoice>) {
    void changeSettings({ travel: { ...display.travel, ...change } });
  }
  async function downloadTravel() {
    downloading = true;
    travelError = null;
    try {
      travel = await backend.downloadTravel();
      // The route was worked out without them: work it out again.
      rerouteTrip();
    } catch (e) {
      travelError = `Couldn't download: ${e}`;
    } finally {
      downloading = false;
    }
  }

  const route = $derived(trip.route);
  /** The step shown on the map strip: a portal to walk through, not a ride or scroll. */
  const stripStep = $derived(step && !step.travel && !hud && showMap && display.showPictures ? step : null);
  const rides = $derived(route?.steps.filter((s) => s.travel).length ?? 0);
  const fares = $derived(route?.steps.reduce((sum, s) => sum + (s.travel?.fare ?? 0), 0) ?? 0);

</script>

{#snippet name(place: Destination)}
  <Ref kind={place.kind === "npc" ? "npc" : "map"} id={place.kind === "npc" ? null : place.maps[0].id} name={place.name} icon={!hud} />
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
          {#if step.to.known}<Ref kind="map" id={step.to.id} name={step.to.name} icon={false} />{:else}<span class="map">{step.to.name}</span>{/if}
          {#if step.travel?.fare}<span class="faint tabular fare">{formatNumber(step.travel.fare)} mesos</span>{/if}
          {#if step.hidden}<span class="hidden-hint">· {HIDDEN_PORTAL_HINT}</span>{/if}
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
            >· {display.travel.enabled ? "some trips need a ship or an NPC" : "taxis and scrolls may find a way"}</span
          >
        {/if}
        {#if trip.unsure && !trip.rerouted}<span class="faint">{" · minimap unclear"}</span>{/if}
      </p>
      {#if step && total > 1 && !hud}
        <!-- The trip so far: steps taken, the one to take now, and those after it. A long trip
             shows one bar filling up instead, as its segments would get too fine to read. -->
        {#if total <= MAX_SEGMENTS}
          <div class="progress" aria-hidden="true">
            {#each { length: total } as _, i}<span class:done={i < trip.next} class:now={i === trip.next}></span>{/each}
          </div>
        {:else}
          <div class="progress whole" aria-hidden="true">
            <span class="fill" style:width="{((trip.next + 0.5) / total) * 100}%"></span>
          </div>
        {/if}
      {/if}
      {#if stripStep}
        <PortalStrip from={stripStep.from} portal={stripStep.portal} to={stripStep.to.id} height={pin ? 96 : 76} />
      {/if}
      {#if !hud}
        <div class="tools">
          {#if route && route.steps.length > 1}
            <button class="tool" class:open={drawer === "steps"} aria-expanded={drawer === "steps"} onclick={() => toggle("steps")}>
              <ListOrdered size={12} /><span>All {route.steps.length} steps</span><span class="chev"><ChevronDown size={11} /></span>
            </button>
          {/if}
          <button
            class="tool"
            class:open={drawer === "travel"}
            class:lit={travelOn}
            aria-expanded={drawer === "travel"}
            use:tip={"Taxis and return scrolls the route may use"}
            onclick={() => toggle("travel")}
          >
            <CarTaxiFront size={12} /><span>Travel: {travelWords}</span><span class="chev"><ChevronDown size={11} /></span>
          </button>
          {#if step && !step.travel && display.showPictures}
            <button
              class="tool square"
              class:lit={showMap}
              aria-pressed={showMap}
              use:tip={showMap ? "Hide the map" : "Show where the portal is on the map"}
              aria-label={showMap ? "Hide the map" : "Show the map"}
              onclick={toggleMap}><MapIcon size={12} /></button
            >
          {/if}
        </div>

        {#if drawer === "steps" && route && route.steps.length > 1}
          <div class="drawer steps">
            <RouteSteps {route} current={trip.next} note={false} />
            <p class="drawer-note tabular">
              {route.steps.length - rides} portal{route.steps.length - rides === 1 ? "" : "s"}{#if rides}, {rides}
                {rides === 1 ? "ride or scroll" : "rides or scrolls"}{/if}{#if route.walkSeconds != null && !rides}
                · about {walkTime(route.walkSeconds)} on foot{/if}{#if fares} · {formatNumber(fares)} mesos in fares{/if}
            </p>
          </div>
        {:else if drawer === "travel"}
          <div class="drawer travel">
            <div class="switch-row">
              <span>
                <span class="name">Taxis and return scrolls</span>
                <span class="help">Routes, training spots and quest hints use them. The same switches as in Settings.</span>
              </span>
              <Switch checked={travelOn} label="Taxis and return scrolls" onchange={(on) => setTravel({ enabled: on })} />
            </div>
            <div class="kinds" class:off={!travelOn}>
              {#each KINDS as kind (kind.key)}
                <label class="kind">
                  <input
                    type="checkbox"
                    disabled={!travelOn}
                    checked={display.travel[kind.key]}
                    onchange={(e) => setTravel({ [kind.key]: e.currentTarget.checked })}
                  />
                  {#if kind.scroll}<ScrollText size={12} />{:else}<CarTaxiFront size={12} />{/if}
                  {kind.label}
                </label>
              {/each}
            </div>
            {#if travelOn && travel?.origin === "none"}
              <p class="drawer-note warn-line">
                Not downloaded yet, so routes can't use them.
                <button class="get" disabled={downloading} onclick={downloadTravel}>
                  {#if downloading}<span class="spin"><LoaderCircle size={11} /></span>Downloading…{:else}Download them{/if}
                </button>
              </p>
            {/if}
            {#if travelError}<p class="drawer-note warn-line">{travelError}</p>{/if}
          </div>
        {/if}
      {/if}
    </div>
    {#if canPin}
      <button class="close" use:tip={"Pin the route"} aria-label="Pin the route" onclick={pinRoute}
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

  /* The step's text, the strip and the drawers grow downwards: the tiles stay at the top. */
  .trip:not(.hud) {
    align-items: flex-start;
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
    color: var(--accent-text-strong);
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

  .progress.whole {
    background: var(--border-strong);
  }

  .progress.whole .fill {
    flex: none;
    background: var(--accent-strong);
  }

  /* The row of small buttons under the step, and what each opens. */
  .tools {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-top: 6px;
  }

  .tool {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 22px;
    padding: 0 6px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--bg) 35%, transparent);
    color: var(--muted);
    font-size: var(--text-xs);
    cursor: pointer;
    transition:
      color 0.12s ease,
      border-color 0.12s ease,
      background 0.12s ease;
  }

  .tool.square {
    justify-content: center;
    width: 22px;
    padding: 0;
  }

  .tool:hover,
  .tool:focus-visible {
    border-color: color-mix(in srgb, var(--accent) 45%, var(--border-strong));
    color: var(--text);
    outline: none;
  }

  .tool.lit {
    color: var(--travel);
  }

  .tool.square.lit {
    color: var(--accent-text-strong);
  }

  .tool.open {
    border-color: color-mix(in srgb, var(--accent) 55%, transparent);
    background: var(--accent-soft);
    color: var(--text);
  }

  .chev {
    display: inline-grid;
    color: var(--faint);
    transition: transform 0.14s ease;
  }

  .tool.open .chev {
    transform: rotate(180deg);
  }

  .drawer {
    margin-top: 6px;
    padding: 6px 8px 7px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--bg) 45%, transparent);
    font-weight: 400;
  }

  /* A long trip scrolls inside the drawer rather than pushing the panels off the window. */
  .drawer.steps {
    max-height: 230px;
    overflow-y: auto;
  }

  .pin .drawer.steps {
    max-height: 320px;
  }

  .drawer.steps :global(.steps) {
    margin-top: 0;
  }

  .drawer-note {
    margin: 4px 0 0;
    color: var(--faint);
    font-size: var(--text-xs);
  }

  .warn-line {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    color: var(--warn);
  }

  .switch-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }

  .switch-row .name {
    display: block;
    color: var(--text);
    font-size: var(--text-sm);
    font-weight: 500;
  }

  .switch-row .help {
    display: block;
    color: var(--faint);
    font-size: var(--text-xs);
    line-height: 1.35;
  }

  .kinds {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 12px;
    margin-top: 6px;
  }

  .kinds.off {
    opacity: 0.5;
  }

  .kind {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    color: var(--muted);
    font-size: var(--text-xs);
    cursor: pointer;
  }

  .kinds.off .kind {
    cursor: default;
  }

  .kind input {
    margin: 0;
    accent-color: var(--accent);
  }

  .get {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 8px;
    border: 1px solid color-mix(in srgb, var(--accent) 55%, transparent);
    border-radius: var(--radius-sm);
    background: var(--accent-soft);
    color: var(--accent-text-strong);
    font-size: var(--text-xs);
    cursor: pointer;
  }

  .get:disabled {
    cursor: default;
    opacity: 0.7;
  }

  .spin {
    display: inline-grid;
    animation: spin 0.9s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
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
    color: var(--accent-text);
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
    color: var(--accent-text-strong);
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
    color: var(--accent-text);
    font-weight: 400;
  }

  .map {
    color: var(--text);
  }

  .fare {
    font-size: var(--text-xs);
    font-weight: 400;
  }

  .hidden-hint {
    color: var(--warn);
    font-size: var(--text-xs);
    font-weight: 400;
  }

  .rest {
    margin-top: 1px;
    color: var(--muted);
    font-size: var(--text-xs);
  }

  .rest :global(.ref) {
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
    background: var(--hover);
    color: var(--text);
    outline: none;
  }
</style>
