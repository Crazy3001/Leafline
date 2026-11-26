// The window's five tabs and the Tools list. A tab is where you are; a tool is something you open
// over it. Everything Leafline does is reached from a tab, the Tools list or search.

import {
  ChartColumn,
  ChartLine,
  FlaskConical,
  Gem,
  Hammer,
  House,
  ListChecks,
  ListTree,
  Map as MapIcon,
  PackageSearch,
  Route,
  Scale,
  Share2,
  Shirt,
  Timer,
  UserRound,
} from "@lucide/svelte";

import type { Panel } from "$lib/types";

export type Tab = "now" | "quests" | "map" | "train" | "me";

export interface TabInfo {
  id: Tab;
  title: string;
  icon: typeof House;
  /**
   * The panel this tab pins into its own window, and whose changes put a dot on the tab.
   */
  panel: Panel;
  /** What the pin button says it pins. */
  pinName: string;
}

export const TABS: TabInfo[] = [
  { id: "now", title: "Now", icon: House, panel: "now", pinName: "the session and quest" },
  { id: "quests", title: "Quests", icon: ListChecks, panel: "quests", pinName: "Quests" },
  { id: "map", title: "Map", icon: MapIcon, panel: "map", pinName: "Map" },
  { id: "train", title: "Train", icon: ChartLine, panel: "train", pinName: "Train" },
  { id: "me", title: "Me", icon: UserRound, panel: "character", pinName: "your character" },
];

export function tabInfo(id: Tab): TabInfo {
  return TABS.find((t) => t.id === id) ?? TABS[0];
}

export function tabId(id: string | null | undefined): Tab | null {
  return TABS.some((t) => t.id === id) ? (id as Tab) : null;
}

/**
 * Where a link to one of the old panels lands now. Lookup has no tab: it is the search sheet.
 */
export function tabOfPanel(panel: Panel): Tab | "search" {
  switch (panel) {
    case "character":
      return "me";
    case "item":
      return "search";
    default:
      return panel;
  }
}

/** A tool that opens as a page over the tab. */
export type ToolPage = "crafting" | "sell" | "timers" | "drops" | "tracked" | "potions" | "route" | "share";

/** What picking a tool does: open its page, jump to where it lives, or open its own window. */
export type ToolAction =
  | { page: ToolPage }
  | { tab: Tab }
  | { search: "items" | "gear" }
  | { window: "training" | "build" };

export interface ToolInfo {
  id: string;
  title: string;
  /** One short line under the title. */
  line: string;
  icon: typeof House;
  action: ToolAction;
}

export const TOOLS: ToolInfo[] = [
  { id: "crafting", title: "Crafting", line: "Recipes and levelling", icon: Hammer, action: { page: "crafting" } },
  { id: "sell", title: "Sell or keep", line: "Sort your open bag", icon: Scale, action: { page: "sell" } },
  { id: "timers", title: "Timers", line: "Ones you start yourself", icon: Timer, action: { page: "timers" } },
  { id: "drops", title: "Drop log", line: "Drops you've logged", icon: Gem, action: { page: "drops" } },
  { id: "tracked", title: "Tracked items", line: "Items you're after", icon: PackageSearch, action: { page: "tracked" } },
  { id: "potions", title: "Potion costs", line: "What this session's potions cost", icon: FlaskConical, action: { page: "potions" } },
  { id: "gear", title: "Gear finder", line: "Best gear for your level", icon: Shirt, action: { search: "gear" } },
  { id: "log", title: "Training log", line: "Past sessions, records", icon: ChartColumn, action: { window: "training" } },
  { id: "build", title: "Build planner", line: "Skills, stats and gear", icon: ListTree, action: { window: "build" } },
  { id: "share", title: "Share card", line: "A session's card to post", icon: Share2, action: { page: "share" } },
  { id: "route", title: "Route planner", line: "Any map to any map", icon: Route, action: { page: "route" } },
];

export function toolPageTitle(page: ToolPage): string {
  return TOOLS.find((t) => "page" in t.action && t.action.page === page)?.title ?? "Tool";
}
