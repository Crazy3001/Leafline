// The sum behind
// mesos per hour after potion costs. Names and prices are the player's own; nothing is bundled.

/** As many slots as the backend keeps. */
export const MAX_SLOTS = 8;

/**
 * Mesos per hour with the potions' cost taken out. The cost is spread over the whole active
 * time, since potions are drunk while hunting whether or not the mesos were in view.
 */
export function netMesosPerHour(mesosPerHour: number | null, cost: number, activeSeconds: number): number | null {
  if (mesosPerHour == null || activeSeconds <= 0) return null;
  return mesosPerHour - (cost * 3600) / activeSeconds;
}
