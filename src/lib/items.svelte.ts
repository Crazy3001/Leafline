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
    .replace(/\(\s*[MFU]\s*\)/gi, "")
    .toLowerCase()
    .replace(/[^a-z0-9]/g, "");

/** How alike two strings are, 0 to 1, by the edits between them. */
function likeness(a: string, b: string): number {
  if (a === b) return 1;
  if (!a.length || !b.length) return 0;
  let row = Array.from({ length: b.length + 1 }, (_, i) => i);
  for (let i = 1; i <= a.length; i++) {
    const next = [i];
    for (let j = 1; j <= b.length; j++) {
      next[j] = Math.min(row[j] + 1, next[j - 1] + 1, row[j - 1] + (a[i - 1] === b[j - 1] ? 0 : 1));
    }
    row = next;
  }
  return 1 - row[b.length] / Math.max(a.length, b.length);
}

/**
 * Whether a name read from the game is the data's item. "exact": the names agree (or the whole
 * name stands after stray text) and the level, if one was read, does too. "likely": the spelling
 * is nearly the same and the level agrees, as with a letter OCR split in two. "close": only the
 * nearest guess in the data.
 */
export function nameMatch(read: ItemTooltip, record: ItemRecord): "exact" | "likely" | "close" {
  const readKey = nameKey(read.name);
  const ownKey = nameKey(record.name);
  const level = read.requirements.level;
  const levelAgrees = level != null && level === record.reqLevel;
  // Text from behind or beside the tooltip can run on before or after the name ("GUida Eagle Crow",
  // "Studded Polearm I-I-I"). The whole name standing inside, with the level agreeing, is the item.
  const trailing = ownKey.length >= 7 && readKey.includes(ownKey) && levelAgrees;
  if ((readKey === ownKey || trailing) && (level == null || levelAgrees)) return "exact";
  // The (M) or (F) after a name is judged apart, so one stray letter in a long name isn't made worse by it.
  const bare = (name: string) => nameKey(name.replace(/\(\s*[MFU]\s*\)/gi, ""));
  return levelAgrees && bare(record.name).length >= 7 && likeness(bare(read.name), bare(record.name)) >= 0.85 ? "likely" : "close";
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

/**
 * The most one successful scroll adds to a single stat in Classic: +4 for the strongest (a Chaos
 * scroll). HP and MP scrolls add more (the guide says 10, and a Top was seen with +15 after one
 * scroll), so they get 20. From a player-written guide and one real item, not the game's data.
 */
const perScroll = (label: string) => (/^Max[HM]P$/.test(label) ? 20 : 4);

/** How far a rolled copy's stat can sit from the data's value and still be believed as read. */
const roll = (standard: number) => Math.max(2, Math.round(Math.abs(standard) * 0.2));

/**
 * The data's stats are the baseline. A read value replaces one only when it can be believed: it
 * shows the item's own value as its base and the bonus adds up, or it sits within a roll of the
 * data's. Anything else keeps the data's value with a note. A read stat the data lacks is shown
 * only when it is a scroll's bonus on an item with scrolls; otherwise it is listed as unconfirmed.
 */
export function checkItem(read: ItemTooltip, record: ItemRecord): CheckedItem {
  const exact = (label: string) => read.stats.find((s) => (DATA_LABELS[s.label] ?? s.label) === label);
  const letters = (label: string) => label.toUpperCase().replace(/[^A-Z]/g, "");
  const dataLabels = new Set(record.stats.map((s) => s.label));
  // A label OCR garbled (`LU-JK` for LUK) is the item's stat when it is one edit from it, the item
  // has that stat, and the tooltip didn't also show it right.
  const spelled = new Map<string, ItemTooltip["stats"][number]>();
  for (const got of read.stats) {
    const label = DATA_LABELS[got.label] ?? got.label;
    if (dataLabels.has(label) || spelled.size > 8) continue;
    // The label is close to the item's, or the tooltip's own (base +bonus) starts from the item's value for
    // a stat nothing else was read for (`II BTR +3 (2 +1)` is the STR of 2).
    const near = [...dataLabels].find((name) => {
      if (!(letters(name).length >= 3 && !exact(name) && !spelled.has(name))) return false;
      const value = record.stats.find((s) => s.label === name)?.value;
      // Its letters are all there in order with a few extras (`LU-JK` has the L, U and K of LUK).
      const inOrder = (own: string, read: string) => {
        let at = 0;
        for (const ch of read) if (ch === own[at]) at++;
        return at === own.length && read.length <= own.length + 3;
      };
      return (
        likeness(letters(label), letters(name)) >= 0.7 ||
        inOrder(letters(name), letters(label)) ||
        (got.base != null && got.bonus != null && got.base === value)
      );
    });
    if (near) spelled.set(near, got);
  }
  const readFor = (label: string) => exact(label) ?? spelled.get(label);
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
    const reach = tolerance + scrolls * perScroll(own.label);
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
    if (split && Math.abs(got.base! - own.value) <= tolerance && got.bonus! >= 0 && got.bonus! <= perScroll(own.label) * (record.upgradeSlots ?? 7)) {
      return { label, value: got.value, base: got.base, bonus: got.bonus, standard: own.value, source: "read", note: null };
    }
    return fromData(`Read as ${formatSignedPlain(got.value)}, which doesn't fit this item: showing its own value`);
  });
  const known = new Set(record.stats.map((s) => s.label));
  const scrolls = read.upgrades ?? (read.upgraded ? (record.upgradeSlots ?? 7) : 0);
  const unconfirmed: string[] = [];
  for (const got of read.stats) {
    if ((known as Set<string>).has(DATA_LABELS[got.label] ?? got.label) || [...spelled.values()].includes(got)) continue;
    // A stat the item doesn't have is a scroll's: believed on a scrolled item when the amount is one
    // those scrolls can give and any (base +bonus) it shows is all bonus.
    const fits = got.value > 0 && got.value <= scrolls * perScroll(got.label);
    const allBonus = got.base == null || (got.base === 0 && got.bonus === got.value);
    // OCR often reads one line twice under a garbled label (`LU-JK +1` beside `LUK +1`), which would
    // only add noise: a value already shown from a good read isn't listed again.
    if (stats.some((s) => s.source === "read" && s.value === got.value)) continue;
    if (fits && allBonus) {
      stats.push({ label: got.label, value: got.value, base: got.base, bonus: got.bonus, standard: null, source: "read", note: null });
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
