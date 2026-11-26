// The region and map the player is in, by the map data: for the colour washed behind the player
// bar and the desk's banner picture.
// Each region gets a steady hue from its name, so the same place always looks the same and a new
// one changes the colour; nothing here knows the game's regions by name. Until the minimap's map
// is matched (or with no map data) the last region stays, so the colour doesn't flicker.

import { backend } from "./api";
import { mapData, watchMapData } from "./mapdata.svelte";
import { live } from "./readings.svelte";
import type { MapDetails } from "./types";

export const place = $state<{ region: string | null; mapId: number | null; card: MapDetails | null }>({
  region: null,
  mapId: null,
  card: null,
});

/** Hues that sit well under text on every look: greens, teals, blues, violet, rose, amber, rust. */
const HUES = [142, 168, 196, 218, 262, 318, 34, 14];

/** A region's hue, the same every time, or null with no region. */
export function regionHue(region: string | null): number | null {
  if (!region) return null;
  let h = 0;
  for (const c of region) h = (h * 31 + c.charCodeAt(0)) >>> 0;
  return HUES[h % HUES.length];
}

let watching = false;
let asked = "";

/** Follows the map the minimap shows, from any window that wants the region. */
export function watchRegion() {
  if (watching) return;
  watching = true;
  watchMapData();
  $effect.root(() => {
    $effect(() => {
      const map = live.analysis?.map;
      const ready = mapData.status?.ready ?? false;
      const key = JSON.stringify([map?.name, map?.street, map?.twin, ready, mapData.status?.dataUpdatedAt]);
      if (key === asked) return;
      asked = key;
      if (!ready || !map?.name) return;
      backend
        .findMap(map.name, map.street ?? null, map.twin ?? null)
        .then((found) => {
          if (key === asked && found && found.score >= 0.85) {
            place.region = found.card.region ?? null;
            place.mapId = found.card.map.id;
            place.card = found.card;
          }
        })
        .catch(() => {
          // Keeps the last region.
        });
    });
  });
}
