// The player's saved training spots ("your spots"), shared so a bookmark anywhere shows
// everywhere: the Where to train list, the Map tab, a map's page in Lookup and the Train tab's
// tables.

import { backend } from "./api";
import type { SavedSpot } from "./types";

export const spots = $state<{ saved: SavedSpot[]; error: string | null }>({ saved: [], error: null });

let watching = false;

export function watchSpots() {
  if (watching) return;
  watching = true;
  backend
    .savedSpots()
    .then((list) => (spots.saved = list))
    .catch(() => (spots.saved = []));
  void backend.onSavedSpots((list) => (spots.saved = list));
}

export const isSpotSaved = (mapId: number) => spots.saved.some((s) => s.mapId === mapId);

/** Saves a map to the player's spots, or takes it out. */
export async function toggleSpot(mapId: number, name: string) {
  spots.error = null;
  try {
    spots.saved = await backend.setSpotSaved(mapId, name, !isSpotSaved(mapId));
  } catch (e) {
    spots.error = String(e);
  }
}
