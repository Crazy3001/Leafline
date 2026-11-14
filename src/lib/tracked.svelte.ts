// Items the player is hunting: starred on an item's page, listed in Lookup → Items with who drops
// each one and where. Kept in this PC's storage, which every Leafline window shares, per
// character like followed quests.

import { following } from "./quests.svelte";

export interface TrackedItem {
  id: number;
  name: string;
  /** Whose list it's on, when a character was read; null shows it for everyone. */
  character: string | null;
}

const KEY = "leafline.items.tracked";

function stored(): TrackedItem[] {
  try {
    const value = localStorage.getItem(KEY);
    return value == null ? [] : (JSON.parse(value) as TrackedItem[]);
  } catch {
    return [];
  }
}

function save() {
  try {
    localStorage.setItem(KEY, JSON.stringify(tracking.list));
  } catch {
    // Tracking is a convenience; without storage it lasts until the window closes.
  }
}

export const tracking = $state<{ list: TrackedItem[] }>({ list: stored() });

const mine = (t: TrackedItem) => t.character == null || following.character == null || t.character === following.character;

/** The items the current character tracks, the newest first. */
export function trackedItems(): TrackedItem[] {
  return tracking.list.filter(mine).reverse();
}

export function isTracked(id: number): boolean {
  return tracking.list.some((t) => t.id === id && mine(t));
}

export function track(id: number, name: string) {
  if (isTracked(id)) return;
  tracking.list = [...tracking.list, { id, name, character: following.character }];
  save();
}

export function untrack(id: number) {
  tracking.list = tracking.list.filter((t) => !(t.id === id && mine(t)));
  save();
}

let watching = false;

/** Follows stars added or removed in another window. */
export function watchTracked() {
  if (watching) return;
  watching = true;
  window.addEventListener("storage", (e) => {
    if (e.key === KEY) tracking.list = stored();
  });
}

/**
 * The first `keep` maps in the order given (closest first), with the one holding the most in
 * place of the last when it wasn't among them, as the quest guide shortens its lists.
 */
export function nearestAndRichest<T extends { count: number }>(maps: T[], keep: number): T[] {
  if (maps.length <= keep) return maps;
  const richest = maps.reduce((best, m, i) => (m.count > maps[best].count ? i : best), 0);
  const kept = maps.slice(0, keep);
  if (richest >= keep) kept[keep - 1] = maps[richest];
  return kept;
}
