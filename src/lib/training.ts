// Sums over saved sessions for the Train tab and the training log: rates per map, by level and
// party, the best run on a map, days played, and the graph data from a session's timeline.

import type { Bar, ChartPoint } from "./ui/types";
import { formatCompact, formatDuration, splitMapLabel } from "./format";
import type { SavedSession, SessionEvent, SessionLine, SessionSummary, TimelinePoint } from "./types";

/** Less time than this on a map makes its rate a rough guess. */
export const SHORT_SECONDS = 30 * 60;

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
  return timeline.map((p) => ({
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
}

function keep(session: SavedSession, line: SessionLine, filter: LineFilter) {
  if (filter.character && session.character && session.character !== filter.character) return false;
  if (filter.since && (session.startedAt ?? session.savedAt) < filter.since) return false;
  if (filter.levels && (line.level == null || line.level < filter.levels[0] || line.level > filter.levels[1])) return false;
  const solo = (line.party ?? 1) <= 1;
  if (filter.party === "solo" && !solo) return false;
  if (filter.party === "party" && solo) return false;
  return line.map != null && line.activeSeconds > 0;
}

export interface MapStats {
  map: string;
  name: string;
  sessions: number;
  activeSeconds: number;
  expPerHour: number;
  mesosPerHour: number;
  /** The best single session's rate on the map, with when and at what level. */
  best: { sessionId: number; expPerHour: number; at: number; level: number | null; party: number | null } | null;
  /** Rates at each character level. */
  byLevel: { level: number | null; activeSeconds: number; expPerHour: number; mesosPerHour: number }[];
}

/** Rates per map over the saved sessions that pass the filter, best EXP rate first. */
export function mapStats(sessions: SavedSession[], filter: LineFilter = {}): MapStats[] {
  const maps = new Map<
    string,
    { ids: Set<number>; seconds: number; exp: number; mesos: number; mesosSeconds: number; best: MapStats["best"]; levels: Map<number | null, { seconds: number; exp: number; mesos: number; mesosSeconds: number }> }
  >();
  for (const session of sessions) {
    // A session's lines on one map, summed, give its rate there.
    const perMap = new Map<string, { seconds: number; exp: number; level: number | null; party: number | null }>();
    for (const line of session.lines) {
      if (!keep(session, line, filter)) continue;
      const map = line.map!;
      const entry = maps.get(map) ?? { ids: new Set(), seconds: 0, exp: 0, mesos: 0, mesosSeconds: 0, best: null, levels: new Map() };
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
      const run = perMap.get(map) ?? { seconds: 0, exp: 0, level: line.level, party: line.party };
      run.seconds += line.activeSeconds;
      run.exp += line.expGained;
      perMap.set(map, run);
    }
    for (const [map, run] of perMap) {
      // A run under ten minutes is too short to call a best.
      if (run.seconds < 600) continue;
      const rate = (run.exp * 3600) / run.seconds;
      const entry = maps.get(map)!;
      if (!entry.best || rate > entry.best.expPerHour)
        entry.best = { sessionId: session.id, expPerHour: rate, at: session.startedAt ?? session.savedAt, level: run.level, party: run.party };
    }
  }
  return [...maps]
    .map(([map, e]) => ({
      map,
      name: splitMapLabel(map).name,
      sessions: e.ids.size,
      activeSeconds: e.seconds,
      expPerHour: (e.exp * 3600) / e.seconds,
      mesosPerHour: e.mesosSeconds > 0 ? (e.mesos * 3600) / e.mesosSeconds : 0,
      best: e.best,
      byLevel: [...e.levels]
        .map(([level, l]) => ({
          level,
          activeSeconds: l.seconds,
          expPerHour: (l.exp * 3600) / l.seconds,
          mesosPerHour: l.mesosSeconds > 0 ? (l.mesos * 3600) / l.mesosSeconds : 0,
        }))
        .sort((a, b) => (a.level ?? 0) - (b.level ?? 0)),
    }))
    .filter((m) => m.activeSeconds > 0)
    .sort((a, b) => b.expPerHour - a.expPerHour);
}

export function mapBars(stats: MapStats[], metric: Metric): Bar[] {
  return [...stats]
    .sort((a, b) => (metric === "exp" ? b.expPerHour - a.expPerHour : b.mesosPerHour - a.mesosPerHour))
    .map((m) => ({
      id: m.map,
      label: m.name,
      sub: `${formatDuration(m.activeSeconds)} · ${m.sessions} session${m.sessions === 1 ? "" : "s"}`,
      value: metric === "exp" ? m.expPerHour : m.mesosPerHour,
      display: formatCompact(metric === "exp" ? m.expPerHour : m.mesosPerHour),
      faded: m.activeSeconds < SHORT_SECONDS,
    }));
}

/**
 * How the player's measured rates compare with the map data's base EXP per clear, weighted by
 * time played: EXP an hour per base point. Estimates for maps not tried yet are this times their
 * base EXP. Null until at least ten minutes are measured on maps the data knows.
 */
export function rateRatio(measured: { expPerClear: number; expPerHour: number; activeSeconds: number }[]): number | null {
  let weighted = 0;
  let seconds = 0;
  for (const m of measured) {
    if (m.expPerClear <= 0 || m.activeSeconds <= 0) continue;
    weighted += (m.expPerHour / m.expPerClear) * m.activeSeconds;
    seconds += m.activeSeconds;
  }
  return seconds >= 600 ? weighted / seconds : null;
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
