// The overlay's panels: one tab each, and each can be pinned into a window of its own.

import { Map as MapIcon, Navigation, ScrollText, Search, Swords, UserRound } from "@lucide/svelte";

import { mapLabel } from "./format";
import type { Analysis, Panel, PinId, SessionSummary } from "./types";

export interface PanelInfo<Id extends PinId = Panel> {
  id: Id;
  title: string;
  icon: typeof UserRound;
}

/** Width of a pinned window, matching `WIDTH` in src-tauri/src/app/pins.rs. */
export const PIN_WIDTH = 340;

/**
 * How tall a pinned window grows before it scrolls. A pinned window shows its panel in short
 * (see PanelView), so most stay well under these; the caps keep a long quest list or map from
 * covering the game.
 */
export const PIN_MAX_HEIGHT: Record<PinId, number> = {
  character: 520,
  map: 480,
  train: 400,
  quests: 520,
  item: 560,
  route: 200,
};

/** The "Go here" next step in a window of its own: pinned from the next-step bar, not a tab. */
export const ROUTE_PIN: PanelInfo<"route"> = { id: "route", title: "Route", icon: Navigation };

/** What a pinned window shows, from its address. */
export function pinId(id: string | null | undefined): PinId | null {
  return id === "route" ? "route" : panelId(id);
}

export function pinInfo(id: PinId): PanelInfo<PinId> {
  return id === "route" ? ROUTE_PIN : panelInfo(id);
}

export const PANELS: PanelInfo[] = [
  { id: "character", title: "Character", icon: UserRound },
  { id: "map", title: "Map", icon: MapIcon },
  { id: "train", title: "Train", icon: Swords },
  { id: "quests", title: "Quests", icon: ScrollText },
  { id: "item", title: "Lookup", icon: Search },
];

/** Stats and Build were tabs of their own until 2026-11-04; remembered tabs and pinned
 *  windows can still name them. */
const RENAMED: Record<string, Panel> = { stats: "character", build: "character" };

/** The panel an id names, old ones included, or null for one that isn't a panel. */
export function panelId(id: string | null | undefined): Panel | null {
  if (id == null) return null;
  if (id in RENAMED) return RENAMED[id];
  return PANELS.some((p) => p.id === id) ? (id as Panel) : null;
}

export function panelInfo(id: Panel): PanelInfo {
  return PANELS.find((p) => p.id === id) ?? PANELS[0];
}

/**
 * What a panel currently shows, boiled down so a change can be noticed: a tab whose
 * signature changes while it is not open gets a dot. Null when the panel has nothing.
 */
export function signature(panel: Panel, analysis: Analysis | null, session: SessionSummary | null): string | null {
  switch (panel) {
    case "character":
      // Changes with the character's level, which moves the build plan on too.
      return analysis?.hud || session ? `${analysis?.hud?.level ?? session?.level}` : null;
    case "map":
      return analysis?.map ? mapLabel(analysis.map) : null;
    case "train":
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
