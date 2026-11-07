// Follows the way to the place set with "Go here", from the live map reading. Runs in the
// overlay window only (the compact HUD is the same window), so one place reports the arrival.
// Also offers the turn-in place when a tracked quest's counters fill, without setting it.

import { untrack } from "svelte";

import { backend } from "./api";
import { cancelTrip, nav, npcPlace, samePlace, watchDestination } from "./destination.svelte";
import { objectiveProgress } from "./format";
import { mapData, watchMapData } from "./mapdata.svelte";
import { live } from "./readings.svelte";
import { travelKey } from "./settings.svelte";
import type { Destination, MapMatch, MapReading, MapRef, MapRoute, Quest } from "./types";
import { TRUSTED_SCORE, arrivedLine, locate, shortest } from "./wayfinding";

/** How long the bar says "you're there" or "new route" before going back to normal. */
const ARRIVED_MS = 8000;
const REROUTED_MS = 6000;
/** Readings in a row a map off the route must be seen in before the route starts again from it. */
const OFF_ROUTE_READINGS = 2;

export const trip = $state<{
  /** The map the player was last read on, held through a missed or rough reading. */
  here: MapRef | null;
  /** The last reading was too rough to trust, so `here` may be out of date. */
  unsure: boolean;
  /** The way from where the route was worked out; `next` is the step to take now. */
  route: MapRoute | null;
  next: number;
  /** Working a route out. */
  routing: boolean;
  /** No way there is known from `here`. */
  lost: boolean;
  /** The route was just worked out again after a turn off it. */
  rerouted: boolean;
  /** Said for a moment after arriving, once the destination has cleared. */
  arrived: string | null;
  /** A quest whose counters just filled, and where to hand it in: offered, never set unasked. */
  offer: { title: string; quest: string; destination: Destination; giver: boolean } | null;
}>({
  here: null,
  unsure: false,
  route: null,
  next: 0,
  routing: false,
  lost: false,
  rerouted: false,
  arrived: null,
  offer: null,
});

let started = false;
let arrivedTimer: ReturnType<typeof setTimeout> | undefined;
let reroutedTimer: ReturnType<typeof setTimeout> | undefined;

/** The last header looked up, so the same text read again isn't looked up again. */
let lastKey = "";
let lastMatch: MapMatch | null = null;
/** A map off the route and how many readings in a row have shown it. */
let straying: { id: number; count: number } | null = null;
/** Bumped with each route asked for, so a slow answer for an old trip is dropped. */
let routeAsk = 0;
/** The map the route (or the search that found none) was last worked out from. */
let triedFrom: number | null = null;
/** What the route was worked out for, to know when to work it out again. */
let routedFor = "";

/** Starts following the live readings. Call once, from the overlay. */
export function startWayfinder() {
  if (started) return;
  started = true;
  watchDestination();
  watchMapData();
  $effect.root(() => {
    // Each live reading is a new object; the header text, and the data it is looked up in,
    // decide whether to look it up again.
    $effect(() => {
      const map = live.analysis?.map ?? null;
      const data = `${mapData.status?.ready}${mapData.status?.dataUpdatedAt}`;
      untrack(() => void onReading(map, data));
    });
    // A new destination, or travel turned on or off, works the route out again from here.
    $effect(() => {
      const key = JSON.stringify([nav.destination, travelKey()]);
      if (key === routedFor) return;
      routedFor = key;
      untrack(startTrip);
    });
    $effect(() => {
      const quests = live.analysis?.quest?.quests ?? null;
      const map = live.analysis?.map ?? null;
      untrack(() => void offerTurnIns(quests, map));
    });
  });
}

/** A new destination, or none: forget the old route and work out the new one from here. */
function startTrip() {
  routeAsk += 1;
  trip.routing = false;
  straying = null;
  triedFrom = null;
  trip.lost = false;
  trip.rerouted = false;
  trip.route = null;
  if (!nav.destination) return;
  clearTimeout(arrivedTimer);
  trip.arrived = null;
  if (trip.offer && samePlace(trip.offer.destination, nav.destination)) trip.offer = null;
  // Already there: say so at once.
  if (trip.here && lastMatch && !trip.unsure) {
    const at = locate(lastMatch, nav.destination, null);
    if (at.kind === "arrived") return arrive(nav.destination, at.at);
  }
  void routeFrom(trip.here, false);
}

async function onReading(map: MapReading | null, data: string) {
  // No minimap in view (a map loading, a window over it): hold what was known.
  if (!map?.name) return;
  const key = JSON.stringify([map.name, map.street, map.twin, data]);
  if (key !== lastKey) {
    lastKey = key;
    lastMatch = await backend.findMap(map.name, map.street, map.twin).catch(() => null);
    if (key !== lastKey) return;
  }
  const match = lastMatch;
  if (!match) return;
  const destination = nav.destination;
  if (!destination) {
    // Nowhere to go: still keep track, so a destination set later starts from here.
    if (match.score >= TRUSTED_SCORE) {
      trip.here = match.card.map;
      trip.unsure = false;
    } else trip.unsure = true;
    return;
  }

  const at = locate(match, destination, trip.route);
  if (at.kind === "unsure") {
    trip.unsure = true;
    // With no route yet there is nothing to hold, so start from the best guess.
    if (!trip.route && !trip.routing && !trip.here) {
      trip.here = at.guess;
      void routeFrom(at.guess, false);
    }
    return;
  }
  trip.unsure = false;
  trip.here = at.at;
  if (at.kind === "arrived") {
    straying = null;
    return arrive(destination, at.at);
  }
  if (at.kind === "on-route") {
    straying = null;
    trip.next = at.next;
    // Moving along the new route: "new route from" has said what it had to.
    if (at.next > 0) trip.rerouted = false;
    return;
  }
  // Off the route. Already worked out from here (or found no way from here): nothing new.
  if (triedFrom === at.at.id) return;
  // Nothing to follow yet: a route from the first clear reading needs no second look.
  if (!trip.route) return void routeFrom(at.at, false);
  // A turn off the route: once the same map has read twice, work the way out from it.
  straying = straying?.id === at.at.id ? { id: at.at.id, count: straying.count + 1 } : { id: at.at.id, count: 1 };
  if (straying.count < OFF_ROUTE_READINGS) return;
  straying = null;
  void routeFrom(at.at, true);
}

/** Works out the way from a map to the nearest of the destination's maps. */
async function routeFrom(from: MapRef | null, afterWrongTurn: boolean) {
  const destination = nav.destination;
  if (!destination || !from) return;
  const ask = ++routeAsk;
  triedFrom = from.id;
  trip.routing = true;
  try {
    const routes = await Promise.all(destination.maps.map((m) => backend.mapRoute(from.id, m.id).catch(() => null)));
    if (ask !== routeAsk || !samePlace(destination, nav.destination)) return;
    const best = shortest(routes);
    trip.route = best;
    trip.next = 0;
    trip.lost = best == null;
    if (best && best.steps.length === 0) return arrive(destination, from);
    clearTimeout(reroutedTimer);
    trip.rerouted = afterWrongTurn && best != null;
    if (trip.rerouted) reroutedTimer = setTimeout(() => (trip.rerouted = false), REROUTED_MS);
  } finally {
    if (ask === routeAsk) trip.routing = false;
  }
}

function arrive(destination: Destination, at: MapRef) {
  trip.route = null;
  trip.lost = false;
  trip.rerouted = false;
  trip.arrived = arrivedLine(destination, at);
  clearTimeout(arrivedTimer);
  arrivedTimer = setTimeout(() => (trip.arrived = null), ARRIVED_MS);
  void backend.arriveAt(destination).then((n) => Object.assign(nav, n));
}

/** Cancels the trip and anything the bar was saying about it. */
export function endTrip() {
  trip.arrived = null;
  return cancelTrip();
}

export function dismissOffer() {
  trip.offer = null;
}

// Quest titles seen with counters still to fill, and ones already offered, this run. Only a
// quest seen unfinished and then finished counts as just finished: one already done when
// Leafline started isn't news, and a counter that reads low for a moment mustn't offer twice.
const open = new Set<string>();
const offered = new Set<string>();

const finished = (q: Quest) => q.objectives.length > 0 && q.objectives.every((o) => objectiveProgress(o) >= 1 && !o.unread);

async function offerTurnIns(quests: Quest[] | null, map: MapReading | null) {
  if (!quests) return;
  const titles = new Set(quests.map((q) => q.title));
  // Handed in, or no longer tracked: the offer goes with it.
  if (trip.offer && !titles.has(trip.offer.title)) trip.offer = null;
  for (const quest of quests) {
    if (!quest.title) continue;
    if (!finished(quest)) {
      open.add(quest.title);
      continue;
    }
    if (!open.has(quest.title) || offered.has(quest.title)) continue;
    offered.add(quest.title);
    const guide = await backend
      .questGuide([{ title: quest.title, objectives: quest.objectives.map((o) => o.target) }], map?.name ?? null, map?.street ?? null, map?.twin ?? null, [])
      .catch(() => null);
    const info = guide?.quests[0]?.quest;
    if (!info) continue;
    const place = info.turnIn ? npcPlace(info.turnIn, info.turnInMaps) : info.giver ? npcPlace(info.giver, info.giverMaps) : null;
    if (!place || samePlace(place, nav.destination)) continue;
    trip.offer = { title: quest.title, quest: info.quest.name, destination: place, giver: !info.turnIn };
  }
}

