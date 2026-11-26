// The look every window is drawn in: one of six sets of colours, fonts, corners and outlines
// over the same layout (app.css for Ember Dark, ui/themes.css for the rest). Focus is the default.

import { display, settingsLoad, watchSettings } from "$lib/settings.svelte";
import type { Theme } from "$lib/types";

export interface ThemeChoice {
  id: Theme;
  name: string;
  /** One line under the name in the picker. */
  line: string;
  dark: boolean;
}

export const THEMES: ThemeChoice[] = [
  { id: "focus-dark", name: "Focus Dark", line: "Forest and leaf, big clear type", dark: true },
  { id: "focus-light", name: "Focus Light", line: "Soft sage, big clear type", dark: false },
  { id: "ember-dark", name: "Ember Dark", line: "Graphite and amber", dark: true },
  { id: "ember-light", name: "Ember Light", line: "Warm stone, dark header", dark: false },
  { id: "sprout-light", name: "Sprout Light", line: "Cream, ink and stickers", dark: false },
  { id: "sprout-dark", name: "Sprout Dark", line: "Plum, ink and stickers", dark: true },
];

/** Remembered per window so a window opens in its look before the settings arrive. */
const KEY = "leafline.theme";

function known(value: unknown): value is Theme {
  return THEMES.some((t) => t.id === value);
}

function remembered(): Theme | null {
  try {
    const value = localStorage.getItem(KEY);
    return known(value) ? value : null;
  } catch {
    return null;
  }
}

function apply(theme: Theme) {
  document.documentElement.dataset.theme = theme;
  try {
    localStorage.setItem(KEY, theme);
  } catch {
    // The look still applies; it just isn't remembered for this window's next start.
  }
}

/**
 * Keeps <html data-theme> in step with the settings. Call once from the root layout, inside an
 * effect root (a component's script).
 */
export function followTheme() {
  if (typeof document === "undefined") return;
  // Before the settings load, the look this window had last time. From then on the setting wins.
  const first = remembered();
  if (first) document.documentElement.dataset.theme = first;
  watchSettings();
  $effect(() => {
    const theme = known(display.theme) ? display.theme : "focus-dark";
    if (settingsLoad.done || !first) apply(theme);
  });
}
