// "Go here": the one place the player is heading for, shared by the overlay, the compact HUD and
// pinned panels through the backend, with the places headed for lately. Any window can set or
// cancel it; the overlay follows the way there (see wayfinder.svelte.ts).

import { backend } from "./api";
import type { Destination, MapRef, NavState } from "./types";

export const nav = $state<NavState>({ destination: null, recent: [] });

let watching = false;

export function watchDestination() {
  if (watching) return;
  watching = true;
  backend.destination().then((n) => Object.assign(nav, n));
  backend.onDestination((n) => Object.assign(nav, n));
}

/** A map to head for. */
export function mapPlace(map: MapRef): Destination {
  return { kind: "map", name: map.name, maps: [map] };
}

/** An NPC to head for, on any of the maps they stand on; null when the data puts them nowhere. */
export function npcPlace(name: string, maps: MapRef[]): Destination | null {
  const known = maps.filter((m) => m.known);
  return known.length ? { kind: "npc", name, maps: known } : null;
}

/** Whether two places are the same, whatever order their maps came in. */
export function samePlace(a: Destination | null, b: Destination | null): boolean {
  if (!a || !b) return false;
  const ids = (d: Destination) => d.maps.map((m) => m.id).sort((x, y) => x - y).join();
  return a.kind === b.kind && a.name.toLowerCase() === b.name.toLowerCase() && ids(a) === ids(b);
}

export function isHeadingFor(place: Destination | null): boolean {
  return samePlace(nav.destination, place);
}

export async function goTo(place: Destination) {
  Object.assign(nav, await backend.setDestination(place));
}

export async function cancelTrip() {
  Object.assign(nav, await backend.setDestination(null));
}

export async function forgetPlace(place: Destination) {
  Object.assign(nav, await backend.forgetDestination(place));
}

/** Sets a place, or cancels it when it is already the one set. */
export function toggleGo(place: Destination) {
  return isHeadingFor(place) ? cancelTrip() : goTo(place);
}
