// Display settings shared by the overlay, pinned panels and the Settings window.

import { backend } from "./api";
import type { BuildRules, DisplaySettings } from "./types";

/** As in src-tauri/src/build/rules.rs, in the same order. */
export const DEFAULT_RULES: BuildRules = {
  apPerLevel: 5,
  spPerLevel: 3,
  beginnerSpPerLevel: 1,
  advancementSp: 1,
  firstJobLevel: 10,
  secondJobLevel: 30,
  secondSpBuysFirst: false,
  gearCounts: true,
};

export const display = $state<DisplaySettings>({
  opacity: 1,
  scale: 1,
  alwaysOnTop: true,
  compactShape: "card",
  hotkeys: { enabled: true, keys: {} },
  travel: { enabled: false, cabs: true, vipCabs: false, scrolls: true },
  live: { enabled: true, interval: 2, pauseAfter: 15, window: null },
  buildRules: DEFAULT_RULES,
  framesFolder: null,
  setupDone: false,
  updateCheck: false,
});

const NUMPAD: Record<string, string> = { Add: "+", Subtract: "-", Multiply: "*", Divide: "/", Decimal: ".", Enter: "Enter" };

/** How a key is written for people: `ScrollLock` as `Scroll Lock`, `Super` as `Win`, `Numpad5` as `Num 5`. */
export function keyName(keys: string): string {
  return keys
    .split("+")
    .map((k) =>
      k.startsWith("Numpad")
        ? `Num ${NUMPAD[k.slice(6)] ?? k.slice(6)}`
        : ({ ScrollLock: "Scroll Lock", Super: "Win", PageUp: "Page Up", PageDown: "Page Down" }[k] ?? k),
    )
    .join("+");
}

/** The hotkey for an action as people read it, or "" if it has none or hotkeys are off. */
export function hotkey(action: string): string {
  const keys = display.hotkeys.enabled ? (display.hotkeys.keys[action] ?? "") : "";
  return keys ? keyName(keys) : "";
}

/** ` (F10)` after a button's title, or nothing when that action has no hotkey. */
export function keyHint(action: string): string {
  const keys = hotkey(action);
  return keys ? ` (${keys})` : "";
}

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
