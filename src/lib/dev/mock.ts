// Stand-in backend for designing the UI in a plain browser (`npm run dev`). Every analysis
// returns a saved real result.

import type { Backend } from "../api";
import type {
  Analysis,
  BaseStats,
  ItemNote,
  LoggedDrop,
  MapNote,
  SavedSpot,
  PotionSlot,
  SessionPotion,
  Destination,
  NavState,
  UpdateStatus,
  Build,
  BuildCatalog,
  BuildPlan,
  CharacterRead,
  Expansion,
  GearItem,
  GearPiece,
  Outfit,
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
  Guide,
  TrainingSpots,
  MapDataStatus,
  MapMatch,
  OverlayStatus,
  Panel,
  PinStatus,
  DeskStatus,
  Trip,
  TripMessage,
  QuestRow,
  SessionSummary,
} from "../types";
import type { BuffRow, PlayerDamage, SkillEffects, StatSheet, StatsView } from "../types";
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
const sessionHandlers: ((session: SessionSummary) => void)[] = [];

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


/** Saved sessions: a made-up week on the sample world's maps. */
let savedSessions: SavedSession[] = [...sampleSavedSessions];
/** The player's map notes; one to start with on Mossbank Meadow II. */
const mapNotes = new Map<number, MapNote>([[3, { mapId: 3, map: "Mossbank Meadow II", note: "Left platform is best. Channel 4 is quiet after 9.", updatedAt: Date.now() / 1000 - 86400 }]]);
const mapNoteHandlers: ((mapId: number) => void)[] = [];
/** The drop log: a few made-up drops inside the newest sample session. */
const dropStart = (sampleSavedSessions[0]?.startedAt ?? Date.now() / 1000 - 7200) + 600;
const sampleDrop = (id: number, item: string, monster: string, monsterId: number, minutes: number): LoggedDrop => ({
  id,
  item,
  itemId: null,
  monster,
  monsterId,
  map: "Mossbank Meadow I",
  mapId: 2,
  character: "Leafling",
  seenAt: dropStart + minutes * 60,
});
let drops: LoggedDrop[] = [
  sampleDrop(1, "Fern Dagger", "Moss Slime", 11, 4),
  sampleDrop(2, "Blue Tonic", "Pebble Snail", 10, 9),
  sampleDrop(3, "Blue Tonic", "Moss Slime", 11, 17),
  sampleDrop(4, "Thistle Cap", "Thistle Sprite", 13, 26),
];
const dropHandlers: (() => void)[] = [];
const historyHandlers: (() => void)[] = [];

// Potion slots and per-session counts, as the backend keeps them.
// Slots by lower-cased character name; the empty name is the shared set.
const potionSlotSets = new Map<string, PotionSlot[]>();
const slotsFor = (character: string | null) => {
  const own = potionSlotSets.get((character ?? "").trim().toLowerCase()) ?? [];
  return own.length || !character ? own : (potionSlotSets.get("") ?? []);
};
const sessionPotions = new Map<number, SessionPotion[]>();
const potionHandlers: (() => void)[] = [];

function mockHistory(): History {
  const byMap = new Map<string, { sessions: number; seconds: number; exp: number; mesos: number }>();
  for (const session of savedSessions) {
    for (const m of session.maps) {
      const row = byMap.get(m.map) ?? { sessions: 0, seconds: 0, exp: 0, mesos: 0 };
      byMap.set(m.map, { sessions: row.sessions + 1, seconds: row.seconds + m.activeSeconds, exp: row.exp + m.expGained, mesos: row.mesos + m.mesosGained });
    }
  }
  const withPotions = savedSessions.map((s) => {
    const potions = s.startedAt != null ? (sessionPotions.get(s.startedAt) ?? []) : [];
    return { ...s, potions, potionCost: potions.reduce((a, p) => a + p.used * p.price, 0) };
  });
  return {
    sessions: withPotions.sort((a, b) => (b.startedAt ?? b.savedAt) - (a.startedAt ?? a.savedAt)),
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
  keepGameFocus: true,
  compactShape: "card",
  hotkeys: { enabled: true, keys: {} },
  travel: { enabled: false, cabs: true, vipCabs: false, scrolls: true },
  live: { enabled: true, interval: 2, pauseAfter: 15, window: null },
  framesFolder: null,
  setupDone: true,
  updateCheck: false,
  showPictures: true,
  buildRules: { apPerLevel: 5, spPerLevel: 3, beginnerSpPerLevel: 1, advancementSp: 1, firstJobLevel: 10, secondJobLevel: 30 },
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
  effects: [],
  buff: false,
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
  knockback: null,
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
const MOCK_SLOTS = ["Weapon", "Hat", "Top", "Bottom", "Overall", "Shoes", "Glove", "Cape", "Shield", "Earring"];
const mockSlotOf = (piece: GearPiece) => {
  const known = mockItems.find((i) => i.id === piece.item);
  if (known) return known.slot;
  const type = (piece.itemType ?? "").toLowerCase();
  return MOCK_SLOTS.slice(1).find((slot) => type.includes(slot.toLowerCase())) ?? "Weapon";
};
const mockClashes = (a: string, b: string) =>
  a === b || (a === "Overall" && (b === "Top" || b === "Bottom")) || (b === "Overall" && (a === "Top" || a === "Bottom"));
const showItemHandlers: ((link: ItemLink) => void)[] = [];
const showQuestHandlers: ((id: number) => void)[] = [];
const showPanelHandlers: ((panel: Panel) => void)[] = [];
const desk: DeskStatus = { open: false, onTop: false };
// Each window of the dev preview is a browser tab with its own mock, so trips, Route pin
// messages and the destination go between tabs on a channel, as Tauri events go between windows.
const tripHandlers: ((trip: Trip) => void)[] = [];
const tripMessageHandlers: ((message: TripMessage) => void)[] = [];
type Relayed = { kind: "trip"; trip: Trip } | { kind: "message"; message: TripMessage } | { kind: "nav"; nav: NavState };
const relay = typeof BroadcastChannel === "undefined" ? null : new BroadcastChannel("leafline-dev");
function relayed(event: MessageEvent<Relayed>) {
  const data = event.data;
  if (data.kind === "trip") tripHandlers.forEach((h) => h(data.trip));
  else if (data.kind === "message") {
    // A tab opened later starts with no destination, where the app keeps one for every window.
    if (data.message === "ask" && nav.destination) relay?.postMessage({ kind: "nav", nav });
    tripMessageHandlers.forEach((h) => h(data.message));
  }
  else {
    nav = data.nav;
    navHandlers.forEach((h) => h(nav));
  }
}
relay?.addEventListener("message", relayed);
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
  const owned = (build.start.pieces ?? []).map((p) => p.item);
  const gear = build.plan.gear.filter((id) => !owned.includes(id)).flatMap((id) => {
    const item = mockItems.find((g) => g.id === id);
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

const delay = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

/** "Go here" in this browser tab, as src-tauri/src/app/nav.rs keeps it. */
let nav: NavState = { destination: null, recent: [] };
const navHandlers: ((nav: NavState) => void)[] = [];
const samePlace = (a: Destination, b: Destination) =>
  a.kind === b.kind &&
  a.name.toLowerCase() === b.name.toLowerCase() &&
  a.maps.map((m) => m.id).sort().join() === b.maps.map((m) => m.id).sort().join();
function changeNav(next: NavState): NavState {
  nav = next;
  navHandlers.forEach((h) => h(nav));
  relay?.postMessage({ kind: "nav", nav });
  return nav;
}

/**
 * Dev-only controls for walking the sample world from the browser console, to try "Go here":
 * `__leafline.walk("Mossbank")` reads that map's header next, `walk(null)` hides the minimap,
 * `walk("Mosbank?")` reads a header too rough to trust, `finish()` fills the first quest's counters.
 */
let liveAnalysis: Analysis = sampleAnalysis;
function emitAnalysis(change: Partial<Analysis>) {
  liveAnalysis = { ...liveAnalysis, ...change };
  analysisHandlers.forEach((h) => h(liveAnalysis));
}
if (typeof window !== "undefined") {
  (window as unknown as { __leafline: object }).__leafline = {
    walk(name: string | null) {
      const map = name == null ? null : { street: "Fern Road", name, collapsed: false, rect: { x: 0, y: 0, w: 100, h: 20 }, twin: null };
      emitAnalysis({ map });
    },
    finish() {
      const quest = liveAnalysis.quest;
      if (!quest) return;
      const [first, ...rest] = quest.quests;
      const done = { ...first, objectives: first.objectives.map((o) => ({ ...o, current: o.required })) };
      emitAnalysis({ quest: { ...quest, quests: [done, ...rest] } });
    },
  };
}

export const mockBackend: Backend = {
  pickFolder: async () => "/test-screenshots",
  overlayStatus: async () => status,
  async setClickThrough(enabled) {
    status.clickThrough = enabled;
  },
  async hideOverlay() {},
  async toggleOverlay() {},
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
    { action: "ToggleOverlay", keys: "Super+Numpad0", suggestedKeys: "Super+Numpad0", label: "Show or hide the overlay", inSetup: true, taken: false },
    { action: "ToggleClickThrough", keys: "Super+NumpadDecimal", suggestedKeys: "Super+NumpadDecimal", label: "Lock the overlay (click-through)", inSetup: true, taken: false },
    { action: "TogglePins", keys: "", suggestedKeys: "", label: "Show or hide pinned panels", inSetup: false, taken: false },
    { action: "ToggleCompact", keys: "", suggestedKeys: "", label: "Compact HUD", inSetup: true, taken: false },
    { action: "ToggleLive", keys: "", suggestedKeys: "", label: "Start or stop live reading", inSetup: false, taken: false },
    { action: "ReadItem", keys: "F10", suggestedKeys: "Super+Numpad1", label: "Read the item you're hovering", inSetup: true, taken: true },
    { action: "ReadCharacter", keys: "", suggestedKeys: "", label: "Read your stat and skill windows, to update your build", inSetup: true, taken: false },
    { action: "PauseTracking", keys: "", suggestedKeys: "", label: "Pause or resume tracking", inSetup: false, taken: false },
    { action: "NewSession", keys: "", suggestedKeys: "", label: "Start a new session", inSetup: false, taken: false },
    { action: "SaveFrame", keys: "", suggestedKeys: "", label: "Save the picture last read", inSetup: true, taken: false },
    ].map((s) => {
      const keys = displaySettings.hotkeys.keys[s.action] ?? s.keys;
      return { ...s, keys, taken: keys === "F10" };
    }),
  liveStatus: async () => liveStatus,
  async setLiveHeld(held) {
    if (liveStatus.state === "reading") setLive({ ...liveStatus, held });
  },
  async resetSession() {},
  async startLive() {
    if (!displaySettings.live.enabled) throw "Turn on live reading in Settings first.";
    setLive({ state: "waiting", reason: "Starting…" });
    let frames = 0;
    clearInterval(liveTimer);
    liveTimer = setInterval(() => {
      frames += 1;
      setLive({ state: "reading", window: "MapleStory", width: 2560, height: 1440, frames, readMs: 412, hdr: false, held: false });
      analysisHandlers.forEach((h) => h(sampleAnalysis));
    }, displaySettings.live.interval * 1000);
  },
  async stopLive() {
    clearInterval(liveTimer);
    setLive({ state: "off" });
  },
  liveWindows: async () => mockWindows,
  saveItemPicture: async () => "C:\Users\you\AppData\Local\app.leafline.overlay\frames\leafline-item-2026-10-30_13-05-09.png",
  saveLiveFrame: async () => "C:\\Users\\you\\AppData\\Local\\app.leafline.overlay\\frames\\leafline-2026-10-30_13-05-09.png",
  async onLive(handler) {
    liveHandlers.push(handler);
    return () => {};
  },
  onFrameSaved: async () => () => {},
  onItemRead: async () => () => {},
  async onAnalysis(handler) {
    analysisHandlers.push(handler);
    if (!preview?.empty) setTimeout(() => handler(liveAnalysis), 200);
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
  async findMap(name) {
    if (!mapStatus.ready) return null;
    if (preview?.map) return preview.map;
    // An exact name finds its own map; anything else stands for the minimap's slightly misread
    // header, which matches the sample's current map.
    const exact = sampleSearch(name).find((m) => m.name === name);
    if (exact) return { score: 1, card: sampleCard(exact.id)!, alternatives: [] };
    // A header ending in "?" stands for a rough reading: a poor match with others nearly as close.
    if (name.endsWith("?")) {
      const near = sampleSearch(name.slice(0, 4));
      if (near.length) return { score: 0.7, card: sampleCard(near[0].id)!, alternatives: near.slice(1, 3) };
    }
    const card = sampleCard(2)!;
    return { score: 0.93, card, alternatives: [sampleSearch("Meadow II")[0]] };
  },
  // A preview server set up for it can stand in a real map's layout; otherwise there is none.
  async mapPicture(id) {
    try {
      const response = await fetch(`/__preview/map-picture/${id}`);
      return response.ok ? await response.json() : null;
    } catch {
      return null;
    }
  },
  mapCard: async (id) =>
    preview?.map ? (preview.map.card.map.id === id ? preview.map.card : null) : mapStatus.ready ? sampleCard(id) : null,
  searchMaps: async (text) => (mapStatus.ready ? sampleSearch(text) : []),
  mapRoute: async (from, to) =>
    preview?.routes ? (preview.routes.find((r) => r.from.id === from && r.to.id === to) ?? null) : sampleRoute(from, to),
  destination: async () => nav,
  async setDestination(destination) {
    if (!destination) return changeNav({ ...nav, destination: null });
    const recent = [destination, ...nav.recent.filter((d) => !samePlace(d, destination))].slice(0, 6);
    return changeNav({ destination, recent });
  },
  async arriveAt(destination) {
    return nav.destination && samePlace(nav.destination, destination) ? changeNav({ ...nav, destination: null }) : nav;
  },
  async forgetDestination(destination) {
    return changeNav({ ...nav, recent: nav.recent.filter((d) => !samePlace(d, destination)) });
  },
  async onDestination(handler) {
    navHandlers.push(handler);
    return () => {};
  },
  async monsterDrops(monsters) {
    await delay(900);
    return sampleDrops(monsters);
  },
  async questGuide(quests) {
    if (!mapStatus.ready) return null;
    if (preview?.guide) return preview.guide;
    return sampleGuide(quests, 2, dropsDownloaded);
  },
  dropsFetchedAt: async () => (dropsDownloaded ? Date.now() / 1000 - 86400 : null),
  async downloadDrops(_retry?: boolean) {
    await delay(900);
    dropsDownloaded = true;
  },
  async trainingSpots(_name, _street, _twin, measured, level, assumptions) {
    if (!mapStatus.ready) return null;
    return preview?.spots ?? sampleSpots(2, measured, level, assumptions);
  },
  async openLink(url) {
    window.open(url, "_blank", "noopener");
  },
  async onMapData(handler) {
    mapDataHandlers.push(handler);
    return () => {};
  },
  savedSpots: async () => [...savedSpots],
  potionSlots: async (character) => [...slotsFor(character)],
  async setPotionSlots(character, slots) {
    const taken = new Set(slots.map((s) => s.slot).filter((n) => n > 0));
    const kept = slots
      .map((s) => {
        if (s.slot > 0) return { ...s, name: s.name.trim() };
        let next = 1;
        while (taken.has(next)) next++;
        taken.add(next);
        return { ...s, slot: next, name: s.name.trim() };
      })
      .sort((a, b) => a.slot - b.slot);
    potionSlotSets.set((character ?? "").trim().toLowerCase(), kept);
    potionHandlers.forEach((h) => h());
    return [...kept];
  },
  sessionPotions: async (startedAt) => [...(sessionPotions.get(startedAt) ?? [])],
  async setSessionPotion(character, startedAt, slot, start, now) {
    const rows = (sessionPotions.get(startedAt) ?? []).filter((p) => p.slot !== slot);
    const def = slotsFor(character).find((p) => p.slot === slot);
    if ((start != null || now != null) && def) {
      const used = start != null && now != null ? Math.max(0, start - now) : 0;
      rows.push({ slot, name: def.name, price: def.price, start, now, used });
    }
    sessionPotions.set(startedAt, rows.sort((a, b) => a.slot - b.slot));
    potionHandlers.forEach((h) => h());
    historyHandlers.forEach((h) => h());
    return [...rows];
  },
  async onPotions(handler) {
    potionHandlers.push(handler);
    return () => {};
  },
  async setSpotSaved(mapId, name, saved) {
    savedSpots = savedSpots.filter((s) => s.mapId !== mapId);
    if (saved) savedSpots.unshift({ mapId, name, savedAt: Date.now() / 1000 });
    spotHandlers.forEach((h) => h([...savedSpots]));
    return [...savedSpots];
  },
  playerDamage: async () => null,
  async onPlayerDamage(handler) {
    damageHandlers.push(handler);
    return () => {};
  },
  characterStats: async () => mockStats(),
  async setBuff(name, skill, on) {
    mockBuffsOn = mockBuffsOn.filter((id) => id !== skill);
    if (on) mockBuffsOn.push(skill);
    const damage: PlayerDamage = { attackMin: 107, attackMax: 289, skills: [], sheet: null, readAt: null, buffsOn: mockBuffsOn, readWith: [] };
    damageHandlers.forEach((h) => h({ name, damage }));
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
  sessionHistory: async () => mockHistory(),
  itemCrafting: async () => ({ madeBy: [], usedIn: [] }),
  cardIsLive: async () => true,
  pinCard: async () => {},
  async showCard(path) {
    window.open(`/${path}`, "leafline-card", "width=380,height=540");
  },
  async popOutCard(path) {
    window.open(`/${path}`, "_blank", "width=380,height=540");
  },
  async craftingRecipes(query) {
    // A few made-up recipes, enough to lay the Crafting tab out.
    const item = (id: number, name: string, count = 1) => ({ id, name, count });
    const all = [
      { id: 1, discipline: "Smelting", skillLevel: 1, craftExp: 3, mesoCost: 100, output: item(900001, "Fern Ingot"), ingredients: [item(900101, "Fern Ore", 5)] },
      { id: 2, discipline: "Smelting", skillLevel: 2, craftExp: 5, mesoCost: 250, output: item(900002, "Ash Ingot"), ingredients: [item(900102, "Ash Ore", 5), item(900001, "Fern Ingot", 1)] },
      { id: 3, discipline: "Stitching", skillLevel: 1, craftExp: 3, mesoCost: 0, output: item(900003, "Moss Cloth", 2), ingredients: [item(900103, "Moss Thread", 4)] },
      { id: 4, discipline: "Stitching", skillLevel: 4, craftExp: 12, mesoCost: 1200, output: item(900004, "Bark Vest"), ingredients: [item(900003, "Moss Cloth", 6), item(900104, "Thorn Button", 2)] },
    ];
    const recipes = all.filter(
      (r) =>
        (!query.discipline || r.discipline === query.discipline) &&
        (query.maxLevel == null || r.skillLevel <= query.maxLevel) &&
        (!query.search || [r.output, ...r.ingredients].some((i) => i.name.toLowerCase().includes(query.search!.toLowerCase()))),
    );
    const disciplines: [string, number][] = [["Smelting", 2], ["Stitching", 2]];
    return { disciplines, recipes };
  },
  levelExp: async () => [
    { level: 20, needed: 6_000 },
    { level: 21, needed: 6_900 },
  ],
  sessionTimeline: async (id) => sampleTimelines.get(id) ?? { points: [], events: [] },
  setTyping: async () => {},
  mapNote: async (mapId) => mapNotes.get(mapId) ?? null,
  async setMapNote(mapId, map, note) {
    const text = note.trim();
    if (!text) mapNotes.delete(mapId);
    else mapNotes.set(mapId, { mapId, map, note: text, updatedAt: Date.now() / 1000 });
    mapNoteHandlers.forEach((h) => h(mapId));
    return mapNotes.get(mapId) ?? null;
  },
  async onMapNote(handler) {
    mapNoteHandlers.push(handler);
    return () => {};
  },
  dropLog: async () => [...drops].sort((a, b) => b.seenAt - a.seenAt || b.id - a.id),
  async logDrop(drop) {
    const id = Math.max(0, ...drops.map((d) => d.id)) + 1;
    drops.push({ ...drop, id, seenAt: Date.now() / 1000 });
    dropHandlers.forEach((h) => h());
    return id;
  },
  async deleteDrop(id) {
    drops = drops.filter((d) => d.id !== id);
    dropHandlers.forEach((h) => h());
  },
  async onDrops(handler) {
    dropHandlers.push(handler);
    return () => {};
  },
  async saveImage(png, name) {
    const url = URL.createObjectURL(new Blob([png as BlobPart], { type: "image/png" }));
    const link = Object.assign(document.createElement("a"), { href: url, download: name });
    link.click();
    URL.revokeObjectURL(url);
    return true;
  },
  async showTraining() {
    window.open("/desk?tab=log", "_blank");
  },
  async showDesk() {
    desk.open = true;
    window.open("/desk", "_blank");
    return { ...desk };
  },
  deskStatus: async () => ({ ...desk }),
  // The dev preview opens tabs in the browser, by address.
  deskTakeTab: async () => new URLSearchParams(location.search).get("tab"),
  async onDeskTab() {
    return () => {};
  },
  async setDeskOnTop(onTop) {
    desk.onTop = onTop;
    return { ...desk };
  },
  monitorCount: async () => 2,
  async setPageZoom(zoom) {
    document.documentElement.style.zoom = String(zoom);
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
    return what === "drops" ? drops.length : savedSessions.length;
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
            source: null,
          },
          {
            id: "fernblade-guide",
            job: 420,
            name: "Dagger Fernblade, example guide",
            summary: "A made-up plan that follows a made-up guide.",
            plan: {
              job: 420,
              skills: [{ skillId: 1, level: 3 }, { skillId: 10, level: 20 }],
              stats: [{ stat: "luk", target: null, fromLevel: null }],
              gear: [500],
              planTo: 70,
            },
            missing: [],
            source: { name: "NiaDB", url: "https://meowdb.com/msclassic/guides/example", credit: "Example guide by Someone, 2026-01-01." },
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
  itemFilters: async () => ({
    slots: ["Weapon", "Hat", "Top"],
    weapons: ["Claw", "Dagger"],
    scrollSlots: ["Weapon", "Hat", "Glove", "Pet equip"],
    scrollStats: { Weapon: ["Attack", "Magic Attack"], Hat: ["HP", "MP", "Accuracy"], Glove: ["Attack", "Accuracy", "Crit. Rate"], "Pet equip": ["Speed", "Jump"] },
    scrollChances: ["100%", "60%", "10%", "Chaos"],
    useGroups: ["Potions", "Buffs", "Food", "Return scrolls", "Bow arrows", "Crossbow arrows", "Throwing stars", "Crafting", "Other"],
    setupGroups: ["Chairs", "Decor"],
    etcGroups: ["Monster loot", "Ores & gems", "Quest items", "Party quest", "Minigames"],
  }),
  itemRecord: async (id) => mockItems.find((item) => item.id === id) ?? null,
  findItem: async (name) => mockItems.find((item) => item.name.toLowerCase() === name.toLowerCase()) ?? null,
  async itemOrigin(id) {
    const item = mockItems.find((i) => i.id === id);
    if (!item) return null;
    const guide = sampleGuide([{ title: "Moss Patrol", objectives: [item.name] }], null, true);
    const quest = guide.quests[0];
    return { from: guide.from, origin: { drops: quest.objectives[0], quests: quest.quest ? [quest.quest] : [], shops: [] } };
  },
  async showItem(id, text) {
    showItemHandlers.forEach((h) => h({ id, text }));
  },
  async onShowItem(handler) {
    showItemHandlers.push(handler);
    return () => {};
  },
  async addPiece(piece) {
    const build = newestBuild();
    if (!build) throw new Error("Plan a build first, in the Character tab.");
    const pieces = (build.start.pieces ??= []);
    const same = (p: GearPiece) => JSON.stringify({ ...p, id: 0 }) === JSON.stringify({ ...piece, id: 0 });
    if (pieces.some(same)) return;
    const slot = mockSlotOf(piece);
    build.start.pieces = pieces.filter((p) => !mockClashes(mockSlotOf(p), slot));
    build.start.pieces.push({ ...piece, id: Math.max(0, ...pieces.map((p) => p.id)) + 1 });
    buildsChanged();
  },
  async wearItem(id) {
    const item = mockItems.find((i) => i.id === id);
    if (!item) throw new Error("Leafline doesn't know that item.");
    const label = (l: string) => (l === "Attack" ? "Weapon Attack" : l === "Magic" ? "Magic Attack" : l);
    await mockBackend.addPiece(
      {
        id: 0,
        item: item.id,
        name: item.name,
        itemType: item.weaponType ?? item.slot,
        level: item.reqLevel,
        stats: item.stats.map((s) => ({ label: label(s.label), value: s.value })),
        upgrades: null,
        slotsLeft: item.upgradeSlots,
        slots: item.upgradeSlots,
      },
      null,
    );
  },
  async updatePiece(piece) {
    const build = newestBuild();
    if (!build) return;
    build.start.pieces = (build.start.pieces ?? []).map((p) => (p.id === piece.id ? piece : p));
    buildsChanged();
  },
  async removePiece(id) {
    const build = newestBuild();
    if (!build) return;
    const item = build.start.pieces?.find((p) => p.id === id)?.item;
    build.start.pieces = (build.start.pieces ?? []).filter((p) => p.id !== id);
    if (item != null && !build.start.pieces.some((p) => p.item === item)) build.plan.gear = build.plan.gear.filter((g) => g !== item);
    buildsChanged();
  },
  async setGearGoal(item, wanted) {
    const build = newestBuild();
    if (!build) return;
    build.plan.gear = build.plan.gear.filter((id) => id !== item).concat(wanted ? [item] : []);
    buildsChanged();
  },
  async fillGear(id, level) {
    const build = mockBuilds.find((b) => b.id === id);
    if (!build) throw new Error("That build is gone.");
    const planned = new Set(build.plan.gear.map((g) => mockItems.find((i) => i.id === g)?.slot));
    const add = mockItems.filter((i) => i.kind === "equip" && !planned.has(i.slot) && i.reqLevel >= level - 10 && !build.plan.gear.includes(i.id));
    build.plan.gear.push(...add.map((i) => i.id));
    buildsChanged();
    return add.length;
  },
  async buildOutfit(id, level) {
    const build = mockBuilds.find((b) => b.id === id);
    if (!build) return null;
    const { gear } = mockExpand(build);
    const pieces = build.start.pieces ?? [];
    const score = (stats: { label: string; value: number }[]) => {
      const of = (label: string) => stats.filter((s) => s.label === label).reduce((sum, s) => sum + s.value, 0);
      return [(of("Attack") + of("Weapon Attack")) * 3 + of("LUK") * 2 + of("DEX"), of("Weapon Def.")];
    };
    const beats = (a: number[], b: number[]) => a[0] > b[0] || (a[0] === b[0] && a[1] > b[1]);
    const overall = pieces.some((p) => mockSlotOf(p) === "Overall");
    const outfit: Outfit = { level, slots: [], paths: [], wornGender: "male" };
    for (const slot of MOCK_SLOTS) {
      if (overall && (slot === "Top" || slot === "Bottom")) continue;
      const wearing = pieces.findLast((p) => mockSlotOf(p) === slot) ?? null;
      const wornLevel = wearing?.level ?? 0;
      const ahead = gear
        .filter((g) => g.item.slot === slot && g.item.id !== wearing?.item && g.item.reqLevel > wornLevel)
        .sort((a, b) => (a.level ?? 999) - (b.level ?? 999));
      const ready = ahead.filter((g) => g.level != null && g.level <= level);
      const waiting = ahead.filter((g) => g.level == null || g.level > level);
      const goalNow = ready.at(-1)?.item ?? null;
      const best = mockItems
        .filter((i) => i.kind === "equip" && i.slot === slot && i.reqLevel <= level)
        .sort((a, b) => (beats(score(a.stats), score(b.stats)) ? -1 : 1))[0];
      const found = best && best.id !== wearing?.item && (!wearing || beats(score(best.stats), score(wearing.stats))) ? best : null;
      const suggestedPlanned = goalNow != null && !(found && beats(score(found.stats), score(goalNow.stats)));
      const suggested = suggestedPlanned ? (mockItems.find((i) => i.id === goalNow!.id) ?? null) : found;
      let next = waiting[0] ?? null;
      const planned = next != null;
      const later = planned ? waiting.length - 1 : 0;
      if (!next) {
        // The first better piece a later level can wear.
        const floors = [wearing && score(wearing.stats), suggested && score(suggested.stats)].filter((f): f is number[] => !!f);
        const coming = mockItems
          .filter((i) => i.kind === "equip" && i.slot === slot && i.reqLevel > level && floors.every((f) => beats(score(i.stats), f)))
          .sort((a, b) => a.reqLevel - b.reqLevel)[0];
        if (coming) next = { item: coming, level: coming.reqLevel, usable: true };
      }
      const optional = slot === "Overall" || slot === "Shield";
      if (optional && !wearing && !planned && !suggestedPlanned) continue;
      const item = next && (mockItems.find((i) => i.id === next.item.id) ?? next.item);
      const fit = (reqLevel: number) => ({ ok: reqLevel <= level, short: reqLevel <= level ? [] : [`Lv ${reqLevel} (you're ${level})`] });
      outfit.slots.push({
        slot,
        wearing,
        suggested,
        suggestedPlanned,
        next: next && item && { ...next, item },
        planned,
        later,
        wearingFit: wearing ? fit(wearing.level ?? 0) : null,
        suggestedFit: suggested ? fit(suggested.reqLevel) : null,
        nextFit: next ? fit(next.level ?? next.item.reqLevel) : null,
      });
      // A tier at a time from here, each better than the last, as the app works it out.
      const steps: Outfit["paths"][number]["steps"] = [];
      let floor = suggested ? score(suggested.stats) : wearing ? score(wearing.stats) : null;
      let tier = suggested?.reqLevel ?? level;
      for (const i of mockItems
        .filter((i) => i.kind === "equip" && i.slot === slot && i.reqLevel > level)
        .sort((a, b) => a.reqLevel - b.reqLevel)) {
        if (i.reqLevel <= tier || (floor && !beats(score(i.stats), floor))) continue;
        steps.push({ item: i, level: i.reqLevel, planned: gear.some((g) => g.item.id === i.id) });
        floor = score(i.stats);
        tier = i.reqLevel;
      }
      outfit.paths.push({ slot, now: suggested ? { item: suggested, level, planned: suggestedPlanned } : null, steps, passed: [] });
    }
    return outfit;
  },
  async searchQuests(search) {
    const quests = mockQuests.filter(
      (q) =>
        q.name.toLowerCase().includes(search.text.toLowerCase()) &&
        (!search.region || q.region === search.region) &&
        (search.atLevel == null || (q.levelMin ?? 0) <= search.atLevel),
    );
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
        chainRest: {
          quests: [
            { id: 72, name: "Slime Stew", levelMin: 4 },
            { id: 73, name: "The Recipe Book", levelMin: 6 },
          ],
          exp: 1850,
          mesos: 2400,
          levelMax: 6,
        },
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
  findNpc: async (name) => sampleNpcs({ text: name, kind: null, region: null, area: null, sort: null, offset: 0 }).npcs[0]?.id ?? null,
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
    [...mockBuilds].sort((a, b) => b.usedAt - a.usedAt).map(({ id, name, character }) => ({ id, name, character, updatedAt: Date.now() / 1000 })),
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
    window.open("/desk?tab=planner", "_blank");
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
  async showPanel(panel) {
    const tab = panel === "route" ? "map" : panel;
    showPanelHandlers.forEach((h) => h(tab));
  },
  async sendTrip(trip) {
    relay?.postMessage({ kind: "trip", trip });
  },
  async onTrip(handler) {
    tripHandlers.push(handler);
    return () => {};
  },
  async tripMessage(message) {
    relay?.postMessage({ kind: "message", message });
  },
  async onTripMessage(handler) {
    tripMessageHandlers.push(handler);
    return () => {};
  },
  async onShowPanel(handler) {
    showPanelHandlers.push(handler);
    return () => {};
  },
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

// Made-up stats for the dev preview's Stats view.
const damageHandlers: ((change: { name: string; damage: PlayerDamage }) => void)[] = [];
let mockBuffsOn: number[] = [];

function mockStats(): StatsView {
  const sheet: StatSheet = {
    level: 33,
    str: 28,
    dex: 149,
    int: 4,
    luk: 4,
    gearStr: 2,
    gearDex: 6,
    gearInt: 0,
    gearLuk: 0,
    abilityPoints: 0,
    maxHp: 1220,
    maxMp: 1004,
    fame: 3,
    attackMin: 107,
    attackMax: 289,
    weaponDef: 117,
    magic: 2,
    magicDef: 28,
    accuracy: 76,
    evasion: 33,
    critRate: 26,
    critDamage: 141,
    speed: 101,
    jump: 100,
  };
  const none: SkillEffects = { attack: 0, magic: 0, weaponDef: 0, weaponDefPercent: 0, magicDef: 0, accuracy: 0, evasion: 0, speed: 0, jump: 0, seconds: 300 };
  const row = (id: number, name: string, level: number | null, maxLevel: number, effects: Partial<SkillEffects>, description: string | null): BuffRow => ({
    id,
    name,
    level,
    maxLevel,
    levelRead: level != null,
    on: level != null && mockBuffsOn.includes(id),
    onAtRead: false,
    effects: { ...none, ...effects },
    description,
  });
  const buffs = [
    row(12, "Soft Landing", 10, 10, { accuracy: 15, evasion: 15 }, "MP -12; Accuracy +15, Evasion +15 for 300 sec"),
    row(21, "Bramble Guard", 4, 20, {}, "HP -20, MP -20; Attack speed up 2 stages for 120 sec"),
    row(30, "Thin Mist", null, 30, {}, null),
  ];
  const on = buffs.filter((b) => b.on);
  const add = (key: keyof SkillEffects) => on.reduce((n, b) => n + b.effects[key], 0);
  const buffed = on.length
    ? { ...sheet, accuracy: sheet.accuracy! + add("accuracy"), evasion: sheet.evasion! + add("evasion") }
    : null;
  return { sheet, readAt: Date.now() / 1000 - 260, buffs, buffed, weaponAttack: 49, attackUnscaled: false };
}
