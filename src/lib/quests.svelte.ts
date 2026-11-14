// The quest book as the Quests tab shows it. It lives here so that switching tabs keeps a search,
// and so a link from another window or panel can bring a quest up.

import { backend } from "./api";
import { formatCompact, formatMesos } from "./format";
import type { QuestSearch, QuestSheet } from "./types";

export const blankSearch = (): QuestSearch => ({ text: "", region: null, repeat: null, atLevel: null, offset: 0 });

export const questView = $state<{ id: number | null; search: QuestSearch; expanded: number[]; seen: number }>({
  /** The quest open from the book. */
  id: null,
  search: blankSearch(),
  /** Tracked quests opened to show their steps. */
  expanded: [],
  /** Goes up with each link followed, so the overlay can open the tab. */
  seen: 0,
});

/** How many reward items a collapsed quest names before counting the rest. */
const NAMED_REWARDS = 2;

/** What a quest pays, in a line: each piece kept apart so it can be coloured or linked. */
export type PayoutPart =
  | { kind: "exp" | "mesos" | "more"; text: string }
  | { kind: "item"; id: number; name: string; count: number };

export function payout(sheet: QuestSheet): PayoutPart[] {
  const named = sheet.rewards.filter((r) => r.sure).slice(0, NAMED_REWARDS);
  const others = sheet.rewards.length - named.length;
  const parts: PayoutPart[] = [];
  if (sheet.row.exp) parts.push({ kind: "exp", text: `${formatCompact(sheet.row.exp)} EXP` });
  if (sheet.row.mesos) parts.push({ kind: "mesos", text: `${formatMesos(sheet.row.mesos)} mesos` });
  for (const r of named) parts.push({ kind: "item", id: r.id, name: r.name, count: r.count });
  if (others) parts.push({ kind: "more", text: `${others} more possible` });
  return parts;
}

export function toggleExpanded(id: number) {
  questView.expanded = questView.expanded.includes(id) ? questView.expanded.filter((q) => q !== id) : [...questView.expanded, id];
}

/**
 * A quest the player follows by hand: one the Quest Helper isn't showing, kept in view with its
 * hints but no counts. Kept in this PC's storage, which every Leafline window shares.
 */
export interface FollowedQuest {
  id: number;
  name: string;
  /** Whose quest log it's in, when a character was read; null shows it for everyone. */
  character: string | null;
}

const FOLLOW_KEY = "leafline.quests.followed";
const CHARACTER_KEY = "leafline.quests.character";

function stored<T>(key: string, fallback: T): T {
  try {
    const value = localStorage.getItem(key);
    return value == null ? fallback : (JSON.parse(value) as T);
  } catch {
    return fallback;
  }
}

function store(key: string, value: unknown) {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {
    // Following is a convenience; without storage it lasts until the window closes.
  }
}

export const following = $state<{ list: FollowedQuest[]; character: string | null }>({
  list: stored<FollowedQuest[]>(FOLLOW_KEY, []),
  /** The character read last, so their quests stay in view while the HUD is hidden. */
  character: stored<string | null>(CHARACTER_KEY, null),
});

const mine = (f: FollowedQuest) => f.character == null || following.character == null || f.character === following.character;

/** The quests the current character follows, oldest first. */
export function followedQuests(): FollowedQuest[] {
  return following.list.filter(mine);
}

export function isFollowed(id: number): boolean {
  return following.list.some((f) => f.id === id && mine(f));
}

export function follow(id: number, name: string) {
  if (isFollowed(id)) return;
  following.list = [...following.list, { id, name, character: following.character }];
  store(FOLLOW_KEY, following.list);
}

export function unfollow(id: number) {
  following.list = following.list.filter((f) => !(f.id === id && mine(f)));
  store(FOLLOW_KEY, following.list);
}

/** Notes whose quests to show. A name not read leaves the last one in place. */
export function noteCharacter(name: string | null) {
  if (!name || name === following.character) return;
  following.character = name;
  store(CHARACTER_KEY, name);
}

/**
 * Which hunting ground a quest objective suggests first: the closest map, or the one with the
 * most of the monster. One choice for every objective, kept on this PC.
 */
export type HuntOrder = "closest" | "most";
const HUNT_ORDER_KEY = "leafline.quests.huntOrder";

export const huntOrder = $state<{ value: HuntOrder }>({ value: stored<HuntOrder>(HUNT_ORDER_KEY, "closest") });

export function setHuntOrder(value: HuntOrder) {
  huntOrder.value = value;
  store(HUNT_ORDER_KEY, value);
}

/** Hunting grounds in the order chosen: as the guide sent them (closest), or the fullest first. */
export function orderGrounds<T extends { count: number; hops: number | null; travel: { hops: number } | null }>(
  maps: T[],
  order: HuntOrder,
): T[] {
  if (order === "closest") return maps;
  const steps = (m: T) => m.travel?.hops ?? m.hops ?? Number.MAX_SAFE_INTEGER;
  return [...maps].sort((a, b) => b.count - a.count || steps(a) - steps(b));
}

let watching = false;

export function watchQuests() {
  if (watching) return;
  watching = true;
  backend.onShowQuest((id) => {
    questView.id = id;
    questView.seen += 1;
  });
  // Another window followed or dropped a quest.
  window.addEventListener("storage", (e) => {
    if (e.key === FOLLOW_KEY) following.list = stored<FollowedQuest[]>(FOLLOW_KEY, []);
    if (e.key === CHARACTER_KEY) following.character = stored<string | null>(CHARACTER_KEY, null);
    if (e.key === HUNT_ORDER_KEY) huntOrder.value = stored<HuntOrder>(HUNT_ORDER_KEY, "closest");
  });
}
