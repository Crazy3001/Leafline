// Stand-in backend for designing the UI in a plain browser (`npm run dev`). Images come from
// the git-ignored test-screenshots folder; every analysis returns a saved real result.

import type { Backend } from "../api";
import type { Analysis, OverlayStatus } from "../types";
import sample from "./sample-analysis.json";

const screenshots = import.meta.glob("/test-screenshots/*.{png,jpg,jpeg,webp}", {
  query: "?url",
  import: "default",
  eager: true,
}) as Record<string, string>;

const status: OverlayStatus = { visible: true, clickThrough: false };
const analysisHandlers: ((analysis: Analysis) => void)[] = [];
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
  onOverlayStatus: async () => () => {},
};
