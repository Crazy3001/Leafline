// Display settings shared by the overlay, pinned panels and the Settings window.

import { backend } from "./api";
import type { BuildRules, DisplaySettings } from "./types";

/** The wiki's numbers, as in src-tauri/src/build/rules.rs. */
export const DEFAULT_RULES: BuildRules = {
  apPerLevel: 5,
  spPerLevel: 3,
  manualApLevel: 11,
  firstSp: { warrior: 11, magician: 9, bowman: 11, thief: 11 },
  secondJobLevel: 30,
};

export const display = $state<DisplaySettings>({
  opacity: 1,
  scale: 1,
  alwaysOnTop: true,
  compactShape: "card",
  hotkeys: { enabled: true, keys: {} },
  travel: { enabled: false, cabs: true, vipCabs: false, scrolls: true },
  live: { enabled: false, interval: 2, pauseAfter: 15, window: null },
  buildRules: DEFAULT_RULES,
});

/** Changes whenever the travel routes may use changes, for effects that ask for routes. */
export function travelKey(): string {
  const t = display.travel;
  return t.enabled ? `${t.cabs}${t.vipCabs}${t.scrolls}` : "off";
}

let watching = false;

/** Loads the settings once and follows changes made in the Settings window. */
export function watchSettings() {
  if (watching) return;
  watching = true;
  backend.settings().then((s) => Object.assign(display, s));
  backend.onSettings((s) => Object.assign(display, s));
}

/** Saves a change; every window receives it through the settings event. */
export async function changeSettings(change: Partial<DisplaySettings>) {
  Object.assign(display, change);
  Object.assign(display, await backend.updateSettings({ ...display }));
}
