// Typed access to the Rust commands and events. In a plain browser (UI development with
// `npm run dev`) a mock backend stands in, so the pages can be designed without Tauri.

import { getVersion } from "@tauri-apps/api/app";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { LogicalSize } from "@tauri-apps/api/dpi";
import { emit, listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { availableMonitors, getCurrentWindow } from "@tauri-apps/api/window";
import { open, save } from "@tauri-apps/plugin-dialog";

import type {
  Analysis,
  Build,
  BuildCatalog,
  BuildPlan,
  BuildSummary,
  CharacterRead,
  DisplaySettings,
  GameWindow,
  GearFor,
  GearFound,
  GearItem,
  GearPiece,
  GearSuggestions,
  ItemFilters,
  ItemLink,
  ItemPage,
  ItemQuery,
  ItemRecord,
  ItemWhere,
  ItemFit,
  ItemRead,
  ItemTooltip,
  LiveStatus,
  TravelInfo,
  Guide,
  History,
  KnownMesos,
  LoggedDrop,
  NewDrop,
  MapDataDownload,
  MapDataStatus,
  MapDetails,
  MapMatch,
  MapPage,
  MapQuery,
  MapRef,
  MapRoute,
  NavState,
  Destination,
  MeasuredRate,
  Assumptions,
  SavedSpot,
  PotionSlot,
  SessionPotion,
  LevelExp,
  ItemCrafting,
  CraftingPage,
  CraftingQuery,
  UpdateStatus,
  ItemNote,
  MonsterDrops,
  MonsterFilters,
  MonsterLink,
  MonsterPage,
  MonsterQuery,
  MonsterWhere,
  NpcPage,
  NpcQuery,
  NpcWhere,
  OverlayStatus,
  Outfit,
  Panel,
  PinId,
  PinStatus,
  DeskStatus,
  Trip,
  TripMessage,
  PlaceLink,
  QuestPage,
  QuestQuery,
  QuestSearch,
  QuestWhere,
  SessionSummary,
  SessionTimeline,
  MapNote,
  ShortcutInfo,
  StarterPlan,
  TrainingSpots,
} from "./types";

export interface Backend {
  pickFolder(): Promise<string | null>;
  overlayStatus(): Promise<OverlayStatus>;
  setClickThrough(enabled: boolean): Promise<void>;
  hideOverlay(): Promise<void>;
  /** Switches the overlay between the full panels and the compact HUD. */
  setCompact(enabled: boolean): Promise<void>;
  toggleOverlay(): Promise<void>;
  shortcuts(): Promise<ShortcutInfo[]>;
  /** While true, a hotkey press only reports itself and runs nothing; it lapses after 30 s unless renewed. */
  setHotkeysQuiet(quiet: boolean): Promise<void>;
  /** Each time a hotkey arrives, with its action's name. */
  onHotkey(handler: (action: string) => void): Promise<UnlistenFn>;
  liveStatus(): Promise<LiveStatus>;
  /** Pauses or resumes tracking; paused time doesn't count. */
  setLiveHeld(held: boolean): Promise<void>;
  /** Starts a new session. The one before stays in the history. */
  resetSession(): Promise<void>;
  /** Keeps this run in or out of the history. */
  /** Starts reading the game window; a new session starts with it. */
  startLive(): Promise<void>;
  /** Stops after the reading in progress; the session stays, ready to save. */
  stopLive(): Promise<void>;
  /** Windows that could be the game, likely ones first. */
  liveWindows(): Promise<GameWindow[]>;
  /** Saves the last live picture to the saved pictures folder and returns its path. */
  saveLiveFrame(): Promise<string>;
  onLive(handler: (status: LiveStatus) => void): Promise<UnlistenFn>;
  onFrameSaved(handler: (path: string) => void): Promise<UnlistenFn>;
  /** An item check from the hotkey has ended. */
  onItemRead(handler: (read: ItemRead) => void): Promise<UnlistenFn>;
  onAnalysis(handler: (analysis: Analysis) => void): Promise<UnlistenFn>;
  onSession(handler: (session: SessionSummary) => void): Promise<UnlistenFn>;
  onOverlayStatus(handler: (status: OverlayStatus) => void): Promise<UnlistenFn>;

  /** What map data is cached. */
  mapDataStatus(): Promise<MapDataStatus>;
  /** The explicit "Download map data" action. Without `force`, skipped if checked in the last day. */
  downloadMapData(force: boolean): Promise<MapDataDownload>;
  /** The card for an OCR'd minimap header, or null without a close enough match or any data. */
  findMap(name: string, street: string | null, twin: number | null): Promise<MapMatch | null>;
  mapCard(id: number): Promise<MapDetails | null>;
  searchMaps(text: string): Promise<MapRef[]>;
  mapRoute(from: number, to: number): Promise<MapRoute | null>;
  /** Where "Go here" is heading, and the places headed for lately. */
  destination(): Promise<NavState>;
  /** Heads for a place from any window, or with null, cancels. */
  setDestination(destination: Destination | null): Promise<NavState>;
  /** Clears the destination on arrival, unless another window has set a new one since. */
  arriveAt(destination: Destination): Promise<NavState>;
  /** Takes a place off the recent list. */
  forgetDestination(destination: Destination): Promise<NavState>;
  onDestination(handler: (nav: NavState) => void): Promise<UnlistenFn>;
  /** Player-observed drops, from a copy of mapleclassic.wiki's table renewed weekly. */
  monsterDrops(monsters: string[]): Promise<MonsterDrops[]>;
  /** Where to go for each quest objective, from cached data only. Null until map data is downloaded. */
  questGuide(
    quests: QuestQuery[],
    mapName: string | null,
    mapStreet: string | null,
    mapTwin: number | null,
    measured: MeasuredRate[],
  ): Promise<Guide | null>;
  /** Downloads mapleclassic.wiki's whole drop table, so quest items can be looked up on this PC. */
  downloadDrops(): Promise<void>;
  /** When the drop table was last downloaded, in Unix seconds, or null before the first time. */
  dropsFetchedAt(): Promise<number | null>;
  /** Every map with monsters to hunt, with the rates the player measured. Null until map data is downloaded. */
  trainingSpots(
    mapName: string | null,
    mapStreet: string | null,
    mapTwin: number | null,
    measured: MeasuredRate[],
    level: number | null,
    assumptions: Assumptions,
  ): Promise<TrainingSpots | null>;
  /** One page of the quest search. */
  searchQuests(search: QuestSearch): Promise<QuestPage>;
  questRegions(): Promise<string[]>;
  /** All the data says about a quest. */
  questSheet(id: number, mapName: string | null, mapStreet: string | null, mapTwin: number | null): Promise<QuestWhere | null>;
  /** Brings a quest up in the Quests tab from any window. */
  showQuest(id: number): Promise<void>;
  onShowQuest(handler: (id: number) => void): Promise<UnlistenFn>;
  /** One page of the monster search. */
  searchMonsters(query: MonsterQuery): Promise<MonsterPage>;
  monsterFilters(): Promise<MonsterFilters>;
  /** The monster a name means. */
  findMonster(name: string): Promise<number | null>;
  /** All the data says about a monster. Downloads the drop table the first time. */
  monsterSheet(
    id: number,
    mapName: string | null,
    mapStreet: string | null,
    mapTwin: number | null,
    measured: MeasuredRate[],
  ): Promise<MonsterWhere | null>;
  /** Brings a monster, or a search for `text`, up in the Lookup tab from any window. */
  showMonster(id: number | null, text: string | null): Promise<void>;
  onShowMonster(handler: (link: MonsterLink) => void): Promise<UnlistenFn>;
  /** One page of the map search. */
  browseMaps(query: MapQuery): Promise<MapPage>;
  /** The regions maps are filed under. */
  mapRegions(): Promise<string[]>;
  /** The areas Lookup narrows maps, monsters and NPCs to, Maple Island first. */
  mapAreas(): Promise<string[]>;
  /** One page of the NPC search. */
  searchNpcs(query: NpcQuery): Promise<NpcPage>;
  /** The NPC a name means. */
  findNpc(name: string): Promise<number | null>;
  /** Where an NPC stands and the quests they give. */
  npcSheet(id: number, mapName: string | null, mapStreet: string | null, mapTwin: number | null): Promise<NpcWhere | null>;
  /** Brings a map or an NPC up in the Lookup tab from any window. */
  showPlace(mode: PlaceLink["mode"], id: number): Promise<void>;
  onShowPlace(handler: (link: PlaceLink) => void): Promise<UnlistenFn>;
  /** Opens one of the card's link-outs or credits in the browser. */
  openLink(url: string): Promise<void>;
  onMapData(handler: (status: MapDataStatus) => void): Promise<UnlistenFn>;

  /** The training spots the player saved, the newest first. */
  savedSpots(): Promise<SavedSpot[]>;
  /** Saves a map as a training spot, or takes it off the list. Every window hears of it. */
  setSpotSaved(mapId: number, name: string, saved: boolean): Promise<SavedSpot[]>;
  onSavedSpots(handler: (spots: SavedSpot[]) => void): Promise<UnlistenFn>;
  /** The potion slots the player set up, in slot order. */
  potionSlots(): Promise<PotionSlot[]>;
  /** Replaces the potion slots; a new slot has number 0. Returns them numbered. */
  setPotionSlots(slots: PotionSlot[]): Promise<PotionSlot[]>;
  /** The potion counts typed for the session that started at `startedAt`. */
  sessionPotions(startedAt: number): Promise<SessionPotion[]>;
  /** How many of a slot's potion the player had at a session's start and has now. */
  setSessionPotion(startedAt: number, slot: number, start: number | null, now: number | null): Promise<SessionPotion[]>;
  /** Fires when the slots or any session's counts change. */
  onPotions(handler: () => void): Promise<UnlistenFn>;
  /** The player's note on an item, matched like the price log. */
  itemNote(item: string): Promise<ItemNote | null>;
  /** Writes it; an empty note deletes it. */
  setItemNote(item: string, note: string): Promise<ItemNote | null>;
  /** The player's note on a map, if any. */
  mapNote(mapId: number): Promise<MapNote | null>;
  /** Writes it; an empty note deletes it. */
  setMapNote(mapId: number, map: string, note: string): Promise<MapNote | null>;
  /** Follows map notes written in any window, with the map's id. */
  onMapNote(handler: (mapId: number) => void): Promise<UnlistenFn>;
  /** Every drop the player logged, the newest first. Kept on this PC. */
  dropLog(): Promise<LoggedDrop[]>;
  /** Logs a drop; returns its id. Every window hears of it. */
  logDrop(drop: NewDrop): Promise<number>;
  deleteDrop(id: number): Promise<void>;
  onDrops(handler: () => void): Promise<UnlistenFn>;
  /** Asks where to save, then writes a PNG the app drew. False if cancelled. */
  saveImage(png: Uint8Array, name: string): Promise<boolean>;

  /** The mesos last read for a character, or with no name, for the character seen last. */
  knownMesos(name: string | null): Promise<KnownMesos | null>;
  onKnownMesos(handler: () => void): Promise<UnlistenFn>;

  sessionHistory(): Promise<History>;
  /** The EXP each level takes, as far as the player's HUD has shown it, lowest first. */
  levelExp(): Promise<LevelExp[]>;
  /** The recipes that make an item and the ones that use it; null without map data. */
  itemCrafting(id: number): Promise<ItemCrafting | null>;
  /** The Crafting section's recipes. */
  craftingRecipes(query: CraftingQuery): Promise<CraftingPage>;
  /** How a saved session's rates moved over time. */
  sessionTimeline(id: number): Promise<SessionTimeline>;
  /** Opens the training log window, or brings it forward. */
  showTraining(): Promise<void>;
  /** Opens the desk view, the panels side by side in a window of its own, or brings it forward. */
  showDesk(): Promise<DeskStatus>;
  deskStatus(): Promise<DeskStatus>;
  setDeskOnTop(onTop: boolean): Promise<DeskStatus>;
  /** Draws the calling window's page larger or smaller, as browser zoom does. */
  setPageZoom(zoom: number): Promise<void>;
  /** How many screens Windows has, for Setup's choice of overlay or desk view. */
  monitorCount(): Promise<number>;
  /** Lets this window take focus while a box in it is typed in, or gives the focus back up. */
  setTyping(typing: boolean): Promise<void>;
  deleteSession(id: number): Promise<void>;
  setSessionNote(id: number, note: string): Promise<void>;
  /** Corrects a misread map name, in what is saved and whenever it is read that way again. */
  renameMap(from: string, to: string): Promise<void>;
  /** Opens the folder that holds the player's records, settings and backups. */
  openDataFolder(): Promise<void>;
  /** The folder saved game pictures go to. */
  framesFolder(): Promise<string>;
  openFramesFolder(): Promise<void>;
  /** How Leafline is doing, for a bug report. Holds no names, chat or file paths. */
  diagnostics(): Promise<string>;
  onHistory(handler: () => void): Promise<UnlistenFn>;
  /** Asks where to save, then writes the price log or session history as CSV. Null if cancelled. */
  exportCsv(what: "sessions" | "drops"): Promise<number | null>;

  /** The jobs and skills to plan with. Null until map data is downloaded. */
  buildCatalog(): Promise<BuildCatalog | null>;
  /** Equipment by name, limited to what the job can use. */
  searchGear(text: string, job: number): Promise<GearItem[]>;
  /** One page of the item search. */
  searchItems(query: ItemQuery, level: number | null, character: string | null): Promise<ItemPage>;
  /** The slots and weapon types the item search offers. */
  itemFilters(): Promise<ItemFilters>;
  itemRecord(id: number): Promise<ItemRecord | null>;
  /** The item a name read from the game most likely means. */
  findItem(name: string, level: number | null): Promise<ItemRecord | null>;
  /** Who drops an item and which quests give it. Downloads the drop table the first time. */
  itemOrigin(id: number, mapName: string | null, mapStreet: string | null, mapTwin: number | null): Promise<ItemWhere | null>;
  /** Brings an item, or a search for `text`, up in the Lookup tab from any window. */
  showItem(id: number | null, text: string | null): Promise<void>;
  onShowItem(handler: (link: ItemLink) => void): Promise<UnlistenFn>;
  /** Adds an item to the gear goals of the build for this character, or takes it off. */
  setGearGoal(item: number, wanted: boolean, character: string | null): Promise<void>;
  /** Adds a piece of equipment, as an item check read it, to the build for this character. */
  addPiece(piece: GearPiece, character: string | null): Promise<void>;
  removePiece(id: number, character: string | null): Promise<void>;
  /** Puts right a piece the item check misread, found by its id. */
  updatePiece(piece: GearPiece, character: string | null): Promise<void>;
  /** Puts on a clean copy of an item from the data, in place of what was in its slot. */
  wearItem(item: number, character: string | null): Promise<void>;
  buildOutfit(build: number, level: number): Promise<Outfit | null>;
  /** Fills each slot with no goals with an upgrade path from `level`; returns how many were added. */
  fillGear(build: number, level: number): Promise<number>;
  /** Reads the open stat and skill windows once and moves the build to match. */
  readCharacter(): Promise<void>;
  onCharacterRead(handler: (read: CharacterRead) => void): Promise<UnlistenFn>;
  /** Gear for a character. `seenJob` is the job the HUD shows. Null until map data is downloaded. */
  findGear(who: GearFor, character: string | null, seenJob: string | null): Promise<GearFound | null>;
  /** Gear to look for at this level on this build. */
  gearSuggestions(build: number, level: number): Promise<GearSuggestions | null>;
  /** The plans written for Leafline, one for each second job. Empty until map data is downloaded. */
  starterPlans(): Promise<StarterPlan[]>;
  listBuilds(): Promise<BuildSummary[]>;
  buildPlan(id: number): Promise<BuildPlan | null>;
  /** The build to show for this character, or the one used last. */
  activeBuild(character: string | null): Promise<BuildPlan | null>;
  /** Saves a build, new or changed, and returns how it plays out. */
  saveBuild(build: Build): Promise<BuildPlan>;
  /** Makes a build the one shown when no other is tied to the character. */
  useBuild(id: number): Promise<void>;
  deleteBuild(id: number): Promise<void>;
  /** The build's plan as a line of text, for the player to pass on themselves. */
  exportBuild(id: number): Promise<string>;
  /** Adds a build from a code. */
  importBuild(code: string, character: string | null): Promise<BuildPlan>;
  /** How a hovered item suits the build for this character. Null with no build to judge by. */
  buildItemFit(item: ItemTooltip, level: number | null, character: string | null): Promise<ItemFit | null>;
  /** Opens the guide link the player typed for a build. */
  openGuide(url: string): Promise<void>;
  /** Opens the build editor, or brings it forward. */
  showBuild(): Promise<void>;
  onBuilds(handler: () => void): Promise<UnlistenFn>;

  /** The latest analysis and session, for a window opened after they were sent. */
  currentReadings(): Promise<{ analysis: Analysis | null; session: SessionSummary | null }>;
  pinStatus(): Promise<PinStatus>;
  /** Opens a panel in its own window that stays up while the overlay is hidden. */
  pinPanel(panel: PinId): Promise<PinStatus>;
  unpinPanel(panel: PinId): Promise<PinStatus>;
  onPins(handler: (status: PinStatus) => void): Promise<UnlistenFn>;
  /** The player resized the calling pinned panel, with its new width and height in screen units. */
  onPinResized(handler: (size: [number, number]) => void): Promise<UnlistenFn>;
  /** Brings the overlay up at a panel's tab, from a pinned panel. The route's is the Map tab. */
  showPanel(panel: PinId): Promise<void>;
  onShowPanel(handler: (panel: Panel) => void): Promise<UnlistenFn>;
  /** The overlay, which follows the route, tells the Route pin how it stands. */
  sendTrip(trip: Trip): Promise<void>;
  onTrip(handler: (trip: Trip) => void): Promise<UnlistenFn>;
  /** The Route pin asks the overlay for the trip, or drops its turn-in offer. */
  tripMessage(message: TripMessage): Promise<void>;
  onTripMessage(handler: (message: TripMessage) => void): Promise<UnlistenFn>;
  settings(): Promise<DisplaySettings>;
  updateSettings(settings: DisplaySettings): Promise<DisplaySettings>;
  onSettings(handler: (settings: DisplaySettings) => void): Promise<UnlistenFn>;
  showSettings(): Promise<void>;
  /** Opens the first-launch setup again. */
  showSetup(): Promise<void>;
  /** Closes setup and starts live reading if it is on. Save `setupDone` first. */
  finishSetup(): Promise<void>;
  /** Leafline's version, for the About section. */
  version(): Promise<string>;
  /** Where the optional update check stands. */
  updateStatus(): Promise<UpdateStatus>;
  /** Checks now; refused while the check is turned off. */
  checkForUpdate(): Promise<UpdateStatus>;
  onUpdateStatus(handler: (status: UpdateStatus) => void): Promise<UnlistenFn>;
  /** The travel connections routes can use, and which file they come from. */
  travelInfo(): Promise<TravelInfo>;
  /** Downloads the taxis and return scrolls players recorded on mapleclassic.wiki. */
  downloadTravel(): Promise<TravelInfo>;
  /** Opens the player's copy of the travel file in their editor, made from the downloaded one if missing. */
  editTravelFile(): Promise<void>;
  /** Sizes the calling window to its content. */
  fitWindow(width: number, height: number): Promise<void>;
}

const tauriBackend: Backend = {
  async pickFolder() {
    const picked = await open({ multiple: false, directory: true });
    return typeof picked === "string" ? picked : null;
  },
  overlayStatus: () => invoke("overlay_status"),
  setClickThrough: (enabled) => invoke("set_click_through", { enabled }),
  hideOverlay: () => invoke("hide_overlay"),
  setCompact: (enabled) => invoke("set_compact", { enabled }),
  toggleOverlay: () => invoke("toggle_overlay"),
  shortcuts: () => invoke("shortcut_list"),
  setHotkeysQuiet: (quiet) => invoke("set_hotkeys_quiet", { quiet }),
  onHotkey: (handler) => listen<string>("hotkey", (e) => handler(e.payload)),
  liveStatus: () => invoke("live_status"),
  setLiveHeld: (held) => invoke("set_live_held", { held }),
  resetSession: () => invoke("reset_session"),
  startLive: () => invoke("start_live"),
  stopLive: () => invoke("stop_live"),
  liveWindows: () => invoke("live_windows"),
  saveLiveFrame: () => invoke("save_live_frame"),
  onLive: (handler) => listen<LiveStatus>("live", (e) => handler(e.payload)),
  onFrameSaved: (handler) => listen<string>("frame-saved", (e) => handler(e.payload)),
  onItemRead: (handler) => listen<ItemRead>("item-read", (e) => handler(e.payload)),
  onAnalysis: (handler) => listen<Analysis>("analysis", (e) => handler(e.payload)),
  onSession: (handler) => listen<SessionSummary>("session", (e) => handler(e.payload)),
  onOverlayStatus: (handler) => listen<OverlayStatus>("overlay-status", (e) => handler(e.payload)),
  mapDataStatus: () => invoke("map_data_status"),
  downloadMapData: (force) => invoke("download_map_data", { force }),
  findMap: (name, street, twin) => invoke("find_map", { name, street, twin }),
  mapCard: (id) => invoke("map_card", { id }),
  searchMaps: (text) => invoke("search_maps", { text }),
  mapRoute: (from, to) => invoke("map_route", { from, to }),
  destination: () => invoke("destination"),
  setDestination: (destination) => invoke("set_destination", { destination }),
  arriveAt: (destination) => invoke("arrive_at", { destination }),
  forgetDestination: (destination) => invoke("forget_destination", { destination }),
  onDestination: (handler) => listen<NavState>("destination", (e) => handler(e.payload)),
  monsterDrops: (monsters) => invoke("monster_drops", { monsters }),
  questGuide: (quests, mapName, mapStreet, mapTwin, measured) =>
    invoke("quest_guide", { quests, mapName, mapStreet, mapTwin, measured }),
  downloadDrops: () => invoke("download_drops"),
  dropsFetchedAt: () => invoke("drops_fetched_at"),
  trainingSpots: (mapName, mapStreet, mapTwin, measured, level, assumptions) =>
    invoke("training_spots", { mapName, mapStreet, mapTwin, measured, level, assumptions }),
  searchQuests: (search) => invoke("search_quests", { search }),
  questRegions: () => invoke("quest_regions"),
  questSheet: (id, mapName, mapStreet, mapTwin) => invoke("quest_sheet", { id, mapName, mapStreet, mapTwin }),
  showQuest: (id) => invoke("show_quest", { id }),
  onShowQuest: (handler) => listen<number>("show-quest", (e) => handler(e.payload)),
  searchMonsters: (query) => invoke("search_monsters", { query }),
  monsterFilters: () => invoke("monster_filters"),
  findMonster: (name) => invoke("find_monster", { name }),
  monsterSheet: (id, mapName, mapStreet, mapTwin, measured) =>
    invoke("monster_sheet", { id, mapName, mapStreet, mapTwin, measured }),
  showMonster: (id, text) => invoke("show_monster", { id, text }),
  onShowMonster: (handler) => listen<MonsterLink>("show-monster", (e) => handler(e.payload)),
  browseMaps: (query) => invoke("browse_maps", { query }),
  mapRegions: () => invoke("map_regions"),
  mapAreas: () => invoke("map_areas"),
  searchNpcs: (query) => invoke("search_npcs", { query }),
  findNpc: (name) => invoke("find_npc", { name }),
  npcSheet: (id, mapName, mapStreet, mapTwin) => invoke("npc_sheet", { id, mapName, mapStreet, mapTwin }),
  showPlace: (mode, id) => invoke("show_place", { mode, id }),
  onShowPlace: (handler) => listen<PlaceLink>("show-place", (e) => handler(e.payload)),
  openLink: (url) => invoke("open_link", { url }),
  onMapData: (handler) => listen<MapDataStatus>("map-data", (e) => handler(e.payload)),
  savedSpots: () => invoke("saved_spots"),
  setSpotSaved: (mapId, name, saved) => invoke("set_spot_saved", { mapId, name, saved }),
  onSavedSpots: (handler) => listen<SavedSpot[]>("saved-spots", (e) => handler(e.payload)),
  potionSlots: () => invoke("potion_slots"),
  setPotionSlots: (slots) => invoke("set_potion_slots", { slots }),
  sessionPotions: (startedAt) => invoke("session_potions", { startedAt }),
  setSessionPotion: (startedAt, slot, start, now) => invoke("set_session_potion", { startedAt, slot, start, now }),
  onPotions: (handler) => listen("potions", () => handler()),
  itemNote: (item) => invoke("item_note", { item }),
  setItemNote: (item, note) => invoke("set_item_note", { item, note }),
  mapNote: (mapId) => invoke("map_note", { mapId }),
  setMapNote: (mapId, map, note) => invoke("set_map_note", { mapId, map, note }),
  onMapNote: (handler) => listen<number>("map-note", (e) => handler(e.payload)),
  dropLog: () => invoke("drop_log"),
  logDrop: (drop) => invoke("log_drop", { drop }),
  deleteDrop: (id) => invoke("delete_drop", { id }),
  onDrops: (handler) => listen("drops", () => handler()),
  async saveImage(png, name) {
    const path = await save({ defaultPath: name, filters: [{ name: "PNG picture", extensions: ["png"] }] });
    if (!path) return false;
    await invoke("save_image", { path, png: Array.from(png) });
    return true;
  },
  knownMesos: (name) => invoke("known_mesos", { name }),
  onKnownMesos: (handler) => listen("known-mesos", () => handler()),
  setSessionNote: (id, note) => invoke("set_session_note", { id, note }),
  renameMap: (from, to) => invoke("rename_map", { from, to }),
  openDataFolder: () => invoke("open_data_folder"),
  framesFolder: () => invoke("frames_folder"),
  openFramesFolder: () => invoke("open_frames_folder"),
  diagnostics: () => invoke("diagnostics"),
  sessionHistory: () => invoke("session_history"),
  levelExp: () => invoke("level_exp"),
  itemCrafting: (id) => invoke("item_crafting", { id }),
  craftingRecipes: (query) => invoke("crafting_recipes", { query }),
  sessionTimeline: (id) => invoke("session_timeline", { id }),
  showTraining: () => invoke("show_training"),
  showDesk: () => invoke("show_desk"),
  deskStatus: () => invoke("desk_status"),
  setDeskOnTop: (onTop) => invoke("set_desk_on_top", { onTop }),
  setPageZoom: (zoom) => getCurrentWebview().setZoom(zoom),
  monitorCount: () => availableMonitors().then((list) => list.length),
  setTyping: (typing) => invoke("set_typing", { typing }),
  deleteSession: (id) => invoke("delete_session", { id }),
  onHistory: (handler) => listen("history", () => handler()),
  async exportCsv(what) {
    const stamp = new Date().toISOString().slice(0, 10);
    const path = await save({
      defaultPath: `leafline-${what === "drops" ? "drop-log" : "sessions"}-${stamp}.csv`,
      filters: [{ name: "CSV", extensions: ["csv"] }],
    });
    return path ? invoke<number>("export_csv", { what, path }) : null;
  },
  buildCatalog: () => invoke("build_catalog"),
  searchGear: (text, job) => invoke("search_gear", { text, job }),
  searchItems: (query, level, character) => invoke("search_items", { query, level, character }),
  itemFilters: () => invoke("item_filters"),
  itemRecord: (id) => invoke("item_record", { id }),
  findItem: (name, level) => invoke("find_item", { name, level }),
  itemOrigin: (id, mapName, mapStreet, mapTwin) => invoke("item_origin", { id, mapName, mapStreet, mapTwin }),
  showItem: (id, text) => invoke("show_item", { id, text }),
  onShowItem: (handler) => listen<ItemLink>("show-item", (e) => handler(e.payload)),
  setGearGoal: (item, wanted, character) => invoke("set_gear_goal", { item, wanted, character }),
  addPiece: (piece, character) => invoke("add_piece", { piece, character }),
  removePiece: (id, character) => invoke("remove_piece", { id, character }),
  updatePiece: (piece, character) => invoke("update_piece", { piece, character }),
  wearItem: (item, character) => invoke("wear_item", { item, character }),
  buildOutfit: (build, level) => invoke("build_outfit", { id: build, level }),
  fillGear: (build, level) => invoke("fill_gear", { id: build, level }),
  gearSuggestions: (build, level) => invoke("gear_suggestions", { id: build, level }),
  findGear: (who, character, seenJob) => invoke("find_gear", { who, character, seenJob }),
  readCharacter: () => invoke("read_character_windows"),
  onCharacterRead: (handler) => listen<CharacterRead>("character-read", (e) => handler(e.payload)),
  starterPlans: () => invoke("starter_plans"),
  listBuilds: () => invoke("list_builds"),
  buildPlan: (id) => invoke("build_plan", { id }),
  activeBuild: (character) => invoke("active_build", { character }),
  saveBuild: (build) => invoke("save_build", { build }),
  useBuild: (id) => invoke("use_build", { id }),
  deleteBuild: (id) => invoke("delete_build", { id }),
  exportBuild: (id) => invoke("export_build", { id }),
  importBuild: (code, character) => invoke("import_build", { code, character }),
  buildItemFit: (item, level, character) =>
    invoke("build_item_fit", { requirements: item.requirements, jobs: item.jobs, itemType: item.itemType, level, character }),
  openGuide: (url) => invoke("open_guide", { url }),
  showBuild: () => invoke("show_build"),
  onBuilds: (handler) => listen("builds", () => handler()),
  currentReadings: () => invoke("current_readings"),
  pinStatus: () => invoke("pin_status"),
  pinPanel: (panel) => invoke("pin_panel", { panel }),
  unpinPanel: (panel) => invoke("unpin_panel", { panel }),
  onPins: (handler) => listen<PinStatus>("pins", (e) => handler(e.payload)),
  onPinResized: (handler) => getCurrentWindow().listen<[number, number]>("pin-resized", (e) => handler(e.payload)),
  showPanel: (panel) => invoke("show_panel", { panel }),
  onShowPanel: (handler) => listen<Panel>("show-panel", (e) => handler(e.payload)),
  // Window to window, without the backend: only the overlay sends trips, only pins send messages.
  sendTrip: (trip) => emit("trip", trip),
  onTrip: (handler) => listen<Trip>("trip", (e) => handler(e.payload)),
  tripMessage: (message) => emit("trip-message", message),
  onTripMessage: (handler) => listen<TripMessage>("trip-message", (e) => handler(e.payload)),
  settings: () => invoke("get_settings"),
  updateSettings: (settings) => invoke("update_settings", { settings }),
  onSettings: (handler) => listen<DisplaySettings>("settings", (e) => handler(e.payload)),
  updateStatus: () => invoke("update_status"),
  checkForUpdate: () => invoke("check_for_update"),
  onUpdateStatus: (handler) => listen<UpdateStatus>("update", (e) => handler(e.payload)),
  showSettings: () => invoke("show_settings"),
  showSetup: () => invoke("show_setup"),
  finishSetup: () => invoke("finish_setup"),
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

/** File name without its folder, for display. */
export function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}
