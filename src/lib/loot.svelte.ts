// What the loot the Pickup Log showed is worth. Each item is valued at the player's own logged
// price when there is one, else at the NPC price the item data lists, which is what selling it
// to a shop would fetch (what that price means in Classic isn't confirmed). Prices are asked for
// once per item and kept for the rest of the run.

import { backend } from "./api";
import { profitPerHour, potionsPerHour } from "./profit";
import type { LootItem, LootPrice } from "./types";

/** Prices found so far, by item id. */
const prices = $state<{ byId: Record<number, LootPrice> }>({ byId: {} });
const asked = new Set<number>();

/** Asks for the prices of the items not asked about yet. */
export function wantPrices(items: LootItem[]) {
  const ids = items.map((i) => i.id).filter((id) => !asked.has(id));
  if (!ids.length) return;
  ids.forEach((id) => asked.add(id));
  backend
    .lootPrices(ids)
    .then((found) => {
      for (const p of found) prices.byId[p.id] = p;
    })
    .catch(() => ids.forEach((id) => asked.delete(id)));
}

/** Forgets the prices, after the player logs one. */
export function refreshPrices() {
  asked.clear();
  prices.byId = {};
}

export interface ValuedItem extends LootItem {
  /** What one is worth, or null when nothing prices it. */
  each: number | null;
  /** Where that worth comes from. */
  source: "own" | "npc" | null;
  total: number | null;
}

export function valued(items: LootItem[]): ValuedItem[] {
  return items.map((item) => {
    const p = prices.byId[item.id];
    const each = p?.own ?? p?.npc ?? null;
    const source = p?.own != null ? "own" : p?.npc != null ? "npc" : null;
    return { ...item, each, source, total: each != null ? each * item.count : null };
  });
}

/** What the items are worth together; items with no price add nothing. */
export function lootValue(items: LootItem[]): number {
  return valued(items).reduce((sum, i) => sum + (i.total ?? 0), 0);
}

/** Loot value an hour over `seconds` of watching the log. Null before half a minute. */
export function lootPerHour(items: LootItem[], seconds: number): number | null {
  return seconds >= 30 ? (lootValue(items) * 3600) / seconds : null;
}

/** The session's whole profit an hour: mesos, plus loot, less potions. */
export function sessionProfit(o: {
  mesosPerHour: number | null;
  lootPerHour: number | null;
  potionCost: number;
  activeSeconds: number;
}): number | null {
  return profitPerHour({
    mesosPerHour: o.mesosPerHour,
    lootPerHour: o.lootPerHour,
    potionsPerHour: potionsPerHour(o.potionCost, o.activeSeconds),
  });
}
