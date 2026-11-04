// The map and NPC searches as the Lookup tab shows them. They live here so that switching tabs
// keeps a search, and so a link from another window or panel can bring a map or NPC up.

import { backend } from "./api";
import { lookup } from "./items.svelte";
import type { MapQuery, NpcQuery } from "./types";

export const blankMapQuery = (): MapQuery => ({ text: "", kind: null, region: null, area: null, levelMin: null, levelMax: null, sort: null, offset: 0 });
export const blankNpcQuery = (): NpcQuery => ({ text: "", kind: null, region: null, area: null, offset: 0 });

export const mapView = $state<{ id: number | null; trail: number[]; query: MapQuery }>({
  /** The map open. */
  id: null,
  /** The maps opened on the way here, to go back through. */
  trail: [],
  query: blankMapQuery(),
});

export const npcView = $state<{ id: number | null; query: NpcQuery }>({ id: null, query: blankNpcQuery() });

/** Goes up with each link followed, so the overlay can open the tab. */
export const placeLinks = $state({ seen: 0 });

let watching = false;

export function watchPlaces() {
  if (watching) return;
  watching = true;
  backend.onShowPlace((link) => {
    if (link.mode === "maps") {
      mapView.id = link.id;
      mapView.trail = [];
    } else {
      npcView.id = link.id;
    }
    lookup.mode = link.mode;
    placeLinks.seen += 1;
  });
}

/** Brings an NPC up by name, or does nothing if no NPC goes by it. */
export async function showNpcNamed(name: string) {
  const id = await backend.findNpc(name);
  if (id != null) await backend.showPlace("npcs", id);
}
