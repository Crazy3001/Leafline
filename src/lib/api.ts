// Typed access to the Rust commands and events. In a plain browser (UI development with
// `npm run dev`) a mock backend stands in, so the pages can be designed without Tauri.

import { getVersion } from "@tauri-apps/api/app";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { LogicalSize } from "@tauri-apps/api/dpi";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open, save } from "@tauri-apps/plugin-dialog";

import type {
  Analysis,
  Calibration,
  DisplaySettings,
  GameWindow,
  LiveStatus,
  TravelInfo,
  Guide,
  History,
  HudValues,
  MapDataDownload,
  MapDataStatus,
  MapDetails,
  MapMatch,
  MapRef,
  MapRoute,
  MeasuredRate,
  NewPrice,
  MonsterDrops,
  OverlayStatus,
  Panel,
  PinStatus,
  PriceLog,
  QuestQuery,
  ReplayOrder,
  ReplayProgress,
  SessionSummary,
  ShortcutInfo,
  TrainingSpots,
} from "./types";

export interface Backend {
  analyzeImage(path: string): Promise<Analysis>;
  /** Teaches the HUD font from a screenshot whose values the user typed in. */
  calibrateHud(path: string, values: HudValues): Promise<Calibration>;
  listImages(folder: string): Promise<string[]>;
  /** Runs every screenshot in the folder through the pipeline as one session. */
  replaySession(folder: string, order: ReplayOrder): Promise<SessionSummary>;
  /** Stops a running replay after its current frame. */
  cancelReplay(): Promise<void>;
  /** An object URL for displaying the image; revoke it when done. */
  imageUrl(path: string): Promise<string>;
  pickImage(): Promise<string | null>;
  pickFolder(): Promise<string | null>;
  overlayStatus(): Promise<OverlayStatus>;
  setClickThrough(enabled: boolean): Promise<void>;
  hideOverlay(): Promise<void>;
  /** Switches the overlay between the full panels and the compact HUD. */
  setCompact(enabled: boolean): Promise<void>;
  toggleOverlay(): Promise<void>;
  showWorkbench(): Promise<void>;
  shortcuts(): Promise<ShortcutInfo[]>;
  liveStatus(): Promise<LiveStatus>;
  /** Pauses or resumes tracking; paused time doesn't count. */
  setLiveHeld(held: boolean): Promise<void>;
  /** Starts a new session. The one before stays in the history. */
  resetSession(): Promise<void>;
  /** Keeps this run in or out of the history. */
  setLiveLogging(logging: boolean): Promise<void>;
  /** Starts reading the game window; a new session starts with it. */
  startLive(): Promise<void>;
  /** Stops after the reading in progress; the session stays, ready to save. */
  stopLive(): Promise<void>;
  /** Windows that could be the game, likely ones first. */
  liveWindows(): Promise<GameWindow[]>;
  /** Saves the last live picture to Pictures\Leafline and returns its path. */
  saveLiveFrame(): Promise<string>;
  onLive(handler: (status: LiveStatus) => void): Promise<UnlistenFn>;
  onFrameSaved(handler: (path: string) => void): Promise<UnlistenFn>;
  /** An image passed on the command line at startup, once. */
  takeLaunchImage(): Promise<string | null>;
  /** Images passed to later launches while the app is already running. */
  onOpenImage(handler: (path: string) => void): Promise<UnlistenFn>;
  onAnalysis(handler: (analysis: Analysis) => void): Promise<UnlistenFn>;
  onReplayProgress(handler: (progress: ReplayProgress) => void): Promise<UnlistenFn>;
  onSession(handler: (session: SessionSummary) => void): Promise<UnlistenFn>;
  onOverlayStatus(handler: (status: OverlayStatus) => void): Promise<UnlistenFn>;

  /** What map data is cached. */
  mapDataStatus(): Promise<MapDataStatus>;
  /** The explicit "Download map data" action. Without `force`, skipped if checked in the last day. */
  downloadMapData(force: boolean): Promise<MapDataDownload>;
  /** The card for an OCR'd minimap header, or null without a close enough match or any data. */
  findMap(name: string, street: string | null): Promise<MapMatch | null>;
  mapCard(id: number): Promise<MapDetails | null>;
  searchMaps(text: string): Promise<MapRef[]>;
  mapRoute(from: number, to: number): Promise<MapRoute | null>;
  /** Player-observed drops from mapleclassic.wiki, cached for a week. */
  monsterDrops(monsters: string[]): Promise<MonsterDrops[]>;
  /** Where to go for each quest objective, from cached data only. Null until map data is downloaded. */
  questGuide(
    quests: QuestQuery[],
    mapName: string | null,
    mapStreet: string | null,
    measured: MeasuredRate[],
  ): Promise<Guide | null>;
  /** Asks mapleclassic.wiki which monsters drop these quest items; cached for a week. */
  lookUpQuestItems(items: string[]): Promise<void>;
  /** Every map with monsters to hunt, with the rates the player measured. Null until map data is downloaded. */
  trainingSpots(mapName: string | null, mapStreet: string | null, measured: MeasuredRate[]): Promise<TrainingSpots | null>;
  /** Opens one of the card's link-outs or credits in the browser. */
  openLink(url: string): Promise<void>;
  onMapData(handler: (status: MapDataStatus) => void): Promise<UnlistenFn>;

  /** The prices logged for an item, summarised for a copy with these upgrades and slots. */
  priceLog(item: string, upgrades: number | null, slots: number | null): Promise<PriceLog>;
  recordPrice(price: NewPrice): Promise<PriceLog>;
  deletePrice(id: number): Promise<void>;

  /** Saves the session the overlay shows (the last replay) to the history. */
  saveSession(label: string): Promise<number>;
  sessionHistory(): Promise<History>;
  deleteSession(id: number): Promise<void>;
  setSessionNote(id: number, note: string): Promise<void>;
  /** Corrects a misread map name, in what is saved and whenever it is read that way again. */
  renameMap(from: string, to: string): Promise<void>;
  /** Opens the folder that holds the player's records, settings and backups. */
  openDataFolder(): Promise<void>;
  /** How Leafline is doing, for a bug report. Holds no names, chat or file paths. */
  diagnostics(): Promise<string>;
  onHistory(handler: () => void): Promise<UnlistenFn>;
  /** Asks where to save, then writes the price log or session history as CSV. Null if cancelled. */
  exportCsv(what: "prices" | "sessions"): Promise<number | null>;

  /** The latest analysis and session, for a window opened after they were sent. */
  currentReadings(): Promise<{ analysis: Analysis | null; session: SessionSummary | null }>;
  pinStatus(): Promise<PinStatus>;
  /** Opens a panel in its own window that stays up while the overlay is hidden. */
  pinPanel(panel: Panel): Promise<PinStatus>;
  unpinPanel(panel: Panel): Promise<PinStatus>;
  onPins(handler: (status: PinStatus) => void): Promise<UnlistenFn>;
  settings(): Promise<DisplaySettings>;
  updateSettings(settings: DisplaySettings): Promise<DisplaySettings>;
  onSettings(handler: (settings: DisplaySettings) => void): Promise<UnlistenFn>;
  showSettings(): Promise<void>;
  /** Leafline's version, for the About section. */
  version(): Promise<string>;
  /** The travel connections routes can use, and which file they come from. */
  travelInfo(): Promise<TravelInfo>;
  /** Downloads the taxis and return scrolls players recorded on mapleclassic.wiki. */
  downloadTravel(): Promise<TravelInfo>;
  /** Opens the player's copy of the travel file in their editor, made from the downloaded one if missing. */
  editTravelFile(): Promise<void>;
  /** Sizes the calling window to its content. */
  fitWindow(width: number, height: number): Promise<void>;
}

const IMAGE_FILTER = { name: "Screenshots", extensions: ["png", "jpg", "jpeg", "webp"] };

const tauriBackend: Backend = {
  analyzeImage: (path) => invoke("analyze_image", { path }),
  calibrateHud: (path, values) => invoke("calibrate_hud", { path, values }),
  listImages: (folder) => invoke("list_images", { folder }),
  replaySession: (folder, order) => invoke("replay_session", { folder, order }),
  cancelReplay: () => invoke("cancel_replay"),
  async imageUrl(path) {
    const bytes = await invoke<ArrayBuffer>("read_image", { path });
    return URL.createObjectURL(new Blob([new Uint8Array(bytes)], { type: imageType(path) }));
  },
  async pickImage() {
    const picked = await open({ multiple: false, directory: false, filters: [IMAGE_FILTER] });
    return typeof picked === "string" ? picked : null;
  },
  async pickFolder() {
    const picked = await open({ multiple: false, directory: true });
    return typeof picked === "string" ? picked : null;
  },
  overlayStatus: () => invoke("overlay_status"),
  setClickThrough: (enabled) => invoke("set_click_through", { enabled }),
  hideOverlay: () => invoke("hide_overlay"),
  setCompact: (enabled) => invoke("set_compact", { enabled }),
  toggleOverlay: () => invoke("toggle_overlay"),
  showWorkbench: () => invoke("show_workbench"),
  shortcuts: () => invoke("shortcut_list"),
  liveStatus: () => invoke("live_status"),
  setLiveHeld: (held) => invoke("set_live_held", { held }),
  resetSession: () => invoke("reset_session"),
  setLiveLogging: (logging) => invoke("set_live_logging", { logging }),
  startLive: () => invoke("start_live"),
  stopLive: () => invoke("stop_live"),
  liveWindows: () => invoke("live_windows"),
  saveLiveFrame: () => invoke("save_live_frame"),
  onLive: (handler) => listen<LiveStatus>("live", (e) => handler(e.payload)),
  onFrameSaved: (handler) => listen<string>("frame-saved", (e) => handler(e.payload)),
  takeLaunchImage: () => invoke("take_launch_image"),
  onOpenImage: (handler) => listen<string>("open-image", (e) => handler(e.payload)),
  onAnalysis: (handler) => listen<Analysis>("analysis", (e) => handler(e.payload)),
  onReplayProgress: (handler) => listen<ReplayProgress>("replay-progress", (e) => handler(e.payload)),
  onSession: (handler) => listen<SessionSummary>("session", (e) => handler(e.payload)),
  onOverlayStatus: (handler) => listen<OverlayStatus>("overlay-status", (e) => handler(e.payload)),
  mapDataStatus: () => invoke("map_data_status"),
  downloadMapData: (force) => invoke("download_map_data", { force }),
  findMap: (name, street) => invoke("find_map", { name, street }),
  mapCard: (id) => invoke("map_card", { id }),
  searchMaps: (text) => invoke("search_maps", { text }),
  mapRoute: (from, to) => invoke("map_route", { from, to }),
  monsterDrops: (monsters) => invoke("monster_drops", { monsters }),
  questGuide: (quests, mapName, mapStreet, measured) => invoke("quest_guide", { quests, mapName, mapStreet, measured }),
  lookUpQuestItems: async (items) => {
    await invoke("look_up_quest_items", { items });
  },
  trainingSpots: (mapName, mapStreet, measured) => invoke("training_spots", { mapName, mapStreet, measured }),
  openLink: (url) => invoke("open_link", { url }),
  onMapData: (handler) => listen<MapDataStatus>("map-data", (e) => handler(e.payload)),
  priceLog: (item, upgrades, slots) => invoke("price_log", { item, upgrades, slots }),
  recordPrice: (price) => invoke("record_price", { price }),
  deletePrice: (id) => invoke("delete_price", { id }),
  saveSession: (label) => invoke("save_session", { label }),
  setSessionNote: (id, note) => invoke("set_session_note", { id, note }),
  renameMap: (from, to) => invoke("rename_map", { from, to }),
  openDataFolder: () => invoke("open_data_folder"),
  diagnostics: () => invoke("diagnostics"),
  sessionHistory: () => invoke("session_history"),
  deleteSession: (id) => invoke("delete_session", { id }),
  onHistory: (handler) => listen("history", () => handler()),
  async exportCsv(what) {
    const stamp = new Date().toISOString().slice(0, 10);
    const path = await save({
      defaultPath: `leafline-${what === "prices" ? "price-log" : "sessions"}-${stamp}.csv`,
      filters: [{ name: "CSV", extensions: ["csv"] }],
    });
    return path ? invoke<number>("export_csv", { what, path }) : null;
  },
  currentReadings: () => invoke("current_readings"),
  pinStatus: () => invoke("pin_status"),
  pinPanel: (panel) => invoke("pin_panel", { panel }),
  unpinPanel: (panel) => invoke("unpin_panel", { panel }),
  onPins: (handler) => listen<PinStatus>("pins", (e) => handler(e.payload)),
  settings: () => invoke("get_settings"),
  updateSettings: (settings) => invoke("update_settings", { settings }),
  onSettings: (handler) => listen<DisplaySettings>("settings", (e) => handler(e.payload)),
  showSettings: () => invoke("show_settings"),
  travelInfo: () => invoke("travel_info"),
  downloadTravel: () => invoke("download_travel"),
  editTravelFile: () => invoke("edit_travel_file"),
  version: () => getVersion(),
  fitWindow: (width, height) => getCurrentWindow().setSize(new LogicalSize(width, height)),
};

/** The mock is dev-only: production builds must never bundle it or the screenshots it loads. */
async function devBackend(): Promise<Backend | null> {
  if (!import.meta.env.DEV || isTauri()) return null;
  return (await import("./dev/mock")).mockBackend;
}

export const backend: Backend = (await devBackend()) ?? tauriBackend;

function imageType(path: string): string {
  const extension = path.split(".").pop()?.toLowerCase();
  return extension === "jpg" ? "image/jpeg" : `image/${extension}`;
}

/** File name without its folder, for display. */
export function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}
