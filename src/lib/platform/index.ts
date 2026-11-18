// Which Leafline this is: the Windows program (Tauri), the website (the same Rust in a worker), or
// the dev mock for UI work in a plain browser. Pages ask `can` before showing a control only one
// of them has.

import { isTauri } from "@tauri-apps/api/core";

import type { Backend } from "../api";

/** Built for the website: `npm run build:web` or `npm run dev:web` (Vite mode "web"). */
export const isWeb: boolean = __LEAFLINE_WEB__;

/** What this copy of Leafline can do. The website has no windows of its own, hotkeys or screen reading. */
export const can = {
  liveReading: !isWeb,
  hotkeys: !isWeb,
  pins: !isWeb,
  clickThrough: !isWeb,
  onTop: !isWeb,
  updates: !isWeb,
  dataFolder: !isWeb,
  /**
   * A small window of Now's numbers that stays over the game: a pinned panel in the program, the
   * browser's picture-in-picture window on the website (Chrome and Edge; see platform/pip.ts).
   */
  popOut: !isWeb || (typeof window !== "undefined" && "documentPictureInPicture" in window),
};

export async function pickBackend(): Promise<Backend> {
  // The literal constant, not `isWeb`, so the program's build leaves the website's code out.
  if (__LEAFLINE_WEB__) return (await import("./web")).webBackend;
  // The mock is dev-only: production builds must never bundle it or the screenshots it loads.
  if (import.meta.env.DEV && !isTauri()) return (await import("../dev/mock")).mockBackend;
  return (await import("./tauri")).tauriBackend;
}
