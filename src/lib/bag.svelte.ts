// Sell or keep: the last bag check, and what says to keep each item. The backend names the items
// by their pictures and says what the data and the build want kept; here the player's own
// tracked items and the quests they are doing are added.

import { backend } from "./api";
import { live } from "./readings.svelte";
import { followedQuests } from "./quests.svelte";
import { isTracked } from "./tracked.svelte";
import type { BagCheck, BagItem } from "./types";

export const bag = $state<{ check: BagCheck | null; seq: number }>({ check: null, seq: 0 });

let watching = false;

export function watchBag() {
  if (watching) return;
  watching = true;
  backend
    .lastBag()
    .then((check) => {
      if (check && !bag.check) bag.check = check;
    })
    .catch(() => {});
  backend.onBag((check) => {
    bag.check = check;
    // A finished check brings the Item tab up; one in progress doesn't.
    if (!check.busy) bag.seq += 1;
  });
}

/** Why an item is kept, in a few words each, or none to sell it. */
export function keepReasons(item: BagItem): string[] {
  const out: string[] = [];
  const name = item.name.toLowerCase();
  const helper = (live.analysis?.quest?.quests ?? []).find((q) => q.objectives.some((o) => o.target.toLowerCase() === name));
  const followed = followedQuests().find((f) => item.wantedBy.some((w) => w.id === f.id));
  if (helper) out.push(`for ${helper.title}`);
  else if (followed) out.push(`for ${followed.name}`);
  if (item.ids.some((id) => isTracked(id))) out.push("tracked");
  for (const k of item.keep) {
    if (k.kind === "recipe") out.push("crafting uses it");
    if (k.kind === "plan") out.push("your build wears it");
    if (k.kind === "quest") out.push("a quest item");
    if (k.kind === "scroll") out.push("scrolls sell better to players");
  }
  return out;
}

/** What one stack fetches from an NPC: the price, or the low end when the look-alikes differ. */
export function stackValue(item: BagItem): number | null {
  const each = item.price ?? item.priceRange?.[0] ?? null;
  return each == null ? null : each * item.count;
}
