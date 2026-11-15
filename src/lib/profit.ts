// Profit per hour: mesos picked up, plus what the loot is worth, less the potions drunk. Each
// part is per hour of active play, so parts measured over different stretches still add up.
//
// Potions come from the counts the player types, or reads from the quick slots. Loot value is
// left for the pickup reader (its own branch): once it lands, it fills `lootPerHour` and nothing
// here changes shape.

export interface ProfitParts {
  /** Mesos an hour, over the time the mesos were seen. Null when they were never seen twice. */
  mesosPerHour: number | null;
  /** What the loot picked up is worth an hour, at NPC prices. Null until something reads it. */
  lootPerHour?: number | null;
  /** What the potions drunk cost an hour. Null when no counts were kept. */
  potionsPerHour?: number | null;
}

/** A session's potion cost spread over its whole active time: potions are drunk while hunting,
 *  whether or not the mesos were in view. Null with no cost or no time. */
export function potionsPerHour(cost: number, activeSeconds: number): number | null {
  return cost > 0 && activeSeconds > 0 ? (cost * 3600) / activeSeconds : null;
}

/** Mesos plus loot less potions, an hour. Null when neither mesos nor loot was measured. */
export function profitPerHour(p: ProfitParts): number | null {
  if (p.mesosPerHour == null && p.lootPerHour == null) return null;
  return (p.mesosPerHour ?? 0) + (p.lootPerHour ?? 0) - (p.potionsPerHour ?? 0);
}
