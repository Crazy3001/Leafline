// Sell or keep: the last bag check, and what says to keep each item. The backend names the items
// by their pictures and says what the data and the build want kept; here the player's own
// tracked items and the quests they are doing are added.

import { backend } from "./api";
import { characters } from "./characters.svelte";
import { live } from "./readings.svelte";
import { followedQuests } from "./quests.svelte";
import { isTracked } from "./tracked.svelte";
import type { BagCheck, BagItem, BagTab } from "./types";

export const bag = $state<{
  /** The check made this run. */
  check: BagCheck | null;
  seq: number;
  /** The last check of each tab kept for the character looked at, from earlier runs, newest first. */
  known: BagCheck[];
  /** The kept tab picked to look at; the newest with none picked. */
  knownTab: BagTab | null;
  /** The player closed the kept one. */
  knownClosed: boolean;
}>({ check: null, seq: 0, known: [], knownTab: null, knownClosed: false });

const same = (a: string | null | undefined, b: string | null | undefined) =>
  a != null && b != null && a.trim().toLowerCase() === b.trim().toLowerCase();

/**
 * The check to show: this run's, unless it is another character's than the one looked at; else the
 * one kept for them, which says when it was read.
 */
export function shownBag(): { check: BagCheck; kept: boolean } | null {
  const active = characters.active;
  const mine = bag.check && (bag.check.busy || !bag.check.character || !active || same(bag.check.character, active));
  if (bag.check && mine) return { check: bag.check, kept: false };
  const kept = keptBag();
  if (kept && !bag.knownClosed) return { check: kept, kept: true };
  return null;
}

/** The kept check of the tab picked, else the newest kept. */
export function keptBag(): BagCheck | null {
  return bag.known.find((c) => c.tab === bag.knownTab) ?? bag.known[0] ?? null;
}

let asked = 0;

function loadKnown(name: string | null) {
  const ask = ++asked;
  backend
    .knownBags(name)
    .then((checks) => {
      if (ask === asked) bag.known = checks;
    })
    .catch(() => {
      if (ask === asked) bag.known = [];
    });
}

let watching = false;

export function watchBag() {
  if (watching) return;
  watching = true;
  $effect.root(() => {
    $effect(() => {
      const name = characters.active;
      bag.knownClosed = false;
      bag.knownTab = null;
      loadKnown(name);
    });
  });
  void backend.onKnownCharacter(() => loadKnown(characters.active));
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
