// Which Leafline this is: the Windows program (Tauri), the website (the same Rust in a worker), or
// the dev mock for UI work in a plain browser. Pages ask `can` before showing a control only one
// of them has.

import { isTauri } from "@tauri-apps/api/core";

import type { Backend } from "../api";
import { canRead } from "./capture";

/** Built for the website: `npm run build:web` or `npm run dev:web` (Vite mode "web"). */
export const isWeb: boolean = __LEAFLINE_WEB__;

/**
 * The dev mock dressed as the website, for UI work on its controls (the Share button): `?web` on a
 * dev-server address in a plain browser. Production builds leave it out.
 */
const mockWeb =
  import.meta.env.DEV && !__LEAFLINE_WEB__ && typeof location !== "undefined" && !isTauri() && new URLSearchParams(location.search).has("web");

/** What this copy of Leafline can do. The website has no windows of its own or hotkeys, and reads the
 * game only from a window the player shares with it. */
export const can = {
  /** The program's live reading: the game window found by itself, with its settings. */
  liveReading: !isWeb && !mockWeb,
  /**
   * The website's live reading: the player shares the game window with the page (Firefox, Chrome
   * and Edge on a computer). Level and EXP for now; see platform/live.ts.
   */
  readWindow: (isWeb || mockWeb) && canRead,
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
