// What the build planner window works out from a plan's levels: the short timeline, and when
// each goal is met. Only reads the expansion the backend sends; nothing here decides the plan.
import { statLabel } from "$lib/builds.svelte";
import type { BaseStat, BaseStats, LevelStep, SkillLevel, StatGoal } from "$lib/types";

/** One line of a timeline row, and what it marks. */
export interface TimelineLine {
  text: string;
  /** "advance": a job advancement; "gear": goal gear to get; "met": a skill goal reached. */
  kind: "plain" | "advance" | "gear" | "met" | "sub";
}

/** One row: a level, or a run of levels that do the same thing. */
export interface TimelineRow {
  from: number;
  to: number;
  lines: TimelineLine[];
  steps: LevelStep[];
}

const hasWork = (s: LevelStep) => s.ap.length > 0 || s.sp.length > 0 || s.advance != null || s.gear.length > 0;
/** Levels that stand alone: something happens there besides spending points. */
const special = (s: LevelStep) => s.advance != null || s.gear.length > 0;
const skillSet = (s: LevelStep) => s.sp.map((g) => g.skillId).sort((a, b) => a - b).join(",");

/**
 * The plan's levels as rows. Folded, a run of levels that put SP into the same skills becomes one
 * row; a job advancement, new gear and the character's own level each get a row of their own.
 */
export function timeline(steps: LevelStep[], goals: SkillLevel[], level: number | null, fold: boolean): TimelineRow[] {
  const worked = steps.filter(hasWork);
  const runs: LevelStep[][] = [];
  for (const step of worked) {
    const run = runs.at(-1);
    const last = run?.at(-1);
    const joins =
      fold &&
      run &&
      last &&
      step.level === last.level + 1 &&
      !special(step) &&
      !special(last) &&
      step.level !== level &&
      last.level !== level &&
      skillSet(step) === skillSet(last);
    if (joins) run.push(step);
    else runs.push([step]);
  }
  return runs.map((run) => ({ from: run[0].level, to: run.at(-1)!.level, lines: describe(run, goals), steps: run }));
}

function describe(run: LevelStep[], goals: SkillLevel[]): TimelineLine[] {
  const one = run.length === 1;
  const lines: TimelineLine[] = [];
  for (const step of run) if (step.advance) lines.push({ text: step.advance, kind: "advance" });
  for (const step of run) for (const name of step.gear) lines.push({ text: `Get ${name}`, kind: "gear" });

  const skills = new Map<number, { name: string; amounts: number[]; from: number; to: number }>();
  for (const step of run) {
    for (const gain of step.sp) {
      const known = skills.get(gain.skillId);
      if (known) {
        known.amounts.push(gain.to - gain.from);
        known.to = gain.to;
      } else skills.set(gain.skillId, { name: gain.name, amounts: [gain.to - gain.from], from: gain.from, to: gain.to });
    }
  }
  for (const [id, s] of skills) {
    const met = goals.some((g) => g.skillId === id && g.level === s.to);
    const each = s.amounts.every((a) => a === s.amounts[0]) && s.amounts.length === run.length;
    const text = one ? `${s.name} ${s.from} → ${s.to}` : each ? `${s.name} +${s.amounts[0]} a level, to ${s.to}` : `${s.name} to ${s.to}`;
    lines.push({ text, kind: met ? "met" : "plain" });
  }

  const stats = new Map<BaseStat, { amounts: number[]; at: number[]; total: number }>();
  for (const step of run) {
    for (const gain of step.ap) {
      const known = stats.get(gain.stat) ?? { amounts: [], at: [], total: 0 };
      known.amounts.push(gain.amount);
      known.at.push(step.level);
      known.total = gain.total;
      stats.set(gain.stat, known);
    }
  }
  const parts = [...stats].map(([stat, s]) => {
    const label = statLabel(stat);
    if (one) return `${label} +${s.amounts[0]}, to ${s.total}`;
    const every = s.amounts.length === run.length && s.amounts.every((a) => a === s.amounts[0]);
    if (every) return `${label} +${s.amounts[0]} a level, to ${s.total}`;
    if (s.at.length === 1) return `${label} +${s.amounts[0]} at ${s.at[0]}, to ${s.total}`;
    return `${label} to ${s.total}`;
  });
  if (parts.length) lines.push({ text: parts.join(" · "), kind: "sub" });
  return lines;
}

/** Where a skill goal stands: met before the plan starts, at a level, or not by the end. */
export type Reached = { kind: "already" } | { kind: "at"; level: number } | { kind: "never"; end: number };

export function skillReached(goal: SkillLevel, start: SkillLevel[], steps: LevelStep[]): Reached {
  if ((start.find((s) => s.skillId === goal.skillId)?.level ?? 0) >= goal.level) return { kind: "already" };
  const step = steps.find((s) => s.sp.some((g) => g.skillId === goal.skillId && g.to >= goal.level));
  return step ? { kind: "at", level: step.level } : { kind: "never", end: steps.at(-1)?.level ?? 0 };
}

/** When a stat goal with a target is met, by the plan's base stats. */
export function statReached(goal: StatGoal, start: BaseStats | null, steps: LevelStep[]): Reached {
  if (goal.target == null) return { kind: "never", end: steps.at(-1)?.level ?? 0 };
  if ((start?.[goal.stat] ?? 0) >= goal.target) return { kind: "already" };
  const step = steps.find((s) => s.stats[goal.stat] >= goal.target!);
  return step ? { kind: "at", level: step.level } : { kind: "never", end: steps.at(-1)?.level ?? 0 };
}

/** The step in force at a level: the last one at or before it. */
export const stepAt = (steps: LevelStep[], level: number) => steps.findLast((s) => s.level <= level) ?? steps[0] ?? null;

/** AP spent on each stat across the plan. */
export function apSplit(steps: LevelStep[]): { stat: BaseStat; amount: number }[] {
  const sums = new Map<BaseStat, number>();
  for (const step of steps) for (const gain of step.ap) sums.set(gain.stat, (sums.get(gain.stat) ?? 0) + gain.amount);
  return [...sums].map(([stat, amount]) => ({ stat, amount })).filter((s) => s.amount > 0);
}
