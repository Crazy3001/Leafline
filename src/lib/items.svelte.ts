// The item database as the Lookup tab shows it. It lives here so that switching tabs keeps a search,
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
  stat: null,
  chance: null,
  offset: 0,
});

/** Which part of the Lookup tab is showing. */
export const lookup = $state<{ mode: "items" | "monsters" | "maps" | "npcs" | "crafting" }>({ mode: "items" });

const GENDER_KEY = "leafline.gear.gender";

function storedGender() {
  try {
    return localStorage.getItem(GENDER_KEY);
  } catch {
    return null;
  }
}

/** Who the Equip list is for. Nothing here is set until the player changes it. */
export const gearView = $state<{ forMe: boolean; level: number | null; job: number | null; gender: string | null; ahead: number }>({
  forMe: true,
  level: null,
  job: null,
  gender: storedGender(),
  /** Levels past the character's own to look at. */
  ahead: 0,
});

/** The game doesn't show a character's gender, so the player's choice is remembered. */
export function setGender(gender: string | null) {
  gearView.gender = gender;
  try {
    if (gender) localStorage.setItem(GENDER_KEY, gender);
    else localStorage.removeItem(GENDER_KEY);
  } catch {
    // A convenience only.
  }
}

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
    lookup.mode = "items";
    itemView.seen += 1;
  });
  // A new item check takes over from whatever was open.
  backend.onItemRead((read) => {
    if (read.found) {
      itemView.id = null;
      itemView.query = blankQuery();
      lookup.mode = "items";
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

/** Tooltip stat names that the item data spells another way. */
const TOOLTIP_LABELS: Record<string, string> = { Attack: "Weapon Attack", Magic: "Magic Attack" };
const DATA_LABELS: Record<string, string> = { "Weapon Attack": "Attack", "Magic Attack": "Magic" };

export interface StatRow {
  label: string;
  value: number;
  /** For a stat that shows gear or scrolls apart: the item's own part and what was added. */
  base: number | null;
  bonus: number | null;
  /** The data's value for this stat when the read one differs: a scroll or a rolled copy, or a misread. */
  standard: number | null;
  /** Not read from the tooltip: the data's value stands in. */
  fromData: boolean;
}

/**
 * A read copy's stats beside the item's own. What the tooltip showed stands, since scrolls and
 * rolls change it, but where it differs from the data's value that is said, and a stat the read
 * missed is filled in from the data so a misread doesn't drop it. Without a match, only what was read.
 */
export function statRows(read: ItemTooltip, matched: ItemRecord | null): StatRow[] {
  const defaults = new Map((matched?.stats ?? []).map((s) => [s.label, s.value]));
  const rows: StatRow[] = read.stats.map((s) => {
    const standard = defaults.get(DATA_LABELS[s.label] ?? s.label);
    return { ...s, standard: standard !== undefined && standard !== s.value ? standard : null, fromData: false };
  });
  if (matched) {
    const seen = new Set(read.stats.map((s) => DATA_LABELS[s.label] ?? s.label));
    for (const s of matched.stats) {
      if (!seen.has(s.label)) {
        rows.push({ label: TOOLTIP_LABELS[s.label] ?? s.label, value: s.value, base: null, bonus: null, standard: null, fromData: true });
      }
    }
  }
  return rows;
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
