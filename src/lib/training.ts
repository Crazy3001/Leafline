// Sums over saved sessions for the Train tab and the training log: rates per map, by level and
// party, the best run on a map, days played, and the graph data from a session's timeline.

import type { Bar, ChartPoint } from "./ui/types";
import { formatCompact, formatDuration, splitMapLabel } from "./format";
import type { LevelExp, SavedSession, SessionEvent, SessionLine, SessionSummary, TimelinePoint } from "./types";

/** Less time than this on a map makes its rate a rough guess. */
export const SHORT_SECONDS = 30 * 60;

/** The graph skips the first stretch of a session, where a rate rests on too little play. */
export const GRAPH_WARMUP_SECONDS = 150;

/** Runs shorter than this are too thin to rank as a record, so they list below the rest. */
export const MIN_RECORD_SECONDS = 60;

/** Width of the level bands the Maps and Records views filter by. */
const BAND_WIDTH = 5;

/** The level bands that hold some play, as the options both views offer. */
export function levelBands(sessions: SavedSession[]): { id: string; label: string }[] {
  const lows = new Set<number>();
  for (const s of sessions) for (const l of s.lines) if (l.level != null) lows.add(Math.floor(l.level / BAND_WIDTH) * BAND_WIDTH);
  return [...lows].sort((a, b) => a - b).map((low) => ({ id: String(low), label: `Lv ${low}–${low + BAND_WIDTH - 1}` }));
}

/** The levels a band option covers, for a [`LineFilter`]. */
export function bandRange(id: string): [number, number] | null {
  return id === "any" ? null : [Number(id), Number(id) + BAND_WIDTH - 1];
}

/** Active time with the mesos out of sight before the player is reminded, once they stop. */
const MESOS_UNSEEN_REMINDER = 5 * 60;

/**
 * How much of the session Mesos / hr covers. The rate is only over the time between looks at
 * the mesos, so it can cover less than the EXP rate beside it: `partial` says to show how much,
 * `thin` to grey the number out.
 */
export function mesosCoverage(s: SessionSummary): { partial: boolean; thin: boolean; label: string } {
  const share = s.activeSeconds > 0 ? Math.min(1, s.mesosSeconds / s.activeSeconds) : 0;
  return {
    partial: s.activeSeconds >= 120 && share < 0.9,
    thin: share < 0.5,
    label: `over ${formatDuration(s.mesosSeconds)} of ${formatDuration(s.activeSeconds)}`,
  };
}

/**
 * A reminder to look at the mesos, once the player stops hunting (in town or not) with a good
 * stretch of hunting not counted yet. While they hunt, the coverage label says enough.
 */
export function mesosReminder(s: SessionSummary): string | null {
  if (s.held || !s.paused || s.mesosUnseenSeconds < MESOS_UNSEEN_REMINDER) return null;
  const counted = `Your last ${formatDuration(s.mesosUnseenSeconds)} of hunting isn't in Mesos / hr yet.`;
  return s.town
    ? `${counted} Open your inventory before you spend, or give a shop a moment before buying.`
    : `${counted} Open your inventory to count it.`;
}

export type Metric = "exp" | "mesos";

/**
 * Time the tracker couldn't place on one map (before the minimap was read, or gains that spanned
 * a map change) gets a row only when there is enough of it to matter; it always counts in the
 * session's totals.
 */
export function showUnplaced(row: { map: string | null; activeSeconds: number }, sessionSeconds: number): boolean {
  return row.map != null || (row.activeSeconds >= 120 && row.activeSeconds >= sessionSeconds * 0.1);
}

/** What a session's graph plots. At high levels the HUD shows only a percent, so EXP is in %. */
export function chartPoints(timeline: TimelinePoint[], metric: Metric, percentOnly: boolean): ChartPoint[] {
  // A point is the rate over the five minutes before it, so the first ones rest on a few
  // seconds of play and swing wildly. They are left out once the rest has enough to draw.
  const settled = timeline.filter((p) => p.at >= GRAPH_WARMUP_SECONDS);
  return (settled.length >= 4 ? settled : timeline).map((p) => ({
    x: p.at,
    y: metric === "mesos" ? p.mesosPerHour : percentOnly ? p.percentPerHour : p.expPerHour,
  }));
}

export function rateFormat(metric: Metric, percentOnly: boolean): (value: number) => string {
  if (metric === "exp" && percentOnly) return (v) => `${v.toFixed(v < 10 ? 1 : 0)}%`;
  return (v) => formatCompact(v);
}

/** Stretches of the timeline on one map, for the shading behind the graph. */
export function mapBands(timeline: TimelinePoint[]): { start: number; end: number; label: string }[] {
  const bands: { start: number; end: number; label: string }[] = [];
  let previous = 0;
  for (const point of timeline) {
    const label = point.map ? splitMapLabel(point.map).name : "Between maps";
    const last = bands[bands.length - 1];
    if (last && last.label === label) last.end = point.at;
    else bands.push({ start: last ? previous : 0, end: point.at, label });
    previous = point.at;
  }
  return bands;
}

export function eventMarkers(events: SessionEvent[]) {
  return events.map((e) => ({
    x: e.at,
    kind: e.kind,
    label: e.kind === "levelUp" ? `Reached Lv ${e.level ?? "?"} at ${formatDuration(e.at)}` : `Died at ${formatDuration(e.at)}`,
  }));
}

/** The map and level at a point, for a tooltip. */
export function pointDetail(point: TimelinePoint | undefined): string | null {
  if (!point) return null;
  const parts = [point.map ? splitMapLabel(point.map).name : null, point.level != null ? `Lv ${point.level}` : null];
  return parts.filter(Boolean).join(" · ") || null;
}

export interface LineFilter {
  /** Character level then, inclusive. */
  levels?: [number, number] | null;
  party?: "solo" | "party" | "any";
  character?: string | null;
  /** Only sessions started at or after this, in Unix seconds. */
  since?: number | null;
  /** The job read then; lines with no job read pass only an empty filter. */
  job?: string | null;
}

function keep(session: SavedSession, line: SessionLine, filter: LineFilter) {
  if (filter.character && session.character && session.character !== filter.character) return false;
  if (filter.since && (session.startedAt ?? session.savedAt) < filter.since) return false;
  if (filter.levels && (line.level == null || line.level < filter.levels[0] || line.level > filter.levels[1])) return false;
  const solo = (line.party ?? 1) <= 1;
  if (filter.party === "solo" && !solo) return false;
  if (filter.party === "party" && solo) return false;
  if (filter.job && line.job !== filter.job) return false;
  return line.map != null && line.activeSeconds > 0;
}

/** Totals behind a rate, so groups can be summed before dividing. */
interface Tally {
  seconds: number;
  exp: number;
  percent: number;
  mesos: number;
  mesosSeconds: number;
  /** Potion cost, each session's shared out by active time. */
  potions: number;
  sessions: Set<number>;
}

const tally = (): Tally => ({ seconds: 0, exp: 0, percent: 0, mesos: 0, mesosSeconds: 0, potions: 0, sessions: new Set() });

function add(t: Tally, session: SavedSession, line: SessionLine) {
  const sessionId = session.id;
  if (session.potionCost > 0 && session.activeSeconds > 0)
    t.potions += (session.potionCost * line.activeSeconds) / session.activeSeconds;
  t.seconds += line.activeSeconds;
  t.exp += line.expGained;
  t.percent += line.percentGained;
  t.mesos += line.mesosGained;
  t.mesosSeconds += line.mesosSeconds;
  t.sessions.add(sessionId);
}

export interface RecordRates {
  activeSeconds: number;
  sessions: number;
  /** Null when the HUD showed only a percentage, so EXP amounts are unknown. */
  expPerHour: number | null;
  percentPerHour: number | null;
  /** Null when the mesos were never looked at twice on this stretch. */
  mesosPerHour: number | null;
  /** The share of the time Mesos / hr covers. */
  mesosShare: number;
  /** Mesos / hr with potion costs taken out; null where no potions were counted. */
  netPerHour: number | null;
}

function rates(t: Tally): RecordRates {
  return {
    activeSeconds: t.seconds,
    sessions: t.sessions.size,
    expPerHour: t.exp > 0 ? (t.exp * 3600) / t.seconds : t.percent > 0 ? null : 0,
    percentPerHour: t.percent > 0 ? (t.percent * 3600) / t.seconds : null,
    mesosPerHour: t.mesosSeconds > 0 ? (t.mesos * 3600) / t.mesosSeconds : null,
    mesosShare: t.seconds > 0 ? Math.min(1, t.mesosSeconds / t.seconds) : 0,
    netPerHour: t.potions > 0 && t.mesosSeconds > 0 ? (t.mesos * 3600) / t.mesosSeconds - (t.potions * 3600) / t.seconds : null,
  };
}

/** One way the player trained on a map: a level, a party size and a job. */
export interface RecordRow extends RecordRates {
  key: string;
  level: number | null;
  party: number | null;
  job: string | null;
}

export interface MapRecord extends RecordRates {
  map: string;
  name: string;
  street: string | null;
  rows: RecordRow[];
}

/**
 * The player's records by map, then by level, party size and job, over the saved sessions that
 * pass the filter. Maps and rows come in the order `by` asks for, most first.
 */
export function mapRecords(
  sessions: SavedSession[],
  filter: LineFilter = {},
  by: "exp" | "mesos" | "time" = "exp",
): MapRecord[] {
  const maps = new Map<string, { total: Tally; rows: Map<string, { row: Omit<RecordRow, keyof RecordRates>; tally: Tally }> }>();
  for (const session of sessions) {
    for (const line of session.lines) {
      if (!keep(session, line, filter) || line.town) continue;
      const entry = maps.get(line.map!) ?? { total: tally(), rows: new Map() };
      add(entry.total, session, line);
      // No party read counts as solo, as the filters take it.
      const party = (line.party ?? 1) <= 1 ? 1 : line.party;
      const key = `${line.level ?? "?"}|${party ?? "?"}|${line.job ?? "?"}`;
      const row = entry.rows.get(key) ?? { row: { key, level: line.level, party, job: line.job }, tally: tally() };
      add(row.tally, session, line);
      entry.rows.set(key, row);
      maps.set(line.map!, entry);
    }
  }
  const value = (r: RecordRates) =>
    by === "time" ? r.activeSeconds : by === "mesos" ? (r.mesosPerHour ?? -1) : (r.expPerHour ?? r.percentPerHour ?? -1);
  return [...maps]
    .map(([map, e]) => ({
      map,
      ...splitMapLabel(map),
      ...rates(e.total),
      rows: [...e.rows.values()]
        .map(({ row, tally }) => ({ ...row, ...rates(tally) }))
        .filter((row) => row.activeSeconds >= MIN_RECORD_SECONDS)
        .sort((a, b) => (b.level ?? 0) - (a.level ?? 0) || value(b) - value(a)),
    }))
    .sort((a, b) => value(b) - value(a));
}

export interface MapStats {
  map: string;
  name: string;
  sessions: number;
  activeSeconds: number;
  expPerHour: number;
  /** Null when the mesos were never looked at twice there, which isn't the same as earning none. */
  mesosPerHour: number | null;
  /** The best single session's rate on the map, with when and at what level. */
  best: { sessionId: number; expPerHour: number; at: number; level: number | null; party: number | null } | null;
  /** The best single session's mesos rate on the map, where the mesos were seen for most of it. */
  bestMesos: { sessionId: number; mesosPerHour: number; at: number; level: number | null } | null;
  /** Rates at each character level. */
  byLevel: { level: number | null; activeSeconds: number; expPerHour: number; mesosPerHour: number | null }[];
}

/** Rates per map over the saved sessions that pass the filter, best EXP rate first. */
export function mapStats(sessions: SavedSession[], filter: LineFilter = {}): MapStats[] {
  const maps = new Map<
    string,
    { ids: Set<number>; seconds: number; exp: number; mesos: number; mesosSeconds: number; best: MapStats["best"]; bestMesos: MapStats["bestMesos"]; levels: Map<number | null, { seconds: number; exp: number; mesos: number; mesosSeconds: number }> }
  >();
  for (const session of sessions) {
    // A session's lines on one map, summed, give its rate there.
    const perMap = new Map<string, { seconds: number; exp: number; mesos: number; mesosSeconds: number; level: number | null; party: number | null }>();
    for (const line of session.lines) {
      if (!keep(session, line, filter)) continue;
      const map = line.map!;
      const entry = maps.get(map) ?? { ids: new Set(), seconds: 0, exp: 0, mesos: 0, mesosSeconds: 0, best: null, bestMesos: null, levels: new Map() };
      entry.ids.add(session.id);
      entry.seconds += line.activeSeconds;
      entry.exp += line.expGained;
      entry.mesos += line.mesosGained;
      entry.mesosSeconds += line.mesosSeconds;
      const level = entry.levels.get(line.level) ?? { seconds: 0, exp: 0, mesos: 0, mesosSeconds: 0 };
      level.seconds += line.activeSeconds;
      level.exp += line.expGained;
      level.mesos += line.mesosGained;
      level.mesosSeconds += line.mesosSeconds;
      entry.levels.set(line.level, level);
      maps.set(map, entry);
      const run = perMap.get(map) ?? { seconds: 0, exp: 0, mesos: 0, mesosSeconds: 0, level: line.level, party: line.party };
      run.seconds += line.activeSeconds;
      run.exp += line.expGained;
      run.mesos += line.mesosGained;
      run.mesosSeconds += line.mesosSeconds;
      perMap.set(map, run);
    }
    for (const [map, run] of perMap) {
      // A run under ten minutes is too short to call a best.
      if (run.seconds < 600) continue;
      const rate = (run.exp * 3600) / run.seconds;
      const entry = maps.get(map)!;
      if (!entry.best || rate > entry.best.expPerHour)
        entry.best = { sessionId: session.id, expPerHour: rate, at: session.startedAt ?? session.savedAt, level: run.level, party: run.party };
      // A mesos best needs the mesos in view for most of the run.
      if (run.mesosSeconds >= Math.max(600, run.seconds * 0.5)) {
        const mesos = (run.mesos * 3600) / run.mesosSeconds;
        if (!entry.bestMesos || mesos > entry.bestMesos.mesosPerHour)
          entry.bestMesos = { sessionId: session.id, mesosPerHour: mesos, at: session.startedAt ?? session.savedAt, level: run.level };
      }
    }
  }
  return [...maps]
    .map(([map, e]) => ({
      map,
      name: splitMapLabel(map).name,
      sessions: e.ids.size,
      activeSeconds: e.seconds,
      expPerHour: (e.exp * 3600) / e.seconds,
      mesosPerHour: e.mesosSeconds > 0 ? (e.mesos * 3600) / e.mesosSeconds : null,
      best: e.best,
      bestMesos: e.bestMesos,
      byLevel: [...e.levels]
        .map(([level, l]) => ({
          level,
          activeSeconds: l.seconds,
          expPerHour: (l.exp * 3600) / l.seconds,
          mesosPerHour: l.mesosSeconds > 0 ? (l.mesos * 3600) / l.mesosSeconds : null,
        }))
        .sort((a, b) => (a.level ?? 0) - (b.level ?? 0)),
    }))
    .filter((m) => m.activeSeconds > 0)
    .sort((a, b) => b.expPerHour - a.expPerHour);
}

export function mapBars(stats: MapStats[], metric: Metric): Bar[] {
  return [...stats]
    .sort((a, b) => (metric === "exp" ? b.expPerHour - a.expPerHour : (b.mesosPerHour ?? -1) - (a.mesosPerHour ?? -1)))
    .map((m) => ({
      id: m.map,
      label: m.name,
      sub: `${formatDuration(m.activeSeconds)} · ${m.sessions} session${m.sessions === 1 ? "" : "s"}`,
      value: (metric === "exp" ? m.expPerHour : m.mesosPerHour) ?? 0,
      // A map whose mesos were never seen has no rate, not a rate of zero.
      display: formatCompact(metric === "exp" ? m.expPerHour : m.mesosPerHour),
      faded: m.activeSeconds < SHORT_SECONDS,
    }));
}

/**
 * Rates from saved sessions at or just below a level, weighted by time: what the player gets now
 * when no session is running. Null with under ten minutes of such play.
 */
export function rateNear(sessions: SavedSession[], level: number): { expPerHour: number | null; percentPerHour: number | null } | null {
  let seconds = 0;
  let exp = 0;
  let percent = 0;
  for (const s of sessions)
    for (const l of s.lines)
      if (l.level != null && l.level <= level && l.level >= level - 2 && l.activeSeconds > 0) {
        seconds += l.activeSeconds;
        exp += l.expGained;
        percent += l.percentGained;
      }
  if (seconds < 600) return null;
  return { expPerHour: exp > 0 ? (exp * 3600) / seconds : null, percentPerHour: percent > 0 ? (percent * 3600) / seconds : null };
}

/** Active play per day over the last two weeks, counting only days played. */
export function dailyPace(sessions: SavedSession[], now = Date.now() / 1000): number | null {
  const recent = days(sessions.filter((s) => (s.startedAt ?? s.savedAt) >= now - 14 * 86400));
  if (!recent.length) return null;
  return recent.reduce((a, d) => a + d.activeSeconds, 0) / recent.length;
}

export interface GoalEstimate {
  /** Levels left, counting the part of this one still to go. */
  levels: number;
  /** Active play to reach the goal at the rate given; null with no rate. */
  seconds: number | null;
  /** "exp" when the EXP each level takes is known or carried on from the levels seen; "percent"
   *  when each level is taken to need as long as this one. */
  method: "exp" | "percent" | null;
  /** Some levels' EXP was carried on from the levels seen rather than seen. */
  extrapolated: boolean;
}

/**
 * Time left to a level goal. With the EXP each level takes (learned from the HUD), the EXP left
 * over the EXP rate; levels not seen yet grow from the last one seen by the average step between
 * the levels seen. Without that, each level is taken to need as much of an hour as this one, from
 * the percent rate.
 */
export function goalEstimate(o: {
  level: number;
  percent: number | null;
  target: number;
  table: LevelExp[];
  expPerHour: number | null;
  percentPerHour: number | null;
}): GoalEstimate {
  const done = (o.percent ?? 0) / 100;
  const levels = Math.max(0, o.target - o.level - done);
  if (levels <= 0) return { levels: 0, seconds: 0, method: null, extrapolated: false };
  const known = new Map(o.table.map((t) => [t.level, t.needed]));
  const steps: number[] = [];
  for (const t of o.table) {
    const next = known.get(t.level + 1);
    if (next && t.needed > 0) steps.push(next / t.needed);
  }
  const growth = steps.length
    ? Math.min(1.6, Math.max(1, Math.exp(steps.reduce((a, r) => a + Math.log(r), 0) / steps.length)))
    : null;
  let extrapolated = false;
  const needed = (level: number): number | null => {
    const seen = known.get(level);
    if (seen) return seen;
    if (growth == null) return null;
    const below = o.table.filter((t) => t.level < level).at(-1);
    if (!below) return null;
    extrapolated = true;
    return below.needed * growth ** (level - below.level);
  };
  if (o.expPerHour && o.expPerHour > 0) {
    let exp = 0;
    let ok = true;
    for (let l = o.level; l < o.target; l++) {
      const n = needed(l);
      if (n == null) {
        ok = false;
        break;
      }
      exp += l === o.level ? n * (1 - done) : n;
    }
    if (ok) return { levels, seconds: (exp / o.expPerHour) * 3600, method: "exp", extrapolated };
  }
  if (o.percentPerHour && o.percentPerHour > 0)
    return { levels, seconds: ((levels * 100) / o.percentPerHour) * 3600, method: "percent", extrapolated: false };
  return { levels, seconds: null, method: null, extrapolated: false };
}

export interface Day {
  /** Local midnight, Unix seconds. */
  start: number;
  activeSeconds: number;
  expGained: number;
  mesosGained: number;
  sessions: number;
}

/** Totals per calendar day for the sessions given, oldest first, days without play left out. */
export function days(sessions: SavedSession[]): Day[] {
  const byDay = new Map<number, Day>();
  for (const s of sessions) {
    const date = new Date((s.startedAt ?? s.savedAt) * 1000);
    date.setHours(0, 0, 0, 0);
    const start = date.getTime() / 1000;
    const day = byDay.get(start) ?? { start, activeSeconds: 0, expGained: 0, mesosGained: 0, sessions: 0 };
    day.activeSeconds += s.activeSeconds;
    day.expGained += s.expGained;
    day.mesosGained += s.mesosGained;
    day.sessions += 1;
    byDay.set(start, day);
  }
  return [...byDay.values()].sort((a, b) => a.start - b.start);
}

export function shortDay(unix: number): string {
  const date = new Date(unix * 1000);
  return `${date.toLocaleDateString(undefined, { weekday: "short" })} ${date.getDate()}`;
}

export function longDay(unix: number): string {
  const date = new Date(unix * 1000);
  return `${shortDay(unix)} ${date.toLocaleDateString(undefined, { month: "short" })}`;
}

export function clock(unix: number): string {
  return new Date(unix * 1000).toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" });
}
