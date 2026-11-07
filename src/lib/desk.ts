// The desk view's layout: columns side by side, each holding one panel or two stacked. Two presets
// cover training and questing; any change the player makes is saved as their own layout, so the
// presets stay as they are to go back to.

import { PANELS } from "./panels";
import type { Panel } from "./types";

/** One column, top to bottom: one panel, or two sharing its height. */
export type DeskColumn = Panel[];

export type PresetId = "training" | "questing" | "mine";

export interface DeskPrefs {
  preset: PresetId;
  /** The player's own layout, once they have changed one. */
  mine: DeskColumn[] | null;
  /** How large the page is drawn, for reading from further away than the overlay. */
  zoom: number;
}

export const PRESETS: Record<Exclude<PresetId, "mine">, DeskColumn[]> = {
  training: [["train"], ["map"], ["character"]],
  questing: [["quests"], ["map"], ["item"]],
};

export const ZOOMS = [1, 1.25, 1.5];
export const MAX_COLUMNS = 4;
/** A column holds at most this many panels. */
const MAX_STACK = 2;

const KEY = "leafline.desk";

export function loadPrefs(): DeskPrefs {
  const fallback: DeskPrefs = { preset: "training", mine: null, zoom: 1 };
  try {
    const saved = JSON.parse(localStorage.getItem(KEY) ?? "null");
    if (!saved || typeof saved !== "object") return fallback;
    const mine = valid(saved.mine) ? saved.mine : null;
    const preset: PresetId =
      saved.preset === "questing" || saved.preset === "training" ? saved.preset : saved.preset === "mine" && mine ? "mine" : "training";
    const zoom = ZOOMS.includes(saved.zoom) ? saved.zoom : 1;
    return { preset, mine, zoom };
  } catch {
    return fallback;
  }
}

export function savePrefs(prefs: DeskPrefs) {
  try {
    localStorage.setItem(KEY, JSON.stringify(prefs));
  } catch {
    // A convenience only: the presets still work.
  }
}

/** A saved layout must still name only panels that exist, each once, in 1 to 4 columns. */
function valid(columns: unknown): columns is DeskColumn[] {
  if (!Array.isArray(columns) || columns.length < 1 || columns.length > MAX_COLUMNS) return false;
  const ids = PANELS.map((p) => p.id as string);
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

export function layoutOf(prefs: DeskPrefs): DeskColumn[] {
  return prefs.preset === "mine" && prefs.mine ? prefs.mine : PRESETS[prefs.preset === "mine" ? "training" : prefs.preset];
}

/** Where a panel is shown, as column and place in it. */
export function find(columns: DeskColumn[], panel: Panel): [number, number] | null {
  for (let c = 0; c < columns.length; c++) {
    const s = columns[c].indexOf(panel);
    if (s >= 0) return [c, s];
  }
  return null;
}

/** Panels no column shows, in tab order. */
export function unused(columns: DeskColumn[]): Panel[] {
  return PANELS.map((p) => p.id).filter((id) => !find(columns, id));
}

const copy = (columns: DeskColumn[]) => columns.map((c) => [...c]);

/** Shows `panel` at a place. One already shown elsewhere swaps places with what was there. */
export function place(columns: DeskColumn[], column: number, slot: number, panel: Panel): DeskColumn[] {
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
