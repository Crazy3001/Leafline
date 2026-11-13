// The desk view's tabs: each a set of columns side by side, each column holding one panel or two
// stacked. A few tabs come ready (training, questing, character, the training log, the build
// planner) and two are left for the player; all of them can be renamed and rearranged, and what
// the player sets is saved for the next session. A tab can go back to how it started.

import { PANELS } from "./panels";
import type { Panel } from "./types";

/** What a desk view column can show: the overlay's panels, the training log and the build planner. */
export type DeskPanel = Panel | "log" | "planner";

/** Every desk panel, in the order the Change menu lists them. */
export const DESK_PANELS: DeskPanel[] = [...PANELS.map((p) => p.id), "log", "planner"];

/** One column, top to bottom: one panel, or two sharing its height. */
export type DeskColumn = DeskPanel[];

/** One tab of the desk view: a name and the columns it shows. */
export interface DeskTab {
  id: string;
  name: string;
  columns: DeskColumn[];
}

export interface DeskPrefs {
  /** Always every tab in DEFAULT_TABS, in that order, with the player's names and layouts. */
  tabs: DeskTab[];
  /** The tab shown, by id. */
  active: string;
  /** How large the page is drawn, for reading from further away than the overlay. */
  zoom: number;
}

/** The tabs a desk starts with, and each tab's layout to go back to. The training log and the
 *  build planner start alone at the window's full width; the two Custom tabs start as a
 *  suggestion to change. Every tab can be renamed and rearranged. */
export const DEFAULT_TABS: DeskTab[] = [
  { id: "training", name: "Training", columns: [["train"], ["map"], ["character"]] },
  { id: "questing", name: "Questing", columns: [["quests"], ["map"], ["item"]] },
  { id: "character", name: "Character", columns: [["character"], ["item"]] },
  { id: "log", name: "Training log", columns: [["log"]] },
  { id: "planner", name: "Build planner", columns: [["planner"]] },
  { id: "custom1", name: "Custom 1", columns: [["map"], ["quests"]] },
  { id: "custom2", name: "Custom 2", columns: [["train"], ["item"]] },
];

export const MAX_NAME = 24;

/** How wide a column is against the others: the training log's tables and graphs, and the
 *  planner's tabs beside its timeline, want room. */
export function weight(column: DeskColumn): number {
  return column.includes("log") || column.includes("planner") ? 2 : 1;
}

export const ZOOMS = [1, 1.25, 1.5];
export const MAX_COLUMNS = 4;
/** A column holds at most this many panels. */
const MAX_STACK = 2;

const KEY = "leafline.desk";

const copyColumns = (columns: DeskColumn[]) => columns.map((c) => [...c]);

/** A tab as it starts. */
export function defaultTab(id: string): DeskTab {
  const tab = DEFAULT_TABS.find((t) => t.id === id) ?? DEFAULT_TABS[0];
  return { id: tab.id, name: tab.name, columns: copyColumns(tab.columns) };
}

export function loadPrefs(): DeskPrefs {
  const fresh = (): DeskPrefs => ({ tabs: DEFAULT_TABS.map((t) => defaultTab(t.id)), active: DEFAULT_TABS[0].id, zoom: 1 });
  try {
    const saved = JSON.parse(localStorage.getItem(KEY) ?? "null");
    if (!saved || typeof saved !== "object") return fresh();
    const tabs = DEFAULT_TABS.map((d) => {
      const own = Array.isArray(saved.tabs) ? saved.tabs.find((t: { id?: unknown }) => t?.id === d.id) : null;
      const name = typeof own?.name === "string" ? own.name.trim().slice(0, MAX_NAME) : "";
      const columns = valid(own?.columns)
        ? own.columns
        : // The layout of before tabs: the player's own one lives on in the first Custom tab.
          d.id === "custom1" && valid(saved.mine)
          ? saved.mine
          : copyColumns(d.columns);
      return { id: d.id, name: name || d.name, columns } as DeskTab;
    });
    const active = tabs.some((t) => t.id === saved.active) ? saved.active : tabs[0].id;
    const zoom = ZOOMS.includes(saved.zoom) ? saved.zoom : 1;
    return { tabs, active, zoom };
  } catch {
    return fresh();
  }
}

export function savePrefs(prefs: DeskPrefs) {
  try {
    localStorage.setItem(KEY, JSON.stringify(prefs));
  } catch {
    // A convenience only: the default tabs still work.
  }
}

/** A saved layout must still name only panels that exist, each once, in 1 to 4 columns. */
function valid(columns: unknown): columns is DeskColumn[] {
  if (!Array.isArray(columns) || columns.length < 1 || columns.length > MAX_COLUMNS) return false;
  const ids = DESK_PANELS as string[];
  const seen = new Set<string>();
  for (const column of columns) {
    if (!Array.isArray(column) || column.length < 1 || column.length > MAX_STACK) return false;
    for (const panel of column) {
      if (typeof panel !== "string" || !ids.includes(panel) || seen.has(panel)) return false;
      seen.add(panel);
    }
  }
  return true;
}

/** Where a panel is shown, as column and place in it. */
export function find(columns: DeskColumn[], panel: DeskPanel): [number, number] | null {
  for (let c = 0; c < columns.length; c++) {
    const s = columns[c].indexOf(panel);
    if (s >= 0) return [c, s];
  }
  return null;
}

/** Panels no column shows, in the Change menu's order. */
export function unused(columns: DeskColumn[]): DeskPanel[] {
  return DESK_PANELS.filter((id) => !find(columns, id));
}

const copy = copyColumns;

/** Shows `panel` at a place. One already shown elsewhere swaps places with what was there. */
export function place(columns: DeskColumn[], column: number, slot: number, panel: DeskPanel): DeskColumn[] {
  const next = copy(columns);
  const here = next[column][slot];
  if (here === panel) return next;
  const from = find(next, panel);
  if (from) next[from[0]][from[1]] = here;
  next[column][slot] = panel;
  return next;
}

/** Adds a second panel under the first in a column, the first one not shown anywhere. */
export function split(columns: DeskColumn[], column: number): DeskColumn[] {
  const spare = unused(columns)[0];
  if (!spare || columns[column].length >= MAX_STACK) return columns;
  const next = copy(columns);
  next[column].push(spare);
  return next;
}

/** Takes a panel out. A column left empty goes, unless it is the only one. */
export function remove(columns: DeskColumn[], column: number, slot: number): DeskColumn[] {
  if (columns.length === 1 && columns[0].length === 1) return columns;
  const next = copy(columns);
  next[column].splice(slot, 1);
  return next.filter((c) => c.length > 0);
}

/** Adds columns of panels not yet shown, or drops columns from the right. */
export function withColumns(columns: DeskColumn[], count: number): DeskColumn[] {
  const next = copy(columns);
  while (next.length > count) next.pop();
  while (next.length < count) {
    const spare = unused(next)[0];
    if (spare) {
      next.push([spare]);
      continue;
    }
    // Every panel is shown: take the lower one of a stacked column.
    const stacked = next.findIndex((c) => c.length > 1);
    if (stacked < 0) break;
    next.push([next[stacked].pop()!]);
  }
  return next;
}
