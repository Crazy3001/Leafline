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

/** What a quest pays, in a line. */
export function payout(sheet: QuestSheet): string {
  const sure = sheet.rewards.filter((r) => r.sure);
  const named = sure.slice(0, NAMED_REWARDS).map((r) => (r.count > 1 ? `${r.name} ×${r.count}` : r.name));
  const others = sheet.rewards.length - named.length;
  return [
    sheet.row.exp ? `${formatCompact(sheet.row.exp)} EXP` : null,
    sheet.row.mesos ? `${formatMesos(sheet.row.mesos)} mesos` : null,
    ...named,
    others ? `${others} more possible` : null,
  ]
    .filter(Boolean)
    .join(" · ");
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
