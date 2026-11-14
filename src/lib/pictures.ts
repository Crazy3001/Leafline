// Game pictures: item, monster and NPC icons, map pictures and area emblems. The backend's `pic`
// scheme hands them over, downloading each from OSMS Data Explorer the first time and keeping it
// on this PC (src-tauri/src/mapdata/pictures.rs). With the setting off, or a picture missing,
// names stand alone.

import { isTauri } from "@tauri-apps/api/core";

import { display } from "./settings.svelte";

/** `mapview` is a map's picture cut to what is drawn and made smaller, for the map section. */
export type PictureKind = "item" | "mob" | "npc" | "skill" | "map" | "minimap" | "mark" | "mapview";

/**
 * Where the `pic` scheme answers: WebView2 reaches custom schemes as `http://<scheme>.localhost`.
 * A plain browser preview asks `/__pic/`, which only a preview server set up for it answers.
 */
const BASE = !isTauri() ? "/__pic/" : navigator.userAgent.includes("Windows") ? "http://pic.localhost/" : "pic://localhost/";

/** Pictures this window found missing, so a list doesn't ask again for each row. */
const missing = new Set<string>();

/** The address of a picture by id, or by name when that is all a link knows; null when off. */
export function pictureUrl(kind: PictureKind, key: number | string | null | undefined): string | null {
  if (!display.showPictures || key == null) return null;
  if (typeof key === "number") return `${BASE}${kind}/${key}`;
  const name = key.trim();
  return name ? `${BASE}${kind}/name/${encodeURIComponent(name)}` : null;
}

export function pictureMissing(src: string): boolean {
  return missing.has(src);
}

export function markMissing(src: string) {
  missing.add(src);
}
