// The player's drop log, shared by the item check and the training log. Follows drops logged or
// deleted in any window. Every drop is the player's own word and stays on this PC; reporting one
// to MeowDB is the player's click in their own browser.

import { backend } from "./api";
import type { LoggedDrop } from "./types";

export const drops = $state<{ value: LoggedDrop[] | null }>({ value: null });

let watching = false;

export function watchDrops() {
  if (watching) return;
  watching = true;
  refresh();
  backend.onDrops(refresh);
}

function refresh() {
  backend
    .dropLog()
    .then((d) => (drops.value = d))
    .catch(() => (drops.value = null));
}

/** MeowDB's monster search; its page for the monster has "+ Add another item" for drops. */
export function meowdbMonster(name: string) {
  return `https://meowdb.com/msclassic/monsters?q=${encodeURIComponent(name.trim())}`;
}

/**
 * Opens the monster's MeowDB page in the player's browser and copies the item's name, ready to
 * paste into "+ Add another item". Nothing is sent from Leafline itself.
 */
export async function reportOnMeowdb(monster: string, item: string): Promise<boolean> {
  let copied = false;
  try {
    await navigator.clipboard.writeText(item);
    copied = true;
  } catch {
    // The page still opens; the player types the name.
  }
  await backend.openLink(meowdbMonster(monster));
  return copied;
}

/** Drops logged between two times (Unix seconds), for one character when it is known. */
export function dropsBetween(from: number, to: number, character: string | null = null): LoggedDrop[] {
  return (drops.value ?? []).filter(
    (d) => d.seenAt >= from && d.seenAt <= to && (!character || !d.character || d.character === character),
  );
}

export interface DropTally {
  monster: string;
  monsterId: number | null;
  total: number;
  items: { item: string; count: number; last: number }[];
}

/** Drops grouped by monster, the most logged first, each with its items, the most logged first. */
export function tallyByMonster(list: LoggedDrop[]): DropTally[] {
  const byMonster = new Map<string, DropTally>();
  for (const d of list) {
    const key = d.monster.toLowerCase();
    const row = byMonster.get(key) ?? { monster: d.monster, monsterId: d.monsterId, total: 0, items: [] };
    row.total += 1;
    const item = row.items.find((i) => i.item.toLowerCase() === d.item.toLowerCase());
    if (item) {
      item.count += 1;
      item.last = Math.max(item.last, d.seenAt);
    } else row.items.push({ item: d.item, count: 1, last: d.seenAt });
    byMonster.set(key, row);
  }
  const rows = [...byMonster.values()];
  for (const r of rows) r.items.sort((a, b) => b.count - a.count || b.last - a.last);
  return rows.sort((a, b) => b.total - a.total || a.monster.localeCompare(b.monster));
}

/** How many of this item the player has logged, from any monster. */
export function timesLogged(item: string): number {
  const key = item.trim().toLowerCase();
  return key ? (drops.value ?? []).filter((d) => d.item.toLowerCase() === key).length : 0;
}
