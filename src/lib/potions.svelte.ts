// The sum behind
// mesos per hour after potion costs. Names and prices are the player's own; nothing is bundled.

import { potionsPerHour, profitPerHour } from "./profit";

/** The quick-slot bar's slots by their default keys, top row first. The keys can be changed in
 *  the game, so the position is what counts. */
export const QUICK_SLOTS = ["Shift", "Ins", "Home", "PgUp", "Ctrl", "Del", "End", "PgDn"].map(
  (key, i) => `${i < 4 ? "Top" : "Bottom"} ${(i % 4) + 1} · ${key}`,
);

/** As many slots as the backend keeps. */
export const MAX_SLOTS = 8;

/**
 * Mesos per hour with the potions' cost taken out. The cost is spread over the whole active
 * time, since potions are drunk while hunting whether or not the mesos were in view.
 */
export function netMesosPerHour(mesosPerHour: number | null, cost: number, activeSeconds: number): number | null {
  if (mesosPerHour == null || activeSeconds <= 0) return null;
  return profitPerHour({ mesosPerHour, potionsPerHour: potionsPerHour(cost, activeSeconds) });
}
