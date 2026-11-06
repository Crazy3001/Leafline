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

let watching = false;

export function watchQuests() {
  if (watching) return;
  watching = true;
  backend.onShowQuest((id) => {
    questView.id = id;
    questView.seen += 1;
  });
}
