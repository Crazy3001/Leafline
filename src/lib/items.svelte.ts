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

/** A name with its upgrade count, case and punctuation gone, for comparing a read name to the data's. */
const nameKey = (name: string) =>
  name
    .replace(/\(\s*\+\s*\d+\s*\)?\s*$/, "")
    .toLowerCase()
    .replace(/[^a-z0-9]/g, "");

/**
 * Whether a name read from the game is the data's item: "exact" when the names agree and the
 * level, if one was read, does too; "close" when the data's item is only the nearest guess.
 */
export function nameMatch(read: ItemTooltip, record: ItemRecord): "exact" | "close" {
  const sameName = nameKey(read.name) === nameKey(record.name);
  const level = read.requirements.level;
  return sameName && (level == null || level === record.reqLevel) ? "exact" : "close";
}

/** One line of the item check: a value to show, how far to trust it, and why when it isn't sure. */
export interface CheckedStat {
  label: string;
  value: number;
  base: number | null;
  bonus: number | null;
  /** The data's value, shown beside a read one that differs. */
  standard: number | null;
  /** Where the value came from. */
  source: "read" | "data";
  /** Why the value is the data's or doubtful, when it is. */
  note: string | null;
}

export interface CheckedItem {
  stats: CheckedStat[];
  /** Read stats that are not in the data and could not be confirmed. */
  unconfirmed: string[];
  /** Slots left, when the read is believable. */
  slotsLeft: number | null;
  slotsNote: string | null;
}

/** The most one successful scroll adds to a single stat. A guess at the generous end, so a real value isn't thrown out. */
const PER_SCROLL = 5;

/** How far a rolled copy's stat can sit from the data's value and still be believed as read. */
const roll = (standard: number) => Math.max(2, Math.round(Math.abs(standard) * 0.2));

/**
 * The data's stats are the baseline. A read value replaces one only when it can be believed: it
 * shows the item's own value as its base and the bonus adds up, or it sits within a roll of the
 * data's. Anything else keeps the data's value with a note. A read stat the data lacks is shown
 * only when it is a scroll's bonus on an item with scrolls; otherwise it is listed as unconfirmed.
 */
export function checkItem(read: ItemTooltip, record: ItemRecord): CheckedItem {
  const readFor = (label: string) => read.stats.find((s) => (DATA_LABELS[s.label] ?? s.label) === label);
  const stats: CheckedStat[] = record.stats.map((own) => {
    const label = TOOLTIP_LABELS[own.label] ?? own.label;
    const got = readFor(own.label);
    const fromData = (note: string | null): CheckedStat => ({ label, value: own.value, base: null, bonus: null, standard: null, source: "data", note });
    if (!got) return fromData(null);
    const split = got.base != null && got.bonus != null && got.base + got.bonus === got.value;
    const tolerance = roll(own.value);
    // Every successful scroll can add several points, so a scrolled copy may sit well above the item's value.
    // The (+N) after the name says how many went on; when only the amber colour was read, any slot may have.
    const scrolls = read.upgrades ?? (read.upgraded ? (record.upgradeSlots ?? 7) : 0);
    const reach = tolerance + scrolls * PER_SCROLL;
    // The tooltip's own split agrees with the item: its base is the item's value and the bonus adds up.
    if (split && got.base === own.value) {
      return { label, value: got.value, base: got.base, bonus: got.bonus, standard: null, source: "read", note: null };
    }
    // A misread digit in the split is common over a busy background, so the total is judged on its
    // own: a rolled or scrolled copy stays near the item's value, and the total is the number that
    // matters. The split is shown only when it agrees, as above.
    if (got.value === own.value) return { label, value: got.value, base: null, bonus: null, standard: null, source: "read", note: null };
    if (Math.abs(got.value - own.value) <= reach) {
      return { label, value: got.value, base: null, bonus: null, standard: own.value, source: "read", note: null };
    }
    // A base near the item's value with a total that adds up is a rolled copy with scrolls on it.
    if (split && Math.abs(got.base! - own.value) <= tolerance && got.bonus! >= 0 && got.bonus! <= 5 * (record.upgradeSlots ?? 7)) {
      return { label, value: got.value, base: got.base, bonus: got.bonus, standard: own.value, source: "read", note: null };
    }
    return fromData(`Read as ${formatSignedPlain(got.value)}, which doesn't fit this item: showing its own value`);
  });
  const known = new Set(record.stats.map((s) => s.label));
  const scrolled = (read.upgrades ?? 0) > 0 || read.upgraded;
  const unconfirmed: string[] = [];
  for (const got of read.stats) {
    if ((known as Set<string>).has(DATA_LABELS[got.label] ?? got.label)) continue;
    const bonusOnly = got.base === 0 && got.bonus != null && got.bonus === got.value && got.value > 0 && got.value <= 30;
    if (scrolled && bonusOnly) {
      stats.push({ label: got.label, value: got.value, base: 0, bonus: got.bonus, standard: null, source: "read", note: null });
    } else {
      unconfirmed.push(`${got.label} ${formatSignedPlain(got.value)}`);
    }
  }
  const total = record.upgradeSlots;
  const left = read.remainingEnhancements;
  const slotsLeft = total != null && left != null && left <= total ? left : null;
  const slotsNote = left != null && slotsLeft == null ? `Slots left read as ${left}, which can't be right for ${total ?? "this item"} slots` : null;
  return { stats, unconfirmed, slotsLeft, slotsNote };
}

const formatSignedPlain = (value: number) => (value >= 0 ? `+${value}` : String(value));

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
