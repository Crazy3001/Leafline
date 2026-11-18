// What the Character tab and the item check share with the editor window.

import { backend } from "./api";
import { mapData, watchMapData } from "./mapdata.svelte";
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
    if (wornWatching) void loadWorn();
  });
}

/**
 * The gear the active build says the character wears, by item and by name, so every item name on
 * screen can show it in a colour of its own.
 */
export const worn = $state<{ ids: number[]; names: string[] }>({ ids: [], names: [] });

let wornWatching = false;

/** A name as worn gear is matched: no upgrade count, any case. */
export const wornKey = (name: string) => name.replace(/\s*\(\+\d+\)\s*$/, "").trim().toLowerCase();

async function loadWorn() {
  try {
    const plan = await backend.activeBuild(null);
    const pieces = plan?.build.start.pieces ?? [];
    worn.ids = pieces.map((p) => p.item).filter((id): id is number => id != null);
    worn.names = pieces.map((p) => wornKey(p.name));
  } catch {
    // Without map data or a build there is nothing worn to show.
  }
}

export function watchWorn() {
  if (wornWatching) return;
  wornWatching = true;
  watchBuilds();
  watchMapData();
  // Builds need the map data: load again once it is there, and after each download.
  $effect.root(() => {
    $effect(() => {
      void mapData.status?.ready;
      void mapData.status?.dataUpdatedAt;
      void loadWorn();
    });
  });
}

export function isWorn(id: number | null | undefined, name: string | null | undefined): boolean {
  if (id != null && worn.ids.includes(id)) return true;
  return name != null && worn.names.includes(wornKey(name));
}
