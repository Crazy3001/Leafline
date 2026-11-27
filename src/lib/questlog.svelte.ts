// Each character's quest log, as the game's Quest window last showed it. Live reading keeps it
// while the player has that window open; it is only true as of then, so everything here says
// when it was seen. Every window shares one copy, reloaded whenever the log changes.

import { backend } from "./api";
import { characters } from "./characters.svelte";
import { following } from "./quests.svelte";
import type { LoggedQuest, QuestLog, QuestTab, RegionTotal, WholeLog } from "./types";

export const questLog = $state<{ log: QuestLog | null; totals: RegionTotal[] }>({ log: null, totals: [] });

/**
 * Whose log to show: the character looked at (on screen, or picked in the character picker), else
 * the character read last, or failing that, whoever's Quest window was read last.
 */
const whose = () => characters.active ?? following.character;

let loading: string | null | undefined = undefined;

async function load() {
  const name = whose();
  loading = name;
  try {
    const log = await backend.questLog(name);
    if (loading === name) questLog.log = log;
  } catch {
    // Without records the log stays as it was; the Quests tab works without it.
  }
}

let watching = false;

/** Loads the log and keeps it current. Safe to call from every component that shows it. */
export function watchQuestLog() {
  if (watching) return;
  watching = true;
  backend.questRegionTotals().then((t) => (questLog.totals = t)).catch(() => {});
  backend.onQuestLog((name) => {
    const now = whose();
    if (now == null || now.toLowerCase() === name.toLowerCase()) void load();
  });
  $effect.root(() => {
    $effect(() => {
      void whose();
      void load();
    });
  });
}

/** The data's region totals again, after the map data changed. */
export function refreshRegionTotals() {
  backend.questRegionTotals().then((t) => (questLog.totals = t)).catch(() => {});
}

/**
 * Quests the game's Quest window lists by their series' name, by the step of the data each stands
 * for: the whole quest log works that out, and `noteSeriesSteps` keeps it here.
 */
const seriesSteps = $state<{ byId: Record<number, string> }>({ byId: {} });

/** Keeps which step each series row stands for, from the whole quest log. */
export function noteSeriesSteps(whole: WholeLog | null) {
  const byId: Record<number, string> = {};
  for (const r of whole?.regions ?? []) for (const q of r.quests) if (q.questId != null && q.series) byId[q.questId] = q.series;
  seriesSteps.byId = byId;
}

/** What the log knows of a quest in the data, if anything, including a row listed by its series' name. */
export function logged(id: number | null | undefined): LoggedQuest | null {
  if (id == null) return null;
  const quests = questLog.log?.quests ?? [];
  const own = quests.find((q) => q.questId === id);
  if (own) return own;
  const series = seriesSteps.byId[id];
  return series ? (quests.find((q) => q.questId == null && fold(q.title) === fold(series)) ?? null) : null;
}

/** The quests the log shows done, by id, with the steps of a line it showed done by the line's name. */
export function doneIds(): number[] {
  const quests = questLog.log?.quests ?? [];
  const own = quests.filter((q) => q.done && q.questId != null).map((q) => q.questId!);
  const lines = new Set(quests.filter((q) => q.done && q.questId == null && q.state !== "inProgress").map((q) => fold(q.title)));
  const steps = Object.entries(seriesSteps.byId)
    .filter(([, series]) => lines.has(fold(series)))
    .map(([id]) => Number(id));
  return [...new Set([...own, ...steps])];
}

/** Whether there is a log to go by: some tab has been looked at. */
export function hasLog(): boolean {
  return (questLog.log?.looks.length ?? 0) > 0;
}

/** When a tab was last looked at, and last looked through whole. */
export function look(tab: QuestTab) {
  return questLog.log?.looks.find((l) => l.tab === tab) ?? null;
}

export type QuestMarkKind = "done" | "inProgress" | "ready" | "available" | "gone";

/**
 * Whether a quest's counters were full when last seen, or the game showed it with the ring that
 * stood by a full one. Either way the turn-in is the next step.
 */
export function readyToHandIn(q: LoggedQuest): boolean {
  if (q.state !== "inProgress") return false;
  if (q.counters.length) return q.counters.every((c) => c.have >= c.need);
  return q.icon === "ring";
}

/** The one mark a quest gets: done wins, then where it stands now. */
export function markOf(q: LoggedQuest | null): QuestMarkKind | null {
  if (!q) return null;
  if (q.state === "inProgress") return readyToHandIn(q) ? "ready" : "inProgress";
  if (q.done) return "done";
  if (q.state === "available") return "available";
  if (q.state === "gone") return "gone";
  return null;
}

export const MARK_LABEL: Record<QuestMarkKind, string> = {
  done: "Done",
  inProgress: "In progress",
  ready: "Ready to hand in",
  available: "Available",
  gone: "Handed in or dropped",
};

/** When what a mark says was seen, unix seconds. */
export function markSeenAt(q: LoggedQuest): number | null {
  return q.state === "none" || (q.done && q.state !== "inProgress") ? (q.doneSeenAt ?? q.stateSeenAt) : q.stateSeenAt;
}

/**
 * How many of these quests the log doesn't show done, or `null` with no log to go by. A quest the
 * log has never seen counts as not done.
 */
export function notDone(ids: number[]): number | null {
  if (!hasLog()) return null;
  const done = new Set(doneIds());
  return ids.filter((id) => !done.has(id)).length;
}

/** A count of quests, with how many aren't done when the log can say: `5 quests, 3 not done`. */
export function questCount(ids: number[]): string {
  const n = ids.length;
  const left = notDone(ids);
  const words = `${n} quest${n === 1 ? "" : "s"}`;
  return left == null || left === n ? words : `${words}, ${left} not done`;
}

/** Folds a name for loose comparison: case, spacing and marks ignored. */
export const fold = (text: string) => text.toLowerCase().replace(/[^a-z0-9]/g, "");
