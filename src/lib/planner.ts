// What the build planner window works out from a plan's levels: the short timeline, and when
// each goal is met. Only reads the expansion the backend sends; nothing here decides the plan.
import { statLabel } from "$lib/builds.svelte";
import type { BaseStat, BaseStats, LevelStep, SkillLevel, StatGoal } from "$lib/types";

/** Each stat's colour wherever the planner draws AP. */
export const STAT_COLOURS: Record<BaseStat, string> = { str: "var(--hp)", dex: "var(--travel)", int: "var(--mp)", luk: "var(--accent)" };

/** One line of a timeline row, and what it marks. */
export interface TimelineLine {
  text: string;
  /** "advance": a job advancement; "gear": gear to get; "met": a skill goal reached. */
  kind: "plain" | "advance" | "gear" | "met" | "sub";
  /** The skill a skill line is about, for its link; the line's text starts with its name. */
  skillId?: number;
  skillName?: string;
  /** A gear line's pieces, shown as their pictures. */
  items?: string[];
}

/** One row: a level, or a run of levels that do the same thing. */
export interface TimelineRow {
  from: number;
  to: number;
  lines: TimelineLine[];
  steps: LevelStep[];
}

/**
 * Gear worth a line: what a level still ahead brings within reach. Gear the character can already
 * wear, at or below their own level, isn't a thing to get later: the gear views have it.
 */
const gearAhead = (s: LevelStep, level: number | null) => s.gear.length > 0 && (level == null || s.level > level);
const hasWork = (s: LevelStep, level: number | null) =>
  s.ap.length > 0 || s.sp.length > 0 || s.advance != null || gearAhead(s, level);
/** Levels that stand alone: something happens there besides spending points. */
const special = (s: LevelStep, level: number | null) => s.advance != null || gearAhead(s, level);
const skillSet = (s: LevelStep) => s.sp.map((g) => g.skillId).sort((a, b) => a - b).join(",");

/**
 * The plan's levels as rows. Folded, a run of levels that put SP into the same skills becomes one
 * row; a job advancement, new gear and the character's own level each get a row of their own.
 */
export function timeline(steps: LevelStep[], goals: SkillLevel[], level: number | null, fold: boolean): TimelineRow[] {
  const worked = steps.filter((s) => hasWork(s, level));
  const runs: LevelStep[][] = [];
  for (const step of worked) {
    const run = runs.at(-1);
    const last = run?.at(-1);
    const joins =
      fold &&
      run &&
      last &&
      step.level === last.level + 1 &&
      !special(step, level) &&
      !special(last, level) &&
      step.level !== level &&
      last.level !== level &&
      skillSet(step) === skillSet(last);
    if (joins) run.push(step);
    else runs.push([step]);
  }
  return runs.map((run) => ({ from: run[0].level, to: run.at(-1)!.level, lines: describe(run, goals, level), steps: run }));
}

function describe(run: LevelStep[], goals: SkillLevel[], level: number | null): TimelineLine[] {
  const one = run.length === 1;
  const lines: TimelineLine[] = [];
  for (const step of run) if (step.advance) lines.push({ text: step.advance, kind: "advance" });
  const gear = run.filter((step) => gearAhead(step, level)).flatMap((step) => step.gear);
  if (gear.length) lines.push({ text: `Gear: ${gear.join(", ")}`, kind: "gear", items: gear });

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
    const text = one || !each ? `${s.name} → ${s.to}` : `${s.name} +${s.amounts[0]}/lv → ${s.to}`;
    lines.push({ text, kind: met ? "met" : "plain", skillId: id, skillName: s.name });
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
    if (one) return `${label} → ${s.total}`;
    const every = s.amounts.length === run.length && s.amounts.every((a) => a === s.amounts[0]);
    if (every) return `${label} +${s.amounts[0]}/lv → ${s.total}`;
    return `${label} → ${s.total}`;
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

/** One skill goal on the chart: the levels whose SP goes into it, and when it is met. */
export interface GoalLane {
  /** The goal's place in the plan's list. */
  index: number;
  skillId: number;
  name: string;
  target: number;
  /** The skill's level when this goal starts: what the character had, or an earlier goal's target. */
  floor: number;
  /** Each level that puts SP into this goal, and how much. */
  spends: { level: number; amount: number }[];
  reached: Reached;
}

/**
 * The plan's skill goals as lanes. A skill named twice (Nimble Body to 3, later to 20) gets two
 * lanes; each level's SP counts toward the goal it fills.
 */
export function goalLanes(goals: SkillLevel[], start: SkillLevel[], steps: LevelStep[], nameOf: (id: number) => string | undefined): GoalLane[] {
  return goals.map((goal, index) => {
    const had = start.find((s) => s.skillId === goal.skillId)?.level ?? 0;
    const earlier = goals.slice(0, index).filter((g) => g.skillId === goal.skillId).map((g) => g.level);
    const floor = Math.max(had, ...earlier);
    const spends: GoalLane["spends"] = [];
    for (const step of steps) {
      for (const gain of step.sp) {
        if (gain.skillId !== goal.skillId) continue;
        const amount = Math.min(gain.to, goal.level) - Math.max(gain.from, floor);
        if (amount > 0) spends.push({ level: step.level, amount });
      }
    }
    const name = nameOf(goal.skillId) ?? steps.flatMap((s) => s.sp).find((g) => g.skillId === goal.skillId)?.name ?? `Skill ${goal.skillId}`;
    return { index, skillId: goal.skillId, name, target: goal.level, floor, spends, reached: skillReached(goal, start, steps) };
  });
}

/** Where a goal stands for a character at a level: met, taking the next level's SP, or ahead. */
export type LaneState = "done" | "now" | "later" | "short";

export function laneState(lane: GoalLane, level: number | null): LaneState {
  const { reached } = lane;
  if (reached.kind === "already" || (reached.kind === "at" && level != null && reached.level <= level)) return "done";
  if (level != null && lane.spends.some((s) => s.level === level + 1)) return "now";
  return reached.kind === "never" ? "short" : "later";
}

/** A run of levels whose AP goes mostly into one stat. */
export interface ApRun {
  from: number;
  to: number;
  stat: BaseStat;
  /** The stat's total at the end of the run. */
  total: number;
  amount: number;
}

/** The AP row of the chart: runs of levels by the stat that gets most of each level's AP. */
export function apRuns(steps: LevelStep[]): ApRun[] {
  const runs: ApRun[] = [];
  for (const step of steps) {
    if (!step.ap.length) continue;
    const main = step.ap.reduce((a, b) => (b.amount > a.amount ? b : a));
    const last = runs.at(-1);
    if (last && last.stat === main.stat && last.to === step.level - 1) {
      last.to = step.level;
      last.total = step.stats[main.stat];
      last.amount += main.amount;
    } else runs.push({ from: step.level, to: step.level, stat: main.stat, total: step.stats[main.stat], amount: main.amount });
  }
  return runs;
}

/** The step in force at a level: the last one at or before it. */
export const stepAt = (steps: LevelStep[], level: number) => steps.findLast((s) => s.level <= level) ?? steps[0] ?? null;

/** AP spent on each stat across the plan. */
export function apSplit(steps: LevelStep[]): { stat: BaseStat; amount: number }[] {
  const sums = new Map<BaseStat, number>();
  for (const step of steps) for (const gain of step.ap) sums.set(gain.stat, (sums.get(gain.stat) ?? 0) + gain.amount);
  return [...sums].map(([stat, amount]) => ({ stat, amount })).filter((s) => s.amount > 0);
}
