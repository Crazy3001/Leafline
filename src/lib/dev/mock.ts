// Stand-in backend for designing the UI in a plain browser (`npm run dev`). Images come from
// the git-ignored test-screenshots folder; every analysis returns a saved real result.

import type { Backend } from "../api";
import type { Analysis, OverlayStatus, ReplayProgress, SessionSummary } from "../types";
import sample from "./sample-analysis.json";
import sampleSession from "./sample-session.json";

const screenshots = import.meta.glob("/test-screenshots/*.{png,jpg,jpeg,webp}", {
  query: "?url",
  import: "default",
  eager: true,
}) as Record<string, string>;

const status: OverlayStatus = { visible: true, clickThrough: false };
const analysisHandlers: ((analysis: Analysis) => void)[] = [];
const progressHandlers: ((progress: ReplayProgress) => void)[] = [];
const sessionHandlers: ((session: SessionSummary) => void)[] = [];
let replayCancelled = false;

/** The saved real session, as if `fraction` of it had been replayed so far. */
function partialSession(fraction: number, frames: number): SessionSummary {
  const full = sampleSession as SessionSummary;
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
    const analysis = sample as unknown as Analysis;
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
      const analysis = sample as unknown as Analysis;
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
    setTimeout(() => handler(sample as unknown as Analysis), 200);
    return () => {};
  },
  async onReplayProgress(handler) {
    progressHandlers.push(handler);
    return () => {};
  },
  async onSession(handler) {
    sessionHandlers.push(handler);
    setTimeout(() => handler(sampleSession as SessionSummary), 200);
    return () => {};
  },
  onOverlayStatus: async () => () => {},
};
