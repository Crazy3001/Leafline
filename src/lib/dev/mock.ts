// Stand-in backend for designing the UI in a plain browser (`npm run dev`). Images come from
// the git-ignored test-screenshots folder; every analysis returns a saved real result.

import type { Backend } from "../api";
import type {
  Analysis,
  BaseStats,
  ItemNote,
  SavedSpot,
  UpdateStatus,
  Build,
  BuildCatalog,
  BuildPlan,
  CharacterRead,
  Expansion,
  GearItem,
  ItemLink,
  MonsterLink,
  PlaceLink,
  ItemRecord,
  LevelStep,
  MapRoute,
  History,
  SavedSession,
  DisplaySettings,
  GameWindow,
  LiveStatus,
  TravelFile,
  TravelInfo,
  PriceEntry,
  PriceLog,
  Guide,
  TrainingSpots,
  MapDataStatus,
  MapMatch,
  OverlayStatus,
  PinStatus,
  QuestRow,
  ReplayProgress,
  SessionSummary,
} from "../types";
import sample from "./sample-analysis.json";
import {
  sampleCard,
  sampleDrops,
  sampleGuide,
  sampleMaps,
  sampleMonsterSheet,
  sampleMonsters,
  sampleNpcSheet,
  sampleNpcs,
  sampleRoute,
  sampleSearch,
  sampleSpots,
} from "./sample-map";
import sampleSession from "./sample-session.json";
import { sampleSavedSessions, sampleTimelines, withTimeline } from "./sample-training";

const screenshots = import.meta.glob("/test-screenshots/*.{png,jpg,jpeg,webp}", {
  query: "?url",
  import: "default",
  eager: true,
}) as Record<string, string>;

/**
 * Readings a screenshot script can put on `window.__LEAFLINE_PREVIEW__` before the page loads,
 * to render mockups with real data. That data lives with the script, never in the repo.
 */
interface Preview {
  analysis?: Analysis;
  session?: SessionSummary;
  map?: MapMatch;
  guide?: Guide;
  spots?: TrainingSpots;
  /** Real routes for the maps the preview's guide names. */
  routes?: MapRoute[];
  pins?: PinStatus;
  /** Start with no readings at all, as on first launch. */
  empty?: boolean;
}
const preview = (globalThis as { __LEAFLINE_PREVIEW__?: Preview }).__LEAFLINE_PREVIEW__;
const sampleAnalysis = preview?.analysis ?? (sample as unknown as Analysis);
const sampleSessionSummary = preview?.session ?? withTimeline(sampleSession as unknown as SessionSummary);

const status: OverlayStatus = {
  visible: true,
  clickThrough: false,
  // A preview can open the compact HUD with `?compact` in the address.
  compact: typeof location !== "undefined" && new URLSearchParams(location.search).has("compact"),
};
const statusHandlers: ((status: OverlayStatus) => void)[] = [];
const analysisHandlers: ((analysis: Analysis) => void)[] = [];
const progressHandlers: ((progress: ReplayProgress) => void)[] = [];
const sessionHandlers: ((session: SessionSummary) => void)[] = [];
let replayCancelled = false;

// Starts without map data, so the download prompt can be designed too.
let mapStatus: MapDataStatus = preview?.map
  ? { ready: true, clientBuild: "preview", dataUpdatedAt: null, checkedAt: null, files: [] }
  : { ready: false, clientBuild: null, dataUpdatedAt: null, checkedAt: null, files: [] };
const mapDataHandlers: ((status: MapDataStatus) => void)[] = [];

/** Training spots saved in this browser tab. */
let savedSpots: SavedSpot[] = [];
const spotHandlers: ((spots: SavedSpot[]) => void)[] = [];

/** Notes the player wrote on items, by lower-case name. */
const notes = new Map<string, ItemNote>([
  ["green robin hat", { item: "Green Robin Hat", note: "Keep for a 60% DEX scroll.", updatedAt: Date.now() / 1000 - 86_400 }],
]);

/** Prices logged in this browser tab; made-up starting records for one item. */
const now = Date.now() / 1000;
let prices: PriceEntry[] = [
  { id: 1, item: "Green Robin Hat", upgrades: 1, slots: 6, stats: "DEX +2, Weapon Def. +21", price: 85_000, kind: "traded", note: null, recordedAt: now - 86400 * 6 },
  { id: 2, item: "Green Robin Hat", upgrades: null, slots: 7, stats: "", price: 30_000, kind: "asking", note: "FM shop", recordedAt: now - 86400 * 3 },
  { id: 3, item: "Green Robin Hat", upgrades: 1, slots: 6, stats: "DEX +2, Weapon Def. +21", price: 95_000, kind: "asking", note: null, recordedAt: now - 86400 },
];

function mockLog(item: string, upgrades: number | null, slots: number | null): PriceLog {
  const entries = prices.filter((p) => p.item.toLowerCase() === item.toLowerCase()).sort((a, b) => b.recordedAt - a.recordedAt);
  const summary = (list: PriceEntry[]) => {
    if (!list.length) return null;
    const sorted = list.map((p) => p.price).sort((a, b) => a - b);
    const n = sorted.length;
    return {
      count: n,
      traded: list.filter((p) => p.kind === "traded").length,
      median: n % 2 ? sorted[(n - 1) / 2] : Math.ceil((sorted[n / 2 - 1] + sorted[n / 2]) / 2),
      low: sorted[0],
      high: sorted[n - 1],
      firstAt: Math.min(...list.map((p) => p.recordedAt)),
      lastAt: Math.max(...list.map((p) => p.recordedAt)),
    };
  };
  return {
    item: entries.at(-1)?.item ?? null,
    entries,
    all: summary(entries),
    likeThis: summary(entries.filter((p) => (p.upgrades ?? 0) === (upgrades ?? 0) && p.slots === slots)),
  };
}

/** Saved sessions: a made-up week on the sample world's maps. */
let savedSessions: SavedSession[] = [...sampleSavedSessions];
const historyHandlers: (() => void)[] = [];

function mockHistory(): History {
  const byMap = new Map<string, { sessions: number; seconds: number; exp: number; mesos: number }>();
  for (const session of savedSessions) {
    for (const m of session.maps) {
      const row = byMap.get(m.map) ?? { sessions: 0, seconds: 0, exp: 0, mesos: 0 };
      byMap.set(m.map, { sessions: row.sessions + 1, seconds: row.seconds + m.activeSeconds, exp: row.exp + m.expGained, mesos: row.mesos + m.mesosGained });
    }
  }
  return {
    sessions: [...savedSessions].sort((a, b) => (b.startedAt ?? b.savedAt) - (a.startedAt ?? a.savedAt)),
    maps: [...byMap].map(([map, r]) => ({ map, sessions: r.sessions, activeSeconds: r.seconds, expPerHour: (r.exp * 3600) / r.seconds, mesosPerHour: (r.mesos * 3600) / r.seconds })).sort((a, b) => b.activeSeconds - a.activeSeconds),
  };
}

/** Made-up taxis and scrolls between the sample maps, in the wiki's shape. */
let travelDownloaded = false;
const sampleTravel: TravelFile = {
  about: "",
  fetchedAt: Date.now() / 1000,
  services: [
    {
      kind: "cab",
      name: "Ox Cart",
      note: "Slow ride between the villages.",
      source: "https://mapleclassic.wiki/wiki/Module:RoutePlanner/config.json",
      routes: [
        { from: "Mossbank", to: "Mossbank Meadow II", fare: 120 },
        { from: "Mossbank Meadow II", to: "Mossbank" },
      ],
    },
    {
      kind: "scroll",
      name: "Return Scroll to Mossbank",
      source: "https://mapleclassic.wiki/wiki/Return_Scroll_to_Mossbank",
      routes: [{ to: "Mossbank" }],
    },
    {
      kind: "scroll",
      name: "Return Scroll to Faraway Isle",
      source: "https://mapleclassic.wiki/wiki/Return_Scroll_to_Faraway_Isle",
      routes: [{ to: "Faraway Isle" }],
    },
  ],
};

function mockTravelInfo(): TravelInfo {
  return {
    file: travelDownloaded ? sampleTravel : null,
    origin: travelDownloaded ? "wiki" : "none",
    path: "%APPDATA%\\app.leafline.overlay\\travel.json",
    error: null,
    missing: travelDownloaded && mapStatus.ready ? ["Faraway Isle"] : null,
  };
}

let displaySettings: DisplaySettings = {
  opacity: 1,
  scale: 1,
  alwaysOnTop: true,
  compactShape: "card",
  hotkeys: { enabled: true, keys: {} },
  travel: { enabled: false, cabs: true, vipCabs: false, scrolls: true },
  live: { enabled: true, interval: 2, pauseAfter: 15, window: null },
  framesFolder: null,
  setupDone: true,
  updateCheck: false,
  buildRules: { apPerLevel: 5, spPerLevel: 3, beginnerSpPerLevel: 1, advancementSp: 1, firstJobLevel: 10, secondJobLevel: 30, secondSpBuysFirst: false, gearCounts: true },
};

/** A made-up job tree and equipment, in the shapes the real data has. None of it is game data. */
const mockSkill = (id: number, name: string, jobId: number, maxLevel: number, requiredSkills: { skillId: number; level: number }[] = []) => ({
  id,
  name,
  jobId,
  className: jobId === 400 ? "Drifter" : jobId === 420 ? "Fernblade" : jobId === 410 ? "Mistwalker" : "Beginner",
  jobTier: "",
  maxLevel,
  passive: false,
  description: null,
  requiredSkills,
  levelDescriptions: Array.from({ length: maxLevel }, (_, i) => `Damage ${100 + i * 5}%`),
});
const mockCatalog: BuildCatalog = {
  jobs: [
    { id: 400, name: "Drifter", tier: 1, parent: null, primary: "luk" },
    { id: 410, name: "Mistwalker", tier: 2, parent: 400, primary: "luk" },
    { id: 420, name: "Fernblade", tier: 2, parent: 400, primary: "luk" },
  ],
  skills: [
    mockSkill(1, "Pebble Toss", 0, 3),
    mockSkill(10, "Quick Step", 400, 20),
    mockSkill(11, "Twin Throw", 400, 20, [{ skillId: 10, level: 3 }]),
    mockSkill(12, "Soft Landing", 400, 10),
    mockSkill(20, "Fern Stab", 420, 30),
    mockSkill(21, "Bramble Guard", 420, 20, [{ skillId: 20, level: 5 }]),
    mockSkill(30, "Thin Mist", 410, 30),
  ],
};
const mockGear: GearItem[] = [
  { id: 500, name: "Fern Dagger", slot: "Weapon", gender: null, classes: ["Thief"], weaponType: "Dagger", reqLevel: 30, reqStr: 0, reqDex: 60, reqInt: 0, reqLuk: 0, stats: [{ label: "Attack", value: 28 }] },
  { id: 501, name: "Mossy Cap", slot: "Hat", gender: null, classes: [], weaponType: null, reqLevel: 15, reqStr: 0, reqDex: 0, reqInt: 0, reqLuk: 0, stats: [{ label: "Weapon Def.", value: 12 }] },
];

/** Made-up items of every kind, in the shape the real data has. */
const mockRecord = (gear: GearItem, more: Partial<ItemRecord> = {}): ItemRecord => ({
  kind: "equip",
  reqFame: 0,
  attackSpeed: null,
  upgradeSlots: 7,
  questItem: false,
  untradable: false,
  price: 1200,
  description: null,
  scroll: null,
  ...gear,
  ...more,
});
const blankGear = (id: number, name: string, slot: string): GearItem => ({ id, name, slot, gender: null, classes: [], weaponType: null, reqLevel: 0, reqStr: 0, reqDex: 0, reqInt: 0, reqLuk: 0, stats: [] });
const mockItems: ItemRecord[] = [
  mockRecord(mockGear[0], { attackSpeed: 4 }),
  mockRecord(mockGear[1]),
  mockRecord({ ...blankGear(502, "Bark Vest", "Top"), gender: "male", classes: ["Thief"], reqLevel: 25, reqLuk: 40, stats: [{ label: "Weapon Def.", value: 36 }, { label: "LUK", value: 2 }] }),
  mockRecord({ ...blankGear(503, "Thorn Claw", "Weapon"), classes: ["Thief"], weaponType: "Claw", reqLevel: 30, reqDex: 60, stats: [{ label: "Attack", value: 24 }] }, { attackSpeed: 3 }),
  mockRecord(blankGear(504, "Berry Tonic", "Consume"), { kind: "use", upgradeSlots: null, price: 40, description: "A tonic pressed from red berries.\nRestores 100 HP." }),
  mockRecord(blankGear(505, "Hat Moss Scroll: Lesser", "Scroll"), { kind: "scroll", upgradeSlots: null, price: 1, description: "Makes a hat mossier.\nSuccess rate: 60%, Weapon Def. +2", scroll: { fits: "Hat", tier: "Lesser", successRate: 60 } }),
  mockRecord(blankGear(506, "Moss Stool", "Install"), { kind: "setup", upgradeSlots: null, description: "Somewhere to sit and rest." }),
  mockRecord(blankGear(507, "Snail Tooth", "Etc"), { kind: "etc", upgradeSlots: null, price: 8, questItem: true, untradable: true, description: "A very small tooth." }),
];
const showItemHandlers: ((link: ItemLink) => void)[] = [];
const showQuestHandlers: ((id: number) => void)[] = [];
const showMonsterHandlers: ((link: MonsterLink) => void)[] = [];
const showPlaceHandlers: ((link: PlaceLink) => void)[] = [];
const mockQuests: QuestRow[] = [
  { id: 71, name: "Slime Samples", region: "Fern Isle", levelMin: 1, giver: "Innkeeper Bo", repeat: null, exp: 30, mesos: 0 },
  { id: 70, name: "Shell Sorting", region: "Mossbank", levelMin: 3, giver: "Warden Ilse", repeat: "daily", exp: 120, mesos: 500 },
  // Made-up stand-ins for the job advancement quests the checklist looks for.
  { id: 90, name: "The Drifter's Next Journey", region: "Mossbank", levelMin: 30, giver: "Warden Ilse", repeat: null, exp: 0, mesos: 0 },
  { id: 91, name: "Finding the Instructor", region: "Mossbank", levelMin: 30, giver: "Warden Ilse", repeat: null, exp: 0, mesos: 0 },
];
const characterHandlers: ((read: CharacterRead) => void)[] = [];

/** Made-up builds kept in this browser tab. */
let mockBuilds: (Build & { usedAt: number })[] = [
  {
    id: 1,
    name: "Fernblade, LUK heavy",
    character: null,
    guide: "",
    start: { level: 29, stats: { str: 4, dex: 60, int: 4, luk: 92 }, ap: 0, sp: 0, skills: [{ skillId: 10, level: 20 }, { skillId: 11, level: 17 }] },
    plan: {
      job: 420,
      skills: [{ skillId: 11, level: 20 }, { skillId: 20, level: 20 }, { skillId: 21, level: 10 }, { skillId: 12, level: 10 }],
      stats: [{ stat: "dex", target: 80, fromLevel: null }, { stat: "luk", target: null, fromLevel: null }],
      gear: [500, 501],
      planTo: 70,
    },
    usedAt: 2,
  },
];
const buildHandlers: (() => void)[] = [];
const buildsChanged = () => buildHandlers.forEach((h) => h());

/**
 * A rough stand-in for src-tauri/src/build/plan.rs, only for designing the pages: the same shape
 * of answer, without the tier, prerequisite and problem rules. The tests cover the real one.
 */
function mockExpand(build: Build): Expansion {
  const rules = displaySettings.buildRules;
  const stats: BaseStats = { ...(build.start.stats ?? { str: 0, dex: 0, int: 0, luk: 0 }) };
  const skills = new Map(build.start.skills.map((s) => [s.skillId, s.level]));
  const steps: LevelStep[] = [];
  for (let level = build.start.level; level <= build.plan.planTo; level++) {
    const step: LevelStep = { level, advance: level === rules.secondJobLevel ? "2nd job: Fernblade" : null, ap: [], sp: [], gear: [], worn: [], stats, apLeft: 0, spLeft: 0 };
    let ap = level > build.start.level ? rules.apPerLevel : 0;
    for (const goal of build.plan.stats) {
      const amount = goal.target == null ? ap : Math.min(ap, Math.max(0, goal.target - stats[goal.stat]));
      if (!amount) continue;
      stats[goal.stat] += amount;
      ap -= amount;
      step.ap.push({ stat: goal.stat, amount, total: stats[goal.stat] });
    }
    let sp = level > build.start.level ? rules.spPerLevel : 0;
    for (const goal of build.plan.skills) {
      const have = skills.get(goal.skillId) ?? 0;
      const amount = Math.min(sp, Math.max(0, goal.level - have));
      if (!amount) continue;
      skills.set(goal.skillId, have + amount);
      sp -= amount;
      const name = mockCatalog.skills.find((s) => s.id === goal.skillId)?.name ?? "";
      step.sp.push({ skillId: goal.skillId, name, from: have, to: have + amount });
    }
    steps.push({ ...step, stats: { ...stats }, apLeft: ap, spLeft: sp });
  }
  const gear = build.plan.gear.flatMap((id) => {
    const item = mockGear.find((g) => g.id === id);
    if (!item) return [];
    const at = steps.find((s) => s.level >= item.reqLevel && s.stats.dex >= item.reqDex);
    at?.gear.push(item.name);
    return [{ item, level: at?.level ?? null, usable: true }];
  });
  const problems = [
    ...(build.start.stats ? [] : [{ severity: "warning" as const, level: null, message: "Enter your stats from the Character Stat window, so the totals are right." }]),
    ...(build.plan.job ? [] : [{ severity: "note" as const, level: null, message: "Choose a job to plan its skills." }]),
  ];
  return { steps, problems, gear, skills: [...skills].map(([skillId, level]) => ({ skillId, level })) };
}

const mockPlan = (build: Build): BuildPlan => {
  const { id, name, character, guide, start, plan } = build;
  return { build: { id, name, character, guide, start, plan }, expansion: mockExpand(build) };
};
const newestBuild = () => [...mockBuilds].sort((a, b) => b.usedAt - a.usedAt)[0];

let liveStatus: LiveStatus = { state: "off" };
let liveTimer: ReturnType<typeof setInterval> | undefined;
const liveHandlers: ((status: LiveStatus) => void)[] = [];
const setLive = (status: LiveStatus) => {
  liveStatus = status;
  liveHandlers.forEach((h) => h(status));
};
const mockWindows: GameWindow[] = [
  { id: 1, title: "MapleStory", class: "MapleStoryClass", width: 2560, height: 1440, likelyGame: true },
  { id: 2, title: "Inbox - Mail", class: "ApplicationFrameWindow", width: 1280, height: 900, likelyGame: false },
];
const settingsHandlers: ((settings: DisplaySettings) => void)[] = [];

/** Whether the mock drop table has been downloaded. */
let dropsDownloaded = false;

const pins: PinStatus = preview?.pins ?? { pinned: [], hidden: false };
const pinHandlers: ((status: PinStatus) => void)[] = [];
const pinsChanged = () => pinHandlers.forEach((h) => h({ ...pins, pinned: [...pins.pinned] }));

/** The saved real session, as if `fraction` of it had been replayed so far. */
function partialSession(fraction: number, frames: number): SessionSummary {
  const full = sampleSessionSummary;
  const part = (value: number) => Math.round(value * fraction);
  return {
    ...full,
    frames,
    elapsedSeconds: full.elapsedSeconds * fraction,
    activeSeconds: full.activeSeconds * fraction,
    expGained: part(full.expGained),
    mesosGained: part(full.mesosGained),
    maps: full.maps.map((m) => ({ ...m, activeSeconds: m.activeSeconds * fraction, expGained: part(m.expGained), mesosGained: part(m.mesosGained) })),
  };
}
const delay = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

export const mockBackend: Backend = {
  async analyzeImage() {
    await delay(350);
    const analysis = sampleAnalysis;
    analysisHandlers.forEach((handler) => handler(analysis));
    return analysis;
  },
  listImages: async () => Object.keys(screenshots),
  async replaySession() {
    replayCancelled = false;
    const paths = Object.keys(screenshots);
    let session = partialSession(0, 0);
    for (const [i, path] of paths.entries()) {
      if (replayCancelled) break;
      await delay(450);
      const analysis = sampleAnalysis;
      session = partialSession((i + 1) / paths.length, i + 1);
      progressHandlers.forEach((h) => h({ done: i + 1, total: paths.length, path, at: i, analysis, session }));
      analysisHandlers.forEach((h) => h(analysis));
      sessionHandlers.forEach((h) => h(session));
    }
    return session;
  },
  async cancelReplay() {
    replayCancelled = true;
  },
  imageUrl: async (path) => screenshots[path] ?? "",
  pickImage: async () => Object.keys(screenshots)[0] ?? null,
  pickFolder: async () => "/test-screenshots",
  overlayStatus: async () => status,
  async setClickThrough(enabled) {
    status.clickThrough = enabled;
  },
  async hideOverlay() {},
  async toggleOverlay() {},
  async showWorkbench() {},
  async setHotkeysQuiet() {},
  async onHotkey(handler) {
    // In the browser, `__pressHotkey("ReadItem")` in the console stands in for pressing one.
    const w = window as unknown as { __pressHotkey?: (action: string) => void };
    w.__pressHotkey = handler;
    return () => {
      if (w.__pressHotkey === handler) delete w.__pressHotkey;
    };
  },
  // Saved keys win over the starting ones, and F10 stands in for a key another program holds.
  shortcuts: async () =>
    [
    { action: "ToggleOverlay", keys: "Super+F9", suggestedKeys: "Super+F9", label: "Show or hide the overlay", inSetup: true, taken: false },
    { action: "ToggleClickThrough", keys: "Super+F7", suggestedKeys: "Super+F7", label: "Lock the overlay (click-through)", inSetup: true, taken: false },
    { action: "TogglePins", keys: "", suggestedKeys: "", label: "Show or hide pinned panels", inSetup: false, taken: false },
    { action: "ToggleCompact", keys: "", suggestedKeys: "", label: "Compact HUD", inSetup: true, taken: false },
    { action: "ToggleLive", keys: "", suggestedKeys: "", label: "Start or stop live reading", inSetup: false, taken: false },
    { action: "ReadItem", keys: "F10", suggestedKeys: "Super+F8", label: "Read the item you're hovering", inSetup: true, taken: true },
    { action: "ReadCharacter", keys: "", suggestedKeys: "", label: "Read your stat and skill windows, to update your build", inSetup: true, taken: false },
    { action: "PauseTracking", keys: "", suggestedKeys: "", label: "Pause or resume tracking", inSetup: false, taken: false },
    { action: "NewSession", keys: "", suggestedKeys: "", label: "Start a new session", inSetup: false, taken: false },
    { action: "ToggleLogging", keys: "", suggestedKeys: "", label: "Log this run or not", inSetup: false, taken: false },
    { action: "SaveFrame", keys: "", suggestedKeys: "", label: "Save the picture last read", inSetup: true, taken: false },
    { action: "ShowWorkbench", keys: "", suggestedKeys: "", label: "Open the workbench", inSetup: false, taken: false },
    ].map((s) => {
      const keys = displaySettings.hotkeys.keys[s.action] ?? s.keys;
      return { ...s, keys, taken: keys === "F10" };
    }),
  liveStatus: async () => liveStatus,
  async setLiveHeld(held) {
    if (liveStatus.state === "reading") setLive({ ...liveStatus, held });
  },
  async resetSession() {},
  async setLiveLogging(logging) {
    if (liveStatus.state === "reading") setLive({ ...liveStatus, logging });
  },
  async startLive() {
    if (!displaySettings.live.enabled) throw "Turn on live reading in Settings first.";
    setLive({ state: "waiting", reason: "Starting…" });
    let frames = 0;
    clearInterval(liveTimer);
    liveTimer = setInterval(() => {
      frames += 1;
      setLive({ state: "reading", window: "MapleStory", width: 2560, height: 1440, frames, readMs: 412, hdr: false, held: false, logging: true });
      analysisHandlers.forEach((h) => h(sampleAnalysis));
    }, displaySettings.live.interval * 1000);
  },
  async stopLive() {
    clearInterval(liveTimer);
    setLive({ state: "off" });
  },
  liveWindows: async () => mockWindows,
  saveLiveFrame: async () => "C:\\Users\\you\\AppData\\Local\\app.leafline.overlay\\frames\\leafline-2026-10-30_13-05-09.png",
  async onLive(handler) {
    liveHandlers.push(handler);
    return () => {};
  },
  onFrameSaved: async () => () => {},
  onItemRead: async () => () => {},
  takeLaunchImage: async () => null,
  onOpenImage: async () => () => {},
  async onAnalysis(handler) {
    analysisHandlers.push(handler);
    if (!preview?.empty) setTimeout(() => handler(sampleAnalysis), 200);
    return () => {};
  },
  async onReplayProgress(handler) {
    progressHandlers.push(handler);
    return () => {};
  },
  async onSession(handler) {
    sessionHandlers.push(handler);
    if (!preview?.empty) setTimeout(() => handler(sampleSessionSummary), 200);
    return () => {};
  },
  async onOverlayStatus(handler) {
    statusHandlers.push(handler);
    return () => {};
  },
  mapDataStatus: async () => mapStatus,
  async downloadMapData() {
    await delay(1200);
    const now = Date.now() / 1000;
    const files = ["maps.json", "monsters.json", "npcs.json", "quests.json", "skills.json", "items.json"];
    mapStatus = {
      ready: true,
      clientBuild: "0000000 (sample)",
      dataUpdatedAt: new Date().toISOString(),
      checkedAt: now,
      files: files.map((name) => ({ name, records: 6, updatedAt: new Date().toISOString(), fetchedAt: now })),
    };
    mapDataHandlers.forEach((h) => h(mapStatus));
    return { report: { checked: true, downloaded: files }, status: mapStatus };
  },
  async findMap() {
    if (!mapStatus.ready) return null;
    if (preview?.map) return preview.map;
    const card = sampleCard(2)!;
    return { score: 0.93, card, alternatives: [sampleSearch("Meadow II")[0]] };
  },
  mapCard: async (id) =>
    preview?.map ? (preview.map.card.map.id === id ? preview.map.card : null) : mapStatus.ready ? sampleCard(id) : null,
  searchMaps: async (text) => (mapStatus.ready ? sampleSearch(text) : []),
  mapRoute: async (from, to) =>
    preview?.routes ? (preview.routes.find((r) => r.from.id === from && r.to.id === to) ?? null) : sampleRoute(from, to),
  async monsterDrops(monsters) {
    await delay(900);
    return sampleDrops(monsters);
  },
  async questGuide(quests) {
    if (!mapStatus.ready) return null;
    if (preview?.guide) return preview.guide;
    return sampleGuide(quests, 2, dropsDownloaded);
  },
  async downloadDrops() {
    await delay(900);
    dropsDownloaded = true;
  },
  async trainingSpots(_name, _street, _twin, measured) {
    if (!mapStatus.ready) return null;
    return preview?.spots ?? sampleSpots(2, measured);
  },
  async openLink(url) {
    window.open(url, "_blank", "noopener");
  },
  async onMapData(handler) {
    mapDataHandlers.push(handler);
    return () => {};
  },
  priceLog: async (item, upgrades, slots) => mockLog(item, upgrades, slots),
  async recordPrice(price) {
    prices.push({ ...price, id: Math.max(0, ...prices.map((p) => p.id)) + 1, recordedAt: Date.now() / 1000 });
    return mockLog(price.item, price.upgrades, price.slots);
  },
  async deletePrice(id) {
    prices = prices.filter((p) => p.id !== id);
  },
  savedSpots: async () => [...savedSpots],
  async setSpotSaved(mapId, name, saved) {
    savedSpots = savedSpots.filter((s) => s.mapId !== mapId);
    if (saved) savedSpots.unshift({ mapId, name, savedAt: Date.now() / 1000 });
    spotHandlers.forEach((h) => h([...savedSpots]));
    return [...savedSpots];
  },
  async onSavedSpots(handler) {
    spotHandlers.push(handler);
    return () => {};
  },
  itemNote: async (item) => notes.get(item.toLowerCase()) ?? null,
  async setItemNote(item, note) {
    const text = note.trim();
    if (!text) {
      notes.delete(item.toLowerCase());
      return null;
    }
    const saved = { item, note: text, updatedAt: Date.now() / 1000 };
    notes.set(item.toLowerCase(), saved);
    return saved;
  },
  async saveSession(label) {
    const s = sampleSessionSummary;
    const id = Math.max(0, ...savedSessions.map((x) => x.id)) + 1;
    savedSessions.push({
      id,
      label,
      savedAt: Date.now() / 1000,
      startedAt: s.startedAt,
      level: s.level,
      levelUps: s.levelUps,
      activeSeconds: s.activeSeconds,
      elapsedSeconds: s.elapsedSeconds,
      expGained: s.expGained,
      expPerHour: s.expPerHour,
      mesosGained: s.mesosGained,
      mesosPerHour: s.mesosPerHour,
      percentPerHour: s.percentPerHour,
      job: s.job,
      character: s.name,
      deaths: s.deaths,
      note: "",
      lines: s.lines,
      maps: s.maps
        .filter((m) => m.map && m.activeSeconds > 0)
        .map((m) => ({ map: m.map!, activeSeconds: m.activeSeconds, expGained: m.expGained, mesosGained: m.mesosGained })),
    });
    sampleTimelines.set(id, { points: s.timeline, events: s.events });
    historyHandlers.forEach((h) => h());
    return id;
  },
  sessionHistory: async () => mockHistory(),
  sessionTimeline: async (id) => sampleTimelines.get(id) ?? { points: [], events: [] },
  async showTraining() {
    window.open("/training", "_blank");
  },
  knownMesos: async () => ({ name: "Mossbank", mesos: 36_644, seenAt: Math.floor(Date.now() / 1000) - 26 * 3600 }),
  onKnownMesos: async () => () => {},
  async setSessionNote(id, note) {
    savedSessions = savedSessions.map((s) => (s.id === id ? { ...s, note } : s));
    historyHandlers.forEach((h) => h());
  },
  async renameMap(from, to) {
    savedSessions = savedSessions.map((s) => ({
      ...s,
      maps: s.maps.map((m) => (m.map === from ? { ...m, map: to } : m)),
      lines: s.lines.map((l) => (l.map === from ? { ...l, map: to } : l)),
    }));
    historyHandlers.forEach((h) => h());
  },
  async openDataFolder() {},
  framesFolder: async () => displaySettings.framesFolder ?? "C:\\Users\\you\\AppData\\Local\\app.leafline.overlay\\frames",
  async openFramesFolder() {},
  diagnostics: async () => "Leafline dev (mock)",
  async deleteSession(id) {
    savedSessions = savedSessions.filter((s) => s.id !== id);
    historyHandlers.forEach((h) => h());
  },
  async onHistory(handler) {
    historyHandlers.push(handler);
    return () => {};
  },
  async exportCsv(what) {
    return what === "prices" ? prices.length : savedSessions.length;
  },
  buildCatalog: async () => (mapStatus.ready ? mockCatalog : null),
  starterPlans: async () =>
    mapStatus.ready
      ? [
          {
            id: "fernblade",
            job: 420,
            name: "Dagger Fernblade",
            summary: "A made-up plan, to design the page with.",
            plan: {
              job: 420,
              skills: [{ skillId: 1, level: 3 }, { skillId: 10, level: 20 }, { skillId: 20, level: 30 }],
              stats: [{ stat: "dex", target: 40, fromLevel: 20 }, { stat: "luk", target: null, fromLevel: null }],
              gear: [500],
              planTo: 70,
            },
            missing: [],
          },
        ]
      : [],
  searchGear: async (text) => mockGear.filter((g) => g.name.toLowerCase().includes(text.toLowerCase())),
  async searchItems(query) {
    const words = query.text.toLowerCase().split(/\s+/).filter(Boolean);
    const items = mockItems.filter(
      (item) =>
        (!query.kind || item.kind === query.kind) &&
        (!query.slot || item.slot === query.slot || item.weaponType === query.slot || item.scroll?.fits === query.slot) &&
        (!query.class || (item.kind === "equip" && (!item.classes.length || item.classes.includes(query.class)))) &&
        (query.levelMin == null || item.reqLevel >= query.levelMin) &&
        (query.levelMax == null || item.reqLevel <= query.levelMax) &&
        words.every((word) => item.name.toLowerCase().includes(word)),
    );
    return { items: items.slice(query.offset, query.offset + 40), total: items.length, closest: false };
  },
  itemFilters: async () => ({ slots: ["Weapon", "Hat", "Top"], weapons: ["Claw", "Dagger"] }),
  itemRecord: async (id) => mockItems.find((item) => item.id === id) ?? null,
  findItem: async (name) => mockItems.find((item) => item.name.toLowerCase() === name.toLowerCase()) ?? null,
  async itemOrigin(id) {
    const item = mockItems.find((i) => i.id === id);
    if (!item) return null;
    const guide = sampleGuide([{ title: "Moss Patrol", objectives: [item.name] }], null, true);
    const quest = guide.quests[0];
    return { from: guide.from, origin: { drops: quest.objectives[0], quests: quest.quest ? [quest.quest] : [] } };
  },
  async showItem(id, text) {
    showItemHandlers.forEach((h) => h({ id, text }));
  },
  async onShowItem(handler) {
    showItemHandlers.push(handler);
    return () => {};
  },
  async setGearGoal(item, wanted) {
    const build = newestBuild();
    if (!build) return;
    build.plan.gear = build.plan.gear.filter((id) => id !== item).concat(wanted ? [item] : []);
    buildsChanged();
  },
  async searchQuests(search) {
    const quests = mockQuests.filter((q) => q.name.toLowerCase().includes(search.text.toLowerCase()) && (!search.region || q.region === search.region));
    return { quests, total: quests.length };
  },
  questRegions: async () => ["Fern Isle", "Mossbank"],
  async questSheet(id) {
    const row = mockQuests.find((q) => q.id === id);
    if (!row) return null;
    const quest = sampleGuide([{ title: row.name, objectives: ["Moss Slime"] }], null, true).quests[0];
    return {
      from: null,
      sheet: {
        info: quest.quest!,
        row,
        chain: "Slime Business",
        story: ["Innkeeper Bo wants slime for a recipe.", "Bring back samples from the meadow."],
        after: [{ id: 70, name: "Shell Sorting", levelMin: 1 }],
        next: null,
        objectives: [{ count: 5, guide: quest.objectives[0] }],
        rewards: [{ id: 504, name: "Berry Tonic", count: 5, sure: true }, { id: 502, name: "Bark Vest", count: 1, sure: false }, { id: 503, name: "Thorn Claw", count: 1, sure: false }],
      },
    };
  },
  async showQuest(id) {
    showQuestHandlers.forEach((h) => h(id));
  },
  async onShowQuest(handler) {
    showQuestHandlers.push(handler);
    return () => {};
  },
  searchMonsters: async (query) => sampleMonsters(query),
  monsterFilters: async () => ({ regions: ["Ashlands", "Fern Isle"], elements: ["Holy", "Ice"] }),
  findMonster: async (name) => sampleMonsters({ text: name, kind: null, weakTo: null, region: null, area: null, levelMin: null, levelMax: null, sort: null, near: null, offset: 0 }).monsters[0]?.id ?? null,
  async monsterSheet(id, mapName) {
    await delay(150);
    const sheet = sampleMonsterSheet(id, mapName ? 2 : null, dropsDownloaded);
    return sheet && { from: mapName ? sampleCard(2)!.map : null, sheet };
  },
  async showMonster(id, text) {
    showMonsterHandlers.forEach((h) => h({ id, text }));
  },
  async onShowMonster(handler) {
    showMonsterHandlers.push(handler);
    return () => {};
  },
  browseMaps: async (query) => sampleMaps(query),
  mapRegions: async () => ["Ashlands", "Fern Isle"],
  mapAreas: async () => ["Fern Isle", "Ashlands"],
  searchNpcs: async (query) => sampleNpcs(query),
  findNpc: async (name) => sampleNpcs({ text: name, kind: null, region: null, area: null, offset: 0 }).npcs[0]?.id ?? null,
  async npcSheet(id, mapName) {
    const sheet = sampleNpcSheet(id, mapName ? 2 : null);
    return sheet && { from: mapName ? sampleCard(2)!.map : null, sheet };
  },
  async showPlace(mode, id) {
    showPlaceHandlers.forEach((h) => h({ mode, id }));
  },
  async onShowPlace(handler) {
    showPlaceHandlers.push(handler);
    return () => {};
  },
  async readCharacter() {
    const read = { stats: true, skills: true, build: newestBuild()?.name ?? null, result: { changes: ["LUK 92 → 97"], unknown: [] }, problem: null, automatic: false };
    characterHandlers.forEach((h) => h(read));
  },
  async onCharacterRead(handler) {
    characterHandlers.push(handler);
    return () => {};
  },
  async findGear(who) {
    if (!mapStatus.ready) return null;
    const level = who.level ?? 1;
    const wearable = mockItems.filter((item) => item.kind === "equip" && item.reqLevel <= level);
    const slots = [...new Set(wearable.map((item) => item.slot))].map((slot) => {
      const picks = wearable.filter((item) => item.slot === slot).map((item) => ({ item, goal: false }));
      return { slot, picks, total: picks.length };
    });
    return {
      level,
      job: who.job ?? 420,
      gender: who.gender,
      build: "Fernblade, LUK heavy",
      statsKnown: true,
      firstJobLevel: 10,
      gear: { level, rankedBy: ["Attack", "LUK", "DEX", "Weapon Def."], slots },
    };
  },
  async gearSuggestions(id, level) {
    const build = mockBuilds.find((b) => b.id === id);
    if (!build) return null;
    const wearable = mockItems.filter((item) => item.kind === "equip" && item.reqLevel <= level);
    const slots = [...new Set(wearable.map((item) => item.slot))].map((slot) => {
      const picks = wearable.filter((item) => item.slot === slot).map((item) => ({ item, goal: build.plan.gear.includes(item.id) }));
      return { slot, picks, total: picks.length };
    });
    return { level, rankedBy: ["Attack", "LUK", "DEX", "Weapon Def."], slots };
  },
  listBuilds: async () =>
    [...mockBuilds].sort((a, b) => b.usedAt - a.usedAt).map(({ id, name, character }) => ({ id, name, character, updatedAt: now })),
  async buildPlan(id) {
    const build = mockBuilds.find((b) => b.id === id);
    return build ? mockPlan(build) : null;
  },
  async activeBuild() {
    const build = newestBuild();
    return mapStatus.ready && build ? mockPlan(build) : null;
  },
  async saveBuild(build) {
    const usedAt = Date.now();
    const id = build.id || Math.max(0, ...mockBuilds.map((b) => b.id)) + 1;
    mockBuilds = [...mockBuilds.filter((b) => b.id !== id), { ...build, id, usedAt }];
    buildsChanged();
    return mockPlan({ ...build, id });
  },
  async useBuild(id) {
    mockBuilds = mockBuilds.map((b) => (b.id === id ? { ...b, usedAt: Date.now() } : b));
    buildsChanged();
  },
  async deleteBuild(id) {
    mockBuilds = mockBuilds.filter((b) => b.id !== id);
    buildsChanged();
  },
  exportBuild: async (id) => `LEAFLINE-BUILD-1:mock${id}`,
  async importBuild() {
    throw "That isn't a Leafline build code.";
  },
  async buildItemFit(item, level) {
    const build = newestBuild();
    if (!build) return null;
    const at = level ?? build.start.level;
    const need = item.requirements.level ?? 0;
    return { level: at, jobOk: null, wearableNow: at >= need, wearableAt: need, short: at >= need ? [] : [{ label: "Lv", amount: need - at }], unread: false };
  },
  async openGuide(url) {
    window.open(url, "_blank", "noopener");
  },
  async showBuild() {
    window.open("/build", "_blank");
  },
  async onBuilds(handler) {
    buildHandlers.push(handler);
    return () => {};
  },
  currentReadings: async () => ({
    analysis: preview?.empty ? null : sampleAnalysis,
    session: preview?.empty ? null : sampleSessionSummary,
  }),
  pinStatus: async () => pins,
  async pinPanel(panel) {
    if (!pins.pinned.includes(panel)) pins.pinned.push(panel);
    pinsChanged();
    return pins;
  },
  async unpinPanel(panel) {
    pins.pinned = pins.pinned.filter((p) => p !== panel);
    pinsChanged();
    return pins;
  },
  async onPins(handler) {
    pinHandlers.push(handler);
    return () => {};
  },
  onPinResized: async () => () => {},
  settings: async () => displaySettings,
  async updateSettings(settings) {
    const checkChanged = settings.updateCheck !== displaySettings.updateCheck;
    displaySettings = { ...settings };
    settingsHandlers.forEach((h) => h(displaySettings));
    if (checkChanged) (settings.updateCheck ? mockUpdateCheck() : setUpdate({ state: "off" }));
    return displaySettings;
  },
  async onSettings(handler) {
    settingsHandlers.push(handler);
    return () => {};
  },
  travelInfo: async () => mockTravelInfo(),
  async downloadTravel() {
    await delay(900);
    travelDownloaded = true;
    return mockTravelInfo();
  },
  async editTravelFile() {},
  async setCompact(enabled) {
    status.compact = enabled;
    statusHandlers.forEach((h) => h({ ...status }));
  },
  async showSettings() {
    window.open("/settings", "_blank");
  },
  async showSetup() {
    window.open("/setup", "_blank");
  },
  async finishSetup() {},
  version: async () => "dev",
  updateStatus: async () => updateStatus,
  async checkForUpdate() {
    if (!displaySettings.updateCheck) throw "Turn on the update check first.";
    mockUpdateCheck();
    return updateStatus;
  },
  async onUpdateStatus(handler) {
    updateHandlers.push(handler);
    return () => {};
  },
  async fitWindow() {},
};

/** The update check in a browser: a made-up newer release, after a moment. */
let updateStatus: UpdateStatus = { state: "off" };
const updateHandlers: ((status: UpdateStatus) => void)[] = [];

function setUpdate(status: UpdateStatus) {
  updateStatus = status;
  updateHandlers.forEach((h) => h(status));
}

function mockUpdateCheck() {
  setUpdate({ state: "checking" });
  setTimeout(() => {
    if (displaySettings.updateCheck)
      setUpdate({ state: "available", current: "0.1.0", latest: "0.2.0", url: "https://github.com/OWNER/leafline/releases" });
  }, 700);
}
