// What the Quests page's list and its questline card share: a quest's freshest counts, whether
// it's ready to hand in, and where a questline stands.
import { fold } from "$lib/questlog.svelte";
import type { QuestReading, WholeQuest } from "$lib/types";

export type Count = { name: string; have: number; need: number };

/**
 * The Quest Helper's counts for a quest while it shows it, matched by the quest's title or the
 * series' name the game lists it by. A count past its target (items held) is kept as read.
 */
export function helperCounts(helper: QuestReading | null | undefined, q: WholeQuest): Count[] | null {
  const names = [q.title, q.series].filter((n): n is string => !!n).map(fold);
  const shown = helper?.quests.find((h) => names.includes(fold(h.title)));
  if (!shown?.objectives.length) return null;
  const read = shown.objectives.filter((o) => !o.unread);
  return read.length ? read.map((o) => ({ name: o.target, have: o.current, need: o.required })) : null;
}

/** Ready to hand in: by the Quest window, or by the Quest Helper's counts now. */
export function isReady(q: WholeQuest, helper: QuestReading | null | undefined): boolean {
  if (q.stand === "ready") return true;
  if (q.stand !== "inProgress") return false;
  const shown = helper?.quests.find((h) => [q.title, q.series].some((n) => n && fold(n) === fold(h.title)));
  return !!shown && shown.objectives.length > 0 && shown.objectives.every((o) => !o.unread && o.current >= o.required);
}

/** A line's quests in step order; a row the game listed by the line's name goes last. */
export function lineSteps(steps: WholeQuest[]): WholeQuest[] {
  const at = (q: WholeQuest) => (q.line?.step ? q.line.step : Number.MAX_SAFE_INTEGER);
  return [...steps].sort((a, b) => at(a) - at(b));
}

/**
 * The step you're on: one in progress, else the first you can take, else the first not done.
 * `null` once every step is done.
 */
export function currentStep(steps: WholeQuest[], helper?: QuestReading | null): WholeQuest | null {
  const ordered = lineSteps(steps).filter((q) => q.questId != null);
  return (
    ordered.find((q) => q.stand === "ready" || q.stand === "inProgress" || isReady(q, helper)) ??
    ordered.find((q) => q.stand === "canTake") ??
    ordered.find((q) => q.stand !== "done" && q.stand !== "gone") ??
    null
  );
}
