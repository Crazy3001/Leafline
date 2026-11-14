// How a route step is put into words, and where on a route a minimap reading puts the player.
// Leafline knows the map the player is on, never where they stand in it, so a step names the
// portal by the data's side of the map, or by its direction, and nothing finer.

import type { Destination, Direction, MapMatch, MapRef, MapRoute, RouteStep, Side } from "./types";

const verbs: Record<Direction, string> = {
  west: "West portal",
  east: "East portal",
  in: "Go in",
  out: "Go out",
  other: "Portal",
};
const sides: Record<Side, string> = { left: "Left portal", centre: "Middle portal", right: "Right portal" };

/** What to do at a hidden portal: the game draws nothing there, so the spot has to be found. */
export const HIDDEN_PORTAL_HINT = "hidden portal: stand on the spot and press ↑";

/** What to take for a portal step: its side when the data places it, otherwise its direction. */
export function stepVerb(step: RouteStep): string {
  return step.side ? sides[step.side] : verbs[step.direction];
}

/**
 * A header this close to a map's name settles it, as `SETTLED_SCORE` does in
 * src-tauri/src/mapdata/atlas.rs. Below it a reading only counts for a map the route expects.
 */
export const TRUSTED_SCORE = 0.85;

/** Where a reading puts the player, given the route being followed. */
export type Position =
  /** On one of the destination's maps. */
  | { kind: "arrived"; at: MapRef }
  /** On the route; `next` is the step to take from here. */
  | { kind: "on-route"; at: MapRef; next: number }
  /** On a map the route doesn't pass through. */
  | { kind: "off-route"; at: MapRef }
  /** A reading too rough to act on: hold the last known map. */
  | { kind: "unsure"; guess: MapRef };

/** The maps a header can mean, the closest first. */
function candidates(match: MapMatch): MapRef[] {
  return [match.card.map, ...match.alternatives];
}

/**
 * Maps sharing the best match's header exactly, such as the two Mushroom Towns on Maple Road.
 * The live loop settles which one when an NPC only one of them has shows; until then the
 * backend lists the others as alternatives with the same name and street.
 */
function twins(match: MapMatch): MapRef[] {
  const best = match.card.map;
  return [best, ...match.alternatives.filter((m) => m.name === best.name && m.street === best.street)];
}

/** Every map on a route, in order, starting where it starts. */
export function routeMaps(route: MapRoute): MapRef[] {
  return [route.from, ...route.steps.map((s) => s.to)];
}

/**
 * Where a minimap reading puts the player. A reading close enough to settle the name means that
 * map, or whichever twin sharing its header the route expects. A rougher one counts only for a
 * map the route expects, the near names included, since a rough read of the next map's name is
 * far likelier to be that map than another; otherwise the last known map holds.
 */
export function locate(match: MapMatch, destination: Destination, route: MapRoute | null): Position {
  const goal = new Set(destination.maps.map((m) => m.id));
  const along = route ? routeMaps(route) : [];
  const expected = (m: MapRef) => goal.has(m.id) || along.some((a) => a.id === m.id);

  const trusted = match.score >= TRUSTED_SCORE;
  // A clear reading of one name rules out the names near it, but not maps sharing it.
  const known = (trusted ? twins(match) : candidates(match)).find(expected);
  if (known) {
    if (goal.has(known.id)) return { kind: "arrived", at: known };
    // The furthest along, should a route pass a map twice.
    return { kind: "on-route", at: known, next: along.map((a) => a.id).lastIndexOf(known.id) };
  }
  if (!trusted) return { kind: "unsure", guess: match.card.map };
  return { kind: "off-route", at: match.card.map };
}

/** The shortest of several routes, by steps; null when there are none. */
export function shortest(routes: (MapRoute | null)[]): MapRoute | null {
  return routes.reduce<MapRoute | null>((best, r) => (r && (!best || r.steps.length < best.steps.length) ? r : best), null);
}

/** What the bar says on arrival. */
export function arrivedLine(destination: Destination, at: MapRef): string {
  return destination.kind === "npc" ? `${destination.name} is on this map` : `You're on ${at.name}`;
}
