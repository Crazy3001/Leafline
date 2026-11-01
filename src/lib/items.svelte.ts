// The item database as the Item tab shows it. It lives here so that switching tabs keeps a search,
// and so a link from another window or panel can bring an item up.

import { backend } from "./api";
import type { ItemQuery, ItemRecord, ItemTooltip } from "./types";

export const JOBS = ["Warrior", "Magician", "Bowman", "Thief"];
/** The job flags a tooltip shows, in its order. */
const FLAGS = ["BEGINNER", "WARRIOR", "MAGICIAN", "BOWMAN", "THIEF"];

export const blankQuery = (): ItemQuery => ({
  text: "",
  kind: null,
  slot: null,
  class: null,
  levelMin: null,
  levelMax: null,
  sort: null,
  mine: false,
  offset: 0,
});

export const itemView = $state<{ id: number | null; query: ItemQuery; seen: number }>({
  /** The item open from the database. */
  id: null,
  query: blankQuery(),
  /** Goes up with each link followed, so the overlay can open the tab. */
  seen: 0,
});

let watching = false;

export function watchItems() {
  if (watching) return;
  watching = true;
  backend.onShowItem((link) => {
    itemView.id = link.id;
    if (link.text != null) itemView.query = { ...blankQuery(), text: link.text };
    itemView.seen += 1;
  });
  // A new price check takes over from whatever was open.
  backend.onItemRead((read) => {
    if (read.found) {
      itemView.id = null;
      itemView.query = blankQuery();
    }
  });
}

/** Brings an item up by name, or searches for the name if no item goes by it. */
export async function showItemNamed(name: string) {
  const item = await backend.findItem(name, null);
  await backend.showItem(item?.id ?? null, item ? null : name);
}

/** An item from the data in the shape the card uses for one read from a tooltip. */
export function tooltipOf(item: ItemRecord): ItemTooltip {
  return {
    name: item.name,
    upgrades: null,
    upgraded: false,
    itemType: item.weaponType ?? itemSlot(item),
    requirements: {
      level: item.reqLevel,
      str: item.reqStr,
      dex: item.reqDex,
      int: item.reqInt,
      luk: item.reqLuk,
      fam: item.reqFame,
    },
    jobs:
      item.kind === "equip"
        ? FLAGS.map((job) => ({
            job,
            eligible: item.classes.length === 0 || item.classes.some((c) => c.toUpperCase() === job),
          }))
        : [],
    stats: item.stats.map((s) => ({ ...s, base: null, bonus: null })),
    properties: [],
    remainingEnhancements: null,
    panel: { x: 0, y: 0, w: 0, h: 0 },
    plain: true,
  };
}

/** The item's slot, or its group for items that have none. */
export function itemSlot(item: ItemRecord): string {
  const groups = { equip: item.slot, use: "Use", scroll: "Scroll", setup: "Set-up", etc: "Etc" };
  return groups[item.kind];
}

/** A line of the item's description, the part that says what it does. */
export function lastLine(item: ItemRecord): string {
  return item.description?.split("\n").filter(Boolean).at(-1) ?? "";
}

/** The first few stats an item gives. */
export const gives = (item: ItemRecord, count = 2) =>
  item.stats
    .slice(0, count)
    .map((s) => `${s.label} ${s.value >= 0 ? "+" : ""}${s.value}`)
    .join(", ");
