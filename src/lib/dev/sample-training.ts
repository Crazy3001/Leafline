// Made-up training history for the dev mock: a week of sessions on the sample world's maps, with
// timelines, so the Train tab and the training log can be designed without the game.

import type { SavedSession, SessionEvent, SessionLine, SessionSummary, SessionTimeline, TimelinePoint } from "../types";

const MEADOW_I = "Fern Road : Mossbank Meadow I";
const MEADOW_II = "Fern Road : Mossbank Meadow II";
const DEN = "Fern Road : Burrow Den";

interface Plan {
  daysAgo: number;
  hour: number;
  /** Active minutes on each map, in order. */
  maps: [string, number][];
  expPerHour: number;
  mesosPerHour: number;
  level: number;
  /** Active minutes at which the character levelled up. */
  ups: number[];
  deaths: number[];
  party?: number;
  note?: string;
}

const PLANS: Plan[] = [
  { daysAgo: 7, hour: 21, maps: [[DEN, 75]], expPerHour: 15_900, mesosPerHour: 7_200, level: 25, ups: [38], deaths: [] },
  { daysAgo: 6, hour: 18, maps: [[DEN, 82]], expPerHour: 16_800, mesosPerHour: 7_700, level: 26, ups: [], deaths: [] },
  { daysAgo: 5, hour: 20, maps: [[MEADOW_I, 70]], expPerHour: 18_900, mesosPerHour: 9_000, level: 26, ups: [44], deaths: [] },
  { daysAgo: 4, hour: 15, maps: [[MEADOW_I, 48]], expPerHour: 18_200, mesosPerHour: 8_400, level: 27, ups: [], deaths: [20], party: 2, note: "Duo with a friend." },
  {
    daysAgo: 3,
    hour: 22,
    maps: [[MEADOW_I, 15], [MEADOW_II, 108]],
    expPerHour: 26_100,
    mesosPerHour: 10_200,
    level: 27,
    ups: [70],
    deaths: [],
    note: "Best run so far. Channel 4 was empty.",
  },
  { daysAgo: 2, hour: 19, maps: [[MEADOW_I, 92]], expPerHour: 19_400, mesosPerHour: 9_600, level: 28, ups: [], deaths: [] },
  { daysAgo: 1, hour: 21, maps: [[MEADOW_II, 65]], expPerHour: 21_200, mesosPerHour: 11_800, level: 28, ups: [51], deaths: [] },
];

/** A smooth-ish made-up rate: a slow start, some wobble, dips at map changes and deaths. */
function timeline(plan: Plan, seed: number): SessionTimeline {
  const total = plan.maps.reduce((a, [, m]) => a + m, 0);
  const starts: [string, number][] = [];
  let at = 0;
  for (const [map, minutes] of plan.maps) {
    starts.push([map, at]);
    at += minutes;
  }
  const mapAt = (m: number) => [...starts].reverse().find(([, s]) => m >= s)?.[0] ?? null;
  const points: TimelinePoint[] = [];
  for (let s = 30; s <= total * 60; s += 30) {
    const m = s / 60;
    let factor = 0.84 + 0.16 * (1 - Math.exp(-m / 8)) + 0.045 * Math.sin(m * 0.9 + seed) + 0.025 * Math.sin(m * 2.1 + seed * 2);
    for (const [, start] of starts.slice(1)) if (Math.abs(m - start) < 2.5) factor -= 0.22;
    for (const d of plan.deaths) if (m - d >= 0 && m - d < 2) factor -= 0.18;
    const level = plan.level + plan.ups.filter((u) => m >= u).length;
    points.push({
      at: s,
      expPerHour: Math.max(0, plan.expPerHour * factor),
      percentPerHour: Math.max(0, plan.expPerHour * factor) / 1000,
      mesosPerHour: Math.max(0, plan.mesosPerHour * (factor + 0.03 * Math.sin(m * 1.3 + seed))),
      level,
      map: mapAt(m),
    });
  }
  const events: SessionEvent[] = [
    ...plan.ups.map((m, i) => ({ at: m * 60, kind: "levelUp" as const, level: plan.level + i + 1 })),
    ...plan.deaths.map((m) => ({ at: m * 60, kind: "death" as const, level: null })),
  ].sort((a, b) => a.at - b.at);
  return { points, events };
}

const now = Date.now() / 1000;
const today = new Date();
today.setHours(0, 0, 0, 0);

function startOf(plan: Plan) {
  return today.getTime() / 1000 - plan.daysAgo * 86400 + plan.hour * 3600;
}

export const sampleTimelines = new Map<number, SessionTimeline>();

export const sampleSavedSessions: SavedSession[] = PLANS.map((plan, i) => {
  const id = i + 1;
  const active = plan.maps.reduce((a, [, m]) => a + m, 0) * 60;
  const startedAt = startOf(plan);
  sampleTimelines.set(id, timeline(plan, id));
  const lines: SessionLine[] = plan.maps.map(([map, minutes], j) => {
    const rate = plan.expPerHour * (j === 0 && plan.maps.length > 1 ? 0.76 : 1.03);
    return {
      map,
      town: false,
      level: plan.level,
      party: plan.party ?? 1,
      job: "Crossbowman",
      activeSeconds: minutes * 60,
      expGained: Math.round((rate * minutes) / 60),
      percentGained: (rate * minutes) / 60 / 1000,
      mesosGained: Math.round((plan.mesosPerHour * minutes) / 60),
      mesosSeconds: minutes * 60,
    };
  });
  // Most active time first, as the backend lists them.
  lines.sort((a, b) => b.activeSeconds - a.activeSeconds);
  return {
    id,
    label: "",
    savedAt: Math.min(now, startedAt + active * 1.25),
    startedAt,
    level: plan.level + plan.ups.length,
    levelUps: plan.ups.length,
    activeSeconds: active,
    elapsedSeconds: active * 1.25,
    expGained: Math.round((plan.expPerHour * active) / 3600),
    expPerHour: plan.expPerHour,
    mesosGained: Math.round((plan.mesosPerHour * active) / 3600),
    mesosPerHour: plan.mesosPerHour,
    percentPerHour: plan.expPerHour / 1000,
    job: "Crossbowman",
    character: "Leafling",
    deaths: plan.deaths.length,
    note: plan.note ?? "",
    lines,
    maps: lines.map((l) => ({ map: l.map!, activeSeconds: l.activeSeconds, expGained: l.expGained, mesosGained: l.mesosGained })),
  };
});

/** The live sample session, on the sample maps, with a timeline. */
export function withTimeline(session: SessionSummary): SessionSummary {
  const plan: Plan = {
    daysAgo: 0,
    hour: 14,
    maps: [[MEADOW_II, 12], [MEADOW_I, 45]],
    expPerHour: 22_600,
    mesosPerHour: 10_500,
    level: 29,
    ups: [40],
    deaths: [32],
  };
  const { points, events } = timeline(plan, 9);
  const maps = [
    { map: MEADOW_I, activeSeconds: 45 * 60, expGained: 17_660, expPerHour: 23_550, mesosGained: 8_180, mesosPerHour: 10_900 },
    { map: MEADOW_II, activeSeconds: 12 * 60, expGained: 3_960, expPerHour: 19_800, mesosGained: 1_840, mesosPerHour: 9_200 },
    { map: "Victoria Road : Henesys", activeSeconds: 15, expGained: 0, expPerHour: null, mesosGained: 0, mesosPerHour: null },
  ];
  return {
    ...session,
    map: MEADOW_I,
    name: "Leafling",
    job: "Crossbowman",
    levelUps: 1,
    paused: false,
    timeline: points,
    events,
    maps,
    lines: maps.map((m) => ({
      map: m.map,
      town: m.map.endsWith("Henesys"),
      level: 30,
      party: 1,
      job: "Crossbowman",
      activeSeconds: m.activeSeconds,
      expGained: m.expGained,
      percentGained: m.expGained / 1000,
      mesosGained: m.mesosGained,
      mesosSeconds: m.activeSeconds,
    })),
  };
}
