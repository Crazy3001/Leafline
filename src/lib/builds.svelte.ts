// What the Build tab and the item check share with the editor window.

import { backend } from "./api";
import type { BaseStat } from "./types";

export const STATS: { id: BaseStat; label: string }[] = [
  { id: "str", label: "STR" },
  { id: "dex", label: "DEX" },
  { id: "int", label: "INT" },
  { id: "luk", label: "LUK" },
];

export const statLabel = (stat: BaseStat) => stat.toUpperCase();

/** Goes up whenever a build is saved, deleted or chosen in any window. */
export const buildState = $state({ version: 0 });

let watching = false;

export function watchBuilds() {
  if (watching) return;
  watching = true;
  backend.onBuilds(() => {
    buildState.version += 1;
  });
}
