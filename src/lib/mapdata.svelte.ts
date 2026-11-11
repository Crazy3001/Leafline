// The map data cache's state, shared by the map card and Settings.
// Data is only ever downloaded when the user presses the button.

import { backend } from "./api";
import type { MapDataStatus } from "./types";

export const OSMS_CREDIT = "Data from OSMS Data Explorer (https://osmsdataexplorer.com), CC BY 4.0";
export const OSMS_URL = "https://osmsdataexplorer.com";
export const WIKI_CREDIT = "Player-observed drops from mapleclassic.wiki, CC BY-NC-SA 4.0";
export const WIKI_URL = "https://mapleclassic.wiki";
// Facts OSMS does not publish (who takes each quest in), as the game client records them.
export const DATA_CREDIT = "Quest turn-ins from the game client, via Leafline-data";
export const DATA_URL = "https://github.com/Crazy3001/Leafline-data";

export const mapData = $state<{
  status: MapDataStatus | null;
  downloading: boolean;
  error: string | null;
  /** What the last download did, for a short confirmation. */
  message: string | null;
}>({ status: null, downloading: false, error: null, message: null });

let watching = false;

/** Loads the cache's status once and follows changes made from any window. */
export function watchMapData() {
  if (watching) return;
  watching = true;
  backend
    .mapDataStatus()
    .then((s) => {
      mapData.status = s;
    })
    .catch((e) => {
      mapData.error = String(e);
    });
  backend.onMapData((s) => {
    mapData.status = s;
  });
}

export async function downloadMapData(force: boolean) {
  if (mapData.downloading) return;
  mapData.downloading = true;
  mapData.error = null;
  mapData.message = null;
  try {
    const { report, status } = await backend.downloadMapData(force);
    mapData.status = status;
    mapData.message = !report.checked
      ? "Checked within the last day."
      : report.downloaded.length
        ? `Updated ${report.downloaded.length} file${report.downloaded.length === 1 ? "" : "s"}.`
        : "Already up to date.";
  } catch (e) {
    mapData.error = String(e);
  } finally {
    mapData.downloading = false;
  }
}

/** `2 hours ago`, for when the cache was last checked. */
export function ago(unixSeconds: number | null | undefined): string {
  if (unixSeconds == null) return "never";
  const minutes = Math.round((Date.now() / 1000 - unixSeconds) / 60);
  if (minutes < 1) return "just now";
  if (minutes < 60) return `${minutes} min ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 48) return `${hours} h ago`;
  return `${Math.round(hours / 24)} days ago`;
}
