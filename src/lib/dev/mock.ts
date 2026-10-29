// Stand-in backend for designing the UI in a plain browser (`npm run dev`). Images come from
// the git-ignored test-screenshots folder; every analysis returns a saved real result.

import type { Backend } from "../api";
import type {
  Analysis,
  MapDataStatus,
  MapMatch,
  OverlayStatus,
  PinStatus,
  ReplayProgress,
  SessionSummary,
} from "../types";
import sample from "./sample-analysis.json";
import { sampleCard, sampleDrops, sampleRoute, sampleSearch } from "./sample-map";
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
  pins?: PinStatus;
}
const preview = (globalThis as { __LEAFLINE_PREVIEW__?: Preview }).__LEAFLINE_PREVIEW__;
const sampleAnalysis = preview?.analysis ?? (sample as unknown as Analysis);
const sampleSessionSummary = preview?.session ?? (sampleSession as SessionSummary);

const status: OverlayStatus = { visible: true, clickThrough: false };
const analysisHandlers: ((analysis: Analysis) => void)[] = [];
const progressHandlers: ((progress: ReplayProgress) => void)[] = [];
const sessionHandlers: ((session: SessionSummary) => void)[] = [];
let replayCancelled = false;

// Starts without map data, so the download prompt can be designed too.
let mapStatus: MapDataStatus = preview?.map
  ? { ready: true, clientBuild: "preview", dataUpdatedAt: null, checkedAt: null, files: [] }
  : { ready: false, clientBuild: null, dataUpdatedAt: null, checkedAt: null, files: [] };
const mapDataHandlers: ((status: MapDataStatus) => void)[] = [];

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
  async calibrateHud() {
    await delay(250);
    return { learned: ["HP", "MP", "EXP", "EXP %", "Level", "Name"], failed: [] };
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
    { action: "toggleOverlay", keys: "Ctrl+Alt+O", label: "Show or hide the overlay" },
    { action: "toggleClickThrough", keys: "Ctrl+Alt+L", label: "Lock the overlay (click-through)" },
    { action: "showWorkbench", keys: "Ctrl+Alt+W", label: "Open the workbench" },
  ],
  takeLaunchImage: async () => null,
  onOpenImage: async () => () => {},
  async onAnalysis(handler) {
    analysisHandlers.push(handler);
    setTimeout(() => handler(sampleAnalysis), 200);
    return () => {};
  },
  async onReplayProgress(handler) {
    progressHandlers.push(handler);
    return () => {};
  },
  async onSession(handler) {
    sessionHandlers.push(handler);
    setTimeout(() => handler(sampleSessionSummary), 200);
    return () => {};
  },
  onOverlayStatus: async () => () => {},
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
  mapRoute: async (from, to) => sampleRoute(from, to),
  async monsterDrops(monsters) {
    await delay(900);
    return sampleDrops(monsters);
  },
  async openLink(url) {
    window.open(url, "_blank", "noopener");
  },
  async onMapData(handler) {
    mapDataHandlers.push(handler);
    return () => {};
  },
  currentReadings: async () => ({
    analysis: sampleAnalysis,
    session: sampleSessionSummary,
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
  async fitWindow() {},
};
