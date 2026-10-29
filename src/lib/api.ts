// Typed access to the Rust commands and events. In a plain browser (UI development with
// `npm run dev`) a mock backend stands in, so the pages can be designed without Tauri.

import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";

import type { Analysis, Calibration, HudValues, OverlayStatus, ShortcutInfo } from "./types";

export interface Backend {
  analyzeImage(path: string): Promise<Analysis>;
  /** Teaches the HUD font from a screenshot whose values the user typed in. */
  calibrateHud(path: string, values: HudValues): Promise<Calibration>;
  listImages(folder: string): Promise<string[]>;
  /** An object URL for displaying the image; revoke it when done. */
  imageUrl(path: string): Promise<string>;
  pickImage(): Promise<string | null>;
  pickFolder(): Promise<string | null>;
  overlayStatus(): Promise<OverlayStatus>;
  setClickThrough(enabled: boolean): Promise<void>;
  hideOverlay(): Promise<void>;
  toggleOverlay(): Promise<void>;
  showWorkbench(): Promise<void>;
  shortcuts(): Promise<ShortcutInfo[]>;
  /** An image passed on the command line at startup, once. */
  takeLaunchImage(): Promise<string | null>;
  /** Images passed to later launches while the app is already running. */
  onOpenImage(handler: (path: string) => void): Promise<UnlistenFn>;
  onAnalysis(handler: (analysis: Analysis) => void): Promise<UnlistenFn>;
  onOverlayStatus(handler: (status: OverlayStatus) => void): Promise<UnlistenFn>;
}

const IMAGE_FILTER = { name: "Screenshots", extensions: ["png", "jpg", "jpeg", "webp"] };

const tauriBackend: Backend = {
  analyzeImage: (path) => invoke("analyze_image", { path }),
  calibrateHud: (path, values) => invoke("calibrate_hud", { path, values }),
  listImages: (folder) => invoke("list_images", { folder }),
  async imageUrl(path) {
    const bytes = await invoke<ArrayBuffer>("read_image", { path });
    return URL.createObjectURL(new Blob([new Uint8Array(bytes)], { type: imageType(path) }));
  },
  async pickImage() {
    const picked = await open({ multiple: false, directory: false, filters: [IMAGE_FILTER] });
    return typeof picked === "string" ? picked : null;
  },
  async pickFolder() {
    const picked = await open({ multiple: false, directory: true });
    return typeof picked === "string" ? picked : null;
  },
  overlayStatus: () => invoke("overlay_status"),
  setClickThrough: (enabled) => invoke("set_click_through", { enabled }),
  hideOverlay: () => invoke("hide_overlay"),
  toggleOverlay: () => invoke("toggle_overlay"),
  showWorkbench: () => invoke("show_workbench"),
  shortcuts: () => invoke("shortcut_list"),
  takeLaunchImage: () => invoke("take_launch_image"),
  onOpenImage: (handler) => listen<string>("open-image", (e) => handler(e.payload)),
  onAnalysis: (handler) => listen<Analysis>("analysis", (e) => handler(e.payload)),
  onOverlayStatus: (handler) => listen<OverlayStatus>("overlay-status", (e) => handler(e.payload)),
};

/** The mock is dev-only: production builds must never bundle it or the screenshots it loads. */
async function devBackend(): Promise<Backend | null> {
  if (!import.meta.env.DEV || isTauri()) return null;
  return (await import("./dev/mock")).mockBackend;
}

export const backend: Backend = (await devBackend()) ?? tauriBackend;

function imageType(path: string): string {
  const extension = path.split(".").pop()?.toLowerCase();
  return extension === "jpg" ? "image/jpeg" : `image/${extension}`;
}

/** File name without its folder, for display. */
export function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}
