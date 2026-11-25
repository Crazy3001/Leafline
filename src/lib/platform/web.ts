// The website's backend: the program's command table, run by the same Rust in a worker (see
// storage.svelte.ts), with the members that move windows, read the game or touch files replaced
// by what a web page does instead. Nothing here sends anything anywhere: downloads are the
// reference data the worker fetches, and saved files go to the player's own Downloads.

import { goto } from "$app/navigation";
import { base } from "$app/paths";

import type { Backend } from "../api";
import type { DeskStatus, MapPicture, PinStatus } from "../types";
import { emit, listen } from "./bus";
import { commandBackend } from "./commands";
import { setMapView, setPictureTable } from "./pictures.svelte";
import { fetchBagPictures, liveMisread, liveReadings, liveSight, liveStatus, resetLiveSession, setLiveHeld, startLive, stopLive } from "./live";
import { closePip, openPip, pipOpen } from "./pip";
import { checkRecords, exportRecords, importRecords, invoke, recordsSummary } from "./storage.svelte";
import { noteRecordsSaved } from "../recordsfile";

/** Asks the player for a file, the way a page does. Null if they close the picker. */
function chooseFile(accept: string): Promise<File | null> {
  return new Promise((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = accept;
    input.onchange = () => resolve(input.files?.[0] ?? null);
    input.oncancel = () => resolve(null);
    input.click();
  });
}

const notHere = (what: string) => () =>
  Promise.reject(new Error(`${what} is in the Windows program only.`));

/** Saves bytes as a file in the player's Downloads, the way a site does. */
export function download(bytes: BlobPart, name: string, type: string) {
  const url = URL.createObjectURL(new Blob([bytes], { type }));
  const link = document.createElement("a");
  link.href = url;
  link.download = name;
  link.click();
  setTimeout(() => URL.revokeObjectURL(url), 10_000);
}

/** Opens a link in a new tab, without telling it where it came from. */
function openTab(url: string) {
  const parsed = new URL(url.trim());
  if (parsed.protocol !== "https:" && parsed.protocol !== "http:") throw new Error("Only web addresses open.");
  window.open(parsed.href, "_blank", "noopener,noreferrer");
}

const desk: DeskStatus = { open: true, onTop: false };

function pins(): PinStatus {
  return { pinned: pipOpen() ? ["now"] : [], hidden: false };
}

const shared = commandBackend(invoke, listen, emit);

/** Picture addresses come with the map data: fetched now, and again when it changes. */
function loadPictures() {
  invoke<Parameters<typeof setPictureTable>[0]>("picture_index").then(setPictureTable, () => {});
}
loadPictures();
void listen("map-data", loadPictures);

export const webBackend: Backend = {
  ...shared,
  pickFolder: async () => null,
  overlayStatus: async () => ({ visible: true, clickThrough: false, compact: false }),
  setClickThrough: async () => {},
  hideOverlay: async () => {},
  setCompact: async () => {},
  toggleOverlay: async () => {},
  shortcuts: async () => [],
  setHotkeysQuiet: async () => {},
  // Reading a shared game window, where the browser can (see live.ts).
  liveStatus: async () => liveStatus(),
  readerSight: async () => liveSight(),
  setLiveHeld: async (held) => setLiveHeld(held),
  resetSession: () => resetLiveSession(),
  startLive: () => startLive(),
  stopLive: () => stopLive(),
  liveWindows: async () => [],
  saveLiveFrame: notHere("Saving game pictures"),
  // "This misread": the picture and what was read from it, as two downloads beside each other.
  async saveMisread() {
    const { png, report } = await liveMisread();
    const stamp = new Date().toISOString().slice(0, 19).replace(/[T:]/g, "-");
    download(png, `leafline-misread-${stamp}.png`, "image/png");
    download(report, `leafline-misread-${stamp}.json`, "application/json");
    return `leafline-misread-${stamp}`;
  },
  saveItemPicture: notHere("Saving game pictures"),
  // The worker cuts the map's picture with the shared code; the page shows it as a data address.
  async mapPicture(id) {
    const cut = await invoke<{ width: number; height: number; points: MapPicture["points"]; jpeg: string } | null>(
      "map_picture",
      { id },
    );
    if (!cut) return null;
    setMapView(id, `data:image/jpeg;base64,${cut.jpeg}`);
    return { width: cut.width, height: cut.height, points: cut.points };
  },
  showQuest: (id) => emit("show-quest", id),
  showMonster: (id, text) => emit("show-monster", { id, text }),
  showPlace: (mode, id) => emit("show-place", { mode, id }),
  showItem: (id, text) => emit("show-item", { id, text }),
  openLink: async (url) => openTab(url),
  openGuide: async (url) => openTab(url),
  async saveImage(png, name) {
    download(png as BlobPart, name, "image/png");
    return true;
  },
  openDataFolder: async () => {},
  framesFolder: async () => "",
  openFramesFolder: async () => {},
  diagnostics: async () =>
    [`Leafline website ${await invoke<string>("version")}`, `Browser: ${navigator.userAgent}`].join("\n"),
  showTraining: () => emit("desk-tab", "log"),
  showBuild: () => emit("desk-tab", "planner"),
  showDesk: async () => desk,
  deskStatus: async () => desk,
  deskTakeTab: async () => null,
  setDeskOnTop: async () => desk,
  async setPageZoom(zoom) {
    document.documentElement.style.setProperty("zoom", String(zoom));
    // Zoom scales viewport units too, so full-height pages divide it back out (or at 150% the
    // page is half again as tall as the window, and the sidebar's foot can't be reached).
    document.documentElement.style.setProperty("--page-zoom", String(zoom));
  },
  monitorCount: async () => 1,
  setTyping: async () => {},
  async exportCsv(what) {
    const { text, rows } = await invoke<{ text: string; rows: number }>("export_csv", { what });
    const stamp = new Date().toISOString().slice(0, 10);
    download(text, `leafline-${what === "drops" ? "drop-log" : what === "loot" ? "loot" : "sessions"}-${stamp}.csv`, "text/csv");
    return rows;
  },
  readCharacter: notHere("Reading the character's windows"),
  // Sell or keep runs by itself while the inventory is open (see live.ts).
  async bagLookFurther() {
    await fetchBagPictures({ wide: true });
    await invoke("bag_look_further");
  },
  lastBag: () => invoke("last_bag"),
  currentReadings: async () => liveReadings(),
  pinStatus: async () => pins(),
  async pinPanel(panel) {
    if (panel === "now") await openPip(() => void emit("pins", pins()));
    const status = pins();
    void emit("pins", status);
    return status;
  },
  async unpinPanel(panel) {
    if (panel === "now") closePip();
    const status = pins();
    void emit("pins", status);
    return status;
  },
  popOutCard: async () => {},
  showCard: async () => {},
  cardIsLive: async () => false,
  pinCard: async () => {},
  onPinResized: async () => () => {},
  showPanel: (panel) => emit("show-panel", panel),
  updateStatus: async () => ({ state: "off" }),
  checkForUpdate: async () => ({ state: "off" }),
  installUpdate: notHere("Updating"),
  showSettings: () => goto(`${base}/settings`),
  showSetup: () => goto(`${base}/settings`),
  finishSetup: async () => {},
  editTravelFile: notHere("Editing the travel file"),
  version: () => invoke<string>("version"),
  fitWindow: async () => {},
  async saveRecords() {
    const bytes = await exportRecords();
    const stamp = new Date().toISOString().slice(0, 10);
    download(bytes as BlobPart, `leafline-records-${stamp}.leafline`, "application/octet-stream");
    noteRecordsSaved();
    return recordsSummary();
  },
  async pickRecords() {
    const file = await chooseFile(".leafline,.sqlite3");
    return file ? checkRecords(new Uint8Array(await file.arrayBuffer())) : null;
  },
  useRecords: () => importRecords(),
  // The website keeps no daily copies: the browser holds its records (see Settings → Data).
  listBackups: async () => [],
  checkBackup: notHere("Restoring a backup"),
};
