// The overlay's panels: one tab each, and each can be pinned into a window of its own.

import { Activity, Map as MapIcon, Milestone, ScrollText, Search, Swords } from "@lucide/svelte";

import { mapLabel } from "./format";
import type { Analysis, Panel, SessionSummary } from "./types";

export interface PanelInfo {
  id: Panel;
  title: string;
  icon: typeof Activity;
}

/** Width of a pinned window, matching `WIDTH` in src-tauri/src/app/pins.rs. */
export const PIN_WIDTH = 340;

export const PANELS: PanelInfo[] = [
  { id: "stats", title: "Stats", icon: Activity },
  { id: "map", title: "Map", icon: MapIcon },
  { id: "train", title: "Train", icon: Swords },
  { id: "quests", title: "Quests", icon: ScrollText },
  { id: "item", title: "Lookup", icon: Search },
  { id: "build", title: "Build", icon: Milestone },
];

export function panelInfo(id: Panel): PanelInfo {
  return PANELS.find((p) => p.id === id) ?? PANELS[0];
}

/**
 * What a panel currently shows, boiled down so a change can be noticed: a tab whose
 * signature changes while it is not open gets a dot. Null when the panel has nothing.
 */
export function signature(panel: Panel, analysis: Analysis | null, session: SessionSummary | null): string | null {
  switch (panel) {
    case "stats":
      return analysis?.hud || session ? `${analysis?.hud?.level}` : null;
    case "map":
      return analysis?.map ? mapLabel(analysis.map) : null;
    case "train":
    case "build":
      // Changes with the character's level; the list itself is looked up, not read.
      return (analysis?.hud?.level ?? session?.level) != null ? `${analysis?.hud?.level ?? session?.level}` : null;
    case "quests":
      return analysis?.quest?.quests.length
        ? analysis.quest.quests.map((q) => q.title + q.objectives.map((o) => o.current).join()).join("|")
        : null;
    case "item":
      return analysis?.tooltips.length ? analysis.tooltips.map((t) => t.name).join("|") : null;
  }
}
