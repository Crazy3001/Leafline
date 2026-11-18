// Typed access to the Rust commands and events. The program calls them through Tauri; the
// website runs the same Rust in a worker (see platform/). In a plain browser during UI
// development (`npm run dev`) a mock backend stands in, so the pages can be designed without either.

import type { UnlistenFn } from "@tauri-apps/api/event";

import { pickBackend } from "./platform";

import type {
  MonsterFight,
  Analysis,
  BagCheck,
  EstimateCheck,
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
  MapPicture,
  MapMatch,
  MapPage,
  MapQuery,
  MapRef,
  MapRoute,
  NavState,
  Destination,
  MeasuredRate,
  Assumptions,
  PlayerDamage,
  StatsView,
  SavedSpot,
  PotionSlot,
  LootPrice,
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
  QuestLog,
  RegionTotal,
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
  RecordsSummary,
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
  /** Saves the last item check's picture, with `reading` (what was read) beside it. */
  saveItemPicture(reading: string | null): Promise<string>;
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
  /** A map's picture cut down with its portals, NPCs and spawns placed on it; null when pictures are off or missing. */
  mapPicture(id: number): Promise<MapPicture | null>;
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
  /** Known drops, from a copy of Leafline's drop list renewed daily. */
  monsterDrops(monsters: string[]): Promise<MonsterDrops[]>;
  /** Where to go for each quest objective, from cached data only. Null until map data is downloaded. */
  questGuide(
    quests: QuestQuery[],
    mapName: string | null,
    mapStreet: string | null,
    mapTwin: number | null,
    measured: MeasuredRate[],
  ): Promise<Guide | null>;
  /** Downloads Leafline's drop list, so quest items can be looked up on this PC. */
  downloadDrops(retry?: boolean): Promise<void>;
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
    /** Whose read stats the combat model uses, and the skill picked (empty for the best read). */
    character: string | null,
    skill: string | null,
  ): Promise<TrainingSpots | null>;
  /** How a fight with one monster goes for the character, from the combat model. Null until their attack is known. */
  monsterFight(
    id: number,
    character: string | null,
    level: number | null,
    measured: MeasuredRate[],
    assumptions: Assumptions,
    skill: string | null,
  ): Promise<MonsterFight | null>;
  /** How close the estimates come on the player's own maps, each hidden in turn. */
  checkEstimates(
    measured: MeasuredRate[],
    level: number | null,
    assumptions: Assumptions,
    character: string | null,
    skill: string | null,
  ): Promise<EstimateCheck | null>;
  /** One page of the quest search. */
  searchQuests(search: QuestSearch): Promise<QuestPage>;
  questRegions(): Promise<string[]>;
  /** A character's quest log as the game's Quest window last showed it, or the last one read's. */
  questLog(name: string | null): Promise<QuestLog>;
  forgetQuestLog(name: string): Promise<void>;
  /** How many quests the data files under each region. */
  questRegionTotals(): Promise<RegionTotal[]>;
  /** A character's quest log changed; with their name. */
  onQuestLog(handler: (name: string) => void): Promise<UnlistenFn>;
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
  /** What was last read of a character's attack range and damaging skills. */
  playerDamage(name: string): Promise<PlayerDamage | null>;
  onPlayerDamage(handler: (change: { name: string; damage: PlayerDamage }) => void): Promise<UnlistenFn>;
  /** The character's stats as last read, their buffs, and the stats with buffs on. */
  characterStats(name: string): Promise<StatsView>;
  /** Marks one of the character's buffs as on or off. */
  setBuff(name: string, skill: number, on: boolean): Promise<void>;
  /** A character's potion slots, in slot order; the shared ones until it sets its own. */
  potionSlots(character: string | null): Promise<PotionSlot[]>;
  /** What one of each item is worth: the player's own logged price, else the NPC price. */
  lootPrices(ids: number[]): Promise<LootPrice[]>;
  /** Replaces a character's potion slots; a new slot has number 0. Returns them numbered. */
  setPotionSlots(character: string | null, slots: PotionSlot[]): Promise<PotionSlot[]>;
  /** The potion counts typed for the session that started at `startedAt`. */
  sessionPotions(startedAt: number): Promise<SessionPotion[]>;
  /** How many of a slot's potion the player had at a session's start and has now. */
  setSessionPotion(
    character: string | null,
    startedAt: number,
    slot: number,
    start: number | null,
    now: number | null,
  ): Promise<SessionPotion[]>;
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
  /** The lowest NPC shop price of each item a shop sells, as [id, price]; the rest are left out. */
  shopPrices(ids: number[]): Promise<[number, number][]>;
  /** How a saved session's rates moved over time. */
  sessionTimeline(id: number): Promise<SessionTimeline>;
  /** Opens the desk view on its Training log tab, or brings it forward there. */
  showTraining(): Promise<void>;
  /** Opens the desk view, the panels side by side in a window of its own, or brings it forward. */
  showDesk(): Promise<DeskStatus>;
  deskStatus(): Promise<DeskStatus>;
  /** The tab ("log" or "planner") the desk view was opened for, once; null otherwise. */
  deskTakeTab(): Promise<string | null>;
  /** The desk view is already open and is asked for one of its tabs. */
  onDeskTab(handler: (tab: string) => void): Promise<() => void>;
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
  exportCsv(what: "sessions" | "drops" | "loot"): Promise<number | null>;

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
  /** Sell or keep: tries every item of the tab last checked, downloading their pictures. */
  bagLookFurther(): Promise<void>;
  /** The last bag check, if any. */
  lastBag(): Promise<BagCheck | null>;
  onBag(handler: (check: BagCheck) => void): Promise<UnlistenFn>;
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
  /** Opens the desk view on its Build planner tab, or brings it forward there. */
  showBuild(): Promise<void>;
  onBuilds(handler: () => void): Promise<UnlistenFn>;

  /** The latest analysis and session, for a window opened after they were sent. */
  currentReadings(): Promise<{ analysis: Analysis | null; session: SessionSummary | null }>;
  pinStatus(): Promise<PinStatus>;
  /** Opens a panel in its own window that stays up while the overlay is hidden. */
  pinPanel(panel: PinId): Promise<PinStatus>;
  /** Opens a link card in a small window of its own at a screen position (logical pixels). */
  popOutCard(path: string, x: number, y: number): Promise<void>;
  /** Shows a card in the live card window at a screen position (logical pixels), opening it if needed. */
  showCard(path: string, x: number, y: number): Promise<void>;
  /** Whether this card window is the one links open their cards in. */
  cardIsLive(): Promise<boolean>;
  /** Pins this card window where it is; the next link opens a new one. */
  pinCard(): Promise<void>;
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
  /** Downloads the newer release's installer, checks its SHA-256, runs it and closes Leafline. */
  installUpdate(): Promise<void>;
  onUpdateStatus(handler: (status: UpdateStatus) => void): Promise<UnlistenFn>;
  /** The travel connections routes can use, and which file they come from. */
  travelInfo(): Promise<TravelInfo>;
  /** Downloads the taxis and return scrolls players recorded on mapleclassic.wiki. */
  downloadTravel(): Promise<TravelInfo>;
  /** Opens the player's copy of the travel file in their editor, made from the downloaded one if missing. */
  editTravelFile(): Promise<void>;
  /** Sizes the calling window to its content. */
  fitWindow(width: number, height: number): Promise<void>;

  /** Saves every record to a `.leafline` file: where the player picks (program) or to Downloads (website). Null if cancelled. */
  saveRecords(): Promise<RecordsSummary | null>;
  /** Picks a records file and says what it holds, changing nothing yet. Null if cancelled. */
  pickRecords(): Promise<RecordsSummary | null>;
  /** Replaces the records with the picked file's. The ones before are kept aside. */
  useRecords(): Promise<RecordsSummary>;
  /** The records were replaced from a file, in any window: what they show is out of date. */
  onRecordsReplaced(handler: () => void): Promise<UnlistenFn>;
}


export const backend: Backend = await pickBackend();

/** File name without its folder, for display. */
export function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}
