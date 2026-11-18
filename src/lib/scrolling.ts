// The odds of scrolling a piece of gear: every attempt uses a slot whether it passes or not, and
// a failed attempt with a "dark" scroll can destroy the item. Exact, slot by slot, not simulated.
//
// The rules are the community's, not Nexon's: success rates come from the scroll's own data, and
// the 50% chance a failed 30% or 70% scroll destroys the item is the long-standing community
// figure, which Classic may change. Both are editable where the calculator is shown.

export interface ScrollPlan {
  /** Upgrade slots to use. */
  slots: number;
  /** Chance each attempt passes, 0 to 1. */
  rate: number;
  /** Chance a failed attempt destroys the item, 0 to 1. */
  destroy: number;
  /** What one pass adds to the stat. */
  stat: number;
  /** Passes wanted. */
  target: number;
  /** Stop scrolling once the target is reached, keeping the slots left. */
  stopAtTarget: boolean;
}

export interface ScrollOdds {
  /** Chance of reaching the target with the item still there. */
  reach: number;
  /** Chance the item is destroyed. */
  destroyed: number;
  /** Stat the item ends with on average, when it survives. */
  expectedStat: number;
  /** Scrolls used on average. */
  scrollsUsed: number;
  /** Chance of ending with exactly n passes and the item still there, n = 0 to slots. */
  passes: number[];
}

/** The community's figure for how often a failed scroll of this rate destroys the item. */
export function destroyChanceFor(rate: number): number {
  const pct = Math.round(rate * 100);
  return pct === 30 || pct === 70 ? 0.5 : 0;
}

export function scrollOdds(plan: ScrollPlan): ScrollOdds {
  const slots = Math.max(0, Math.floor(plan.slots));
  const rate = Math.min(1, Math.max(0, plan.rate));
  const destroy = Math.min(1, Math.max(0, plan.destroy));
  const target = Math.max(0, Math.floor(plan.target));
  // alive[k]: still scrolling, with k passes. done[k]: stopped at the target with k passes.
  let alive = new Array<number>(slots + 1).fill(0);
  const done = new Array<number>(slots + 1).fill(0);
  alive[0] = 1;
  let destroyed = 0;
  let scrollsUsed = 0;
  for (let slot = 0; slot < slots; slot++) {
    const next = new Array<number>(slots + 1).fill(0);
    for (let k = 0; k <= slot; k++) {
      const p = alive[k];
      if (!p) continue;
      if (plan.stopAtTarget && k >= target) {
        done[k] += p;
        continue;
      }
      scrollsUsed += p;
      next[k + 1] += p * rate;
      next[k] += p * (1 - rate) * (1 - destroy);
      destroyed += p * (1 - rate) * destroy;
    }
    alive = next;
  }
  const passes = alive.map((p, k) => p + done[k]);
  const survived = passes.reduce((a, b) => a + b, 0);
  return {
    reach: passes.reduce((sum, p, k) => (k >= target ? sum + p : sum), 0),
    destroyed,
    expectedStat: survived > 0 ? (passes.reduce((sum, p, k) => sum + p * k, 0) / survived) * plan.stat : 0,
    scrollsUsed,
    passes,
  };
}

/**
 * What one item that reaches the target costs on average: each try buys an item and the scrolls
 * it uses, and only `reach` of the tries get there. Infinity when none can.
 */
export function costPerSuccess(odds: ScrollOdds, scrollPrice: number, itemPrice: number): number {
  if (odds.reach <= 0) return Infinity;
  return (itemPrice + odds.scrollsUsed * scrollPrice) / odds.reach;
}
