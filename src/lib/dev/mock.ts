// Stand-in backend for designing the UI in a plain browser (`npm run dev`). Images come from
// the git-ignored test-screenshots folder; every analysis returns a saved real result.

import type { Backend } from "../api";
import type {
  Analysis,
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
  ReplayProgress,
  SessionSummary,
} from "../types";
import sample from "./sample-analysis.json";
import { sampleCard, sampleDrops, sampleGuide, sampleRoute, sampleSearch, sampleSpots } from "./sample-map";
import sampleSession from "./sample-session.json";

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
const sampleSessionSummary = preview?.session ?? (sampleSession as SessionSummary);

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

/** Saved sessions; one made-up session on the sample world's maps to start with. */
let savedSessions: SavedSession[] = [
  {
    id: 1,
    label: "",
    savedAt: now - 86400 * 2,
    startedAt: now - 86400 * 2 - 5400,
    level: 29,
    levelUps: 1,
    activeSeconds: 4200,
    elapsedSeconds: 5400,
    expGained: 25_400,
    expPerHour: 21_771,
    mesosGained: 11_200,
    mesosPerHour: 9_600,
    percentPerHour: 22.4,
    job: "Bandit",
    deaths: 1,
    note: "",
    lines: [
      {
        map: "Fern Road : Mossbank Meadow II",
        town: false,
        level: 29,
        party: 1,
        job: "Bandit",
        activeSeconds: 3000,
        expGained: 19_600,
        percentGained: 18.4,
        mesosGained: 8_100,
        mesosSeconds: 3000,
      },
    ],
    maps: [
      { map: "Fern Road : Mossbank Meadow II", activeSeconds: 3000, expGained: 19_600, mesosGained: 8_100 },
      { map: "Fern Road : Burrow Den", activeSeconds: 1200, expGained: 5_800, mesosGained: 3_100 },
    ],
  },
];
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
  live: { enabled: false, interval: 2, pauseAfter: 15, window: null },
};

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
  shortcuts: async () => [
    { action: "ToggleOverlay", keys: "Ctrl+Alt+O", defaultKeys: "Ctrl+Alt+O", label: "Show or hide the overlay" },
    { action: "ToggleClickThrough", keys: "Ctrl+Alt+L", defaultKeys: "Ctrl+Alt+L", label: "Lock the overlay (click-through)" },
    { action: "TogglePins", keys: "Ctrl+Alt+P", defaultKeys: "Ctrl+Alt+P", label: "Show or hide pinned panels" },
    { action: "ToggleCompact", keys: "Ctrl+Alt+H", defaultKeys: "Ctrl+Alt+H", label: "Compact HUD" },
    { action: "ToggleLive", keys: "Ctrl+Alt+R", defaultKeys: "Ctrl+Alt+R", label: "Start or stop live reading" },
    { action: "ReadItem", keys: "Ctrl+Alt+I", defaultKeys: "Ctrl+Alt+I", label: "Read the item you're hovering, for a price check" },
    { action: "PauseTracking", keys: "Ctrl+Alt+K", defaultKeys: "Ctrl+Alt+K", label: "Pause or resume tracking" },
    { action: "NewSession", keys: "Ctrl+Alt+N", defaultKeys: "Ctrl+Alt+N", label: "Start a new session" },
    { action: "ToggleLogging", keys: "Ctrl+Alt+J", defaultKeys: "Ctrl+Alt+J", label: "Log this run or not" },
    { action: "SaveFrame", keys: "Ctrl+Alt+F", defaultKeys: "Ctrl+Alt+F", label: "Save the picture last read, to Pictures\\Leafline" },
    { action: "ShowWorkbench", keys: "Ctrl+Alt+W", defaultKeys: "Ctrl+Alt+W", label: "Open the workbench" },
  ],
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
  saveLiveFrame: async () => "C:\\Users\\you\\Pictures\\Leafline\\leafline-2026-10-30_13-05-09.png",
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
    const files = ["maps.json", "monsters.json", "npcs.json", "quests.json"];
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
  async trainingSpots() {
    if (!mapStatus.ready) return null;
    return preview?.spots ?? sampleSpots(2);
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
      deaths: s.deaths,
      note: "",
      lines: s.lines,
      maps: s.maps
        .filter((m) => m.map && m.activeSeconds > 0)
        .map((m) => ({ map: m.map!, activeSeconds: m.activeSeconds, expGained: m.expGained, mesosGained: m.mesosGained })),
    });
    historyHandlers.forEach((h) => h());
    return id;
  },
  sessionHistory: async () => mockHistory(),
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
    displaySettings = { ...settings };
    settingsHandlers.forEach((h) => h(displaySettings));
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
  version: async () => "dev",
  async fitWindow() {},
};
