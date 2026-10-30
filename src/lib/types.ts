// Mirrors the structs the Rust side serialises (see src-tauri/src/pipeline.rs and readers/).

export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface Gauge {
  current: number;
  max: number;
}

export interface Exp {
  points: number;
  percent: number;
}

export interface HudReading {
  level: number | null;
  job: string | null;
  name: string | null;
  hp: Gauge | null;
  mp: Gauge | null;
  exp: Exp | null;
  /** The HUD font has not been learned at this size, or shows a character not seen yet. */
  needsCalibration: boolean;
}

/** What the user reads off their HUD to teach Leafline its font. */
export interface HudValues {
  level: number;
  hp: [number, number];
  mp: [number, number];
  expPoints: number;
  expPercent: string;
  name: string | null;
}

/** Which fields calibration could match to the screen. */
export interface Calibration {
  learned: string[];
  failed: string[];
}

export interface HudLayout {
  hp: Rect;
  mp: Rect;
  exp: Rect;
  labels: Rect;
  level: Rect | null;
  identity: Rect;
}

export interface Requirements {
  level: number | null;
  str: number | null;
  dex: number | null;
  int: number | null;
  luk: number | null;
  fam: number | null;
}

export interface JobFlag {
  job: string;
  eligible: boolean;
}

export interface Stat {
  label: string;
  value: number;
  base: number | null;
  bonus: number | null;
}

export interface Property {
  label: string;
  value: string;
}

export interface ItemTooltip {
  name: string;
  upgrades: number | null;
  itemType: string | null;
  requirements: Requirements;
  jobs: JobFlag[];
  stats: Stat[];
  properties: Property[];
  remainingEnhancements: number | null;
  panel: Rect;
}

export interface Objective {
  target: string;
  current: number;
  required: number;
}

export interface Quest {
  title: string;
  objectives: Objective[];
}

export interface QuestReading {
  quests: Quest[];
  /** Quests shown and the most that can be tracked. */
  page: [number, number] | null;
  panel: Rect;
}

export interface MesoReading {
  mesos: number;
  rect: Rect;
}

export interface MapReading {
  /** The street or area above the map's name, such as `Victoria Road`, when it was read. */
  street: string | null;
  name: string;
  /** Read from the collapsed minimap's one-line title. */
  collapsed: boolean;
  rect: Rect;
}

export interface Word {
  text: string;
  rect: Rect;
}

export interface Line {
  text: string;
  rect: Rect;
  words: Word[];
}

export interface Timing {
  stage: string;
  ms: number;
}

export interface Analysis {
  width: number;
  height: number;
  hud: HudReading | null;
  hudLayout: HudLayout | null;
  tooltips: ItemTooltip[];
  quest: QuestReading | null;
  mesos: MesoReading | null;
  /** Meso boxes on screen; more than one means a storage or trade window is open. */
  mesoBoxes: number;
  map: MapReading | null;
  overview: Line[];
  timings: Timing[];
}

/** The overlay's tabs, each of which can be pinned into a window of its own. */
export type Panel = "stats" | "map" | "train" | "quests" | "item";

export interface PinStatus {
  pinned: Panel[];
  /** Pinned panels are hidden for now, without being unpinned. */
  hidden: boolean;
}

export interface OverlayStatus {
  visible: boolean;
  clickThrough: boolean;
  /** Showing only EXP per hour and the quest counters, in a small window. */
  compact: boolean;
}

export interface ShortcutInfo {
  action: string;
  keys: string;
  label: string;
}

export interface MapSummary {
  /** `null` collects what cannot be placed: before any map was read, or spans across a map change. */
  map: string | null;
  activeSeconds: number;
  expGained: number;
  expPerHour: number | null;
  mesosGained: number;
  mesosPerHour: number | null;
}

/** What the session tracker has worked out so far (see src-tauri/src/session.rs). */
export interface SessionSummary {
  frames: number;
  startedAt: number | null;
  updatedAt: number | null;
  elapsedSeconds: number;
  /** Time spent gaining EXP, with idle stretches and map changes paused out. */
  activeSeconds: number;
  paused: boolean;
  level: number | null;
  exp: Exp | null;
  /** The EXP the current level needs, as measured from the HUD. */
  expNeeded: number | null;
  expGained: number;
  expPerHour: number | null;
  percentPerHour: number | null;
  secondsToLevel: number | null;
  levelUps: number;
  mesos: number | null;
  mesosGained: number;
  mesosPerHour: number | null;
  /** Net change while a storage or trade window was open, left out of the rates. */
  mesosExcluded: number;
  map: string | null;
  maps: MapSummary[];
  /** Quest counter gains per objective and map, most active time first. */
  counters: CounterRate[];
  rejected: { exp: number; mesos: number };
}

/** How fast one quest counter went up on one map (see src-tauri/src/session.rs). */
export interface CounterRate {
  target: string;
  required: number;
  /** `null` for gains that spanned a map change. */
  map: string | null;
  gained: number;
  activeSeconds: number;
  /** Shown once the counter went up at least twice over enough active time. */
  perHour: number | null;
}

/** How a replay orders frames: by file time, or by name a fixed interval apart. */
export type ReplayOrder = { by: "modified" } | { by: "name"; intervalSeconds: number };

export interface ReplayProgress {
  done: number;
  total: number;
  path: string;
  at: number;
  analysis: Analysis;
  session: SessionSummary;
}

// Map data (see src-tauri/src/mapdata/). Facts from OSMS Data Explorer, fetched on request.

/** What map data is cached locally. */
export interface MapDataStatus {
  /** Every file the map card needs has been downloaded. */
  ready: boolean;
  clientBuild: string | null;
  dataUpdatedAt: string | null;
  /** When OSMS was last asked for changes, in Unix seconds. */
  checkedAt: number | null;
  files: { name: string; records: number; updatedAt: string; fetchedAt: number }[];
}

export interface MapDataDownload {
  report: { checked: boolean; downloaded: string[] };
  status: MapDataStatus;
}

export interface MapRef {
  id: number;
  name: string;
  street: string | null;
  /** False for maps the data refers to but does not contain. */
  known: boolean;
}

export interface MobRow {
  id: number;
  name: string;
  level: number;
  hp: number;
  exp: number;
  count: number;
  boss: boolean;
  undead: boolean;
}

export interface QuestRef {
  id: number;
  name: string;
  levelMin: number | null;
}

export interface QuestNeed {
  quest: QuestRef;
  giver: string | null;
  targets: { mobId: number; name: string; count: number }[];
}

/** Which way a portal leads, judged from its name. */
export type Direction = "west" | "east" | "in" | "out" | "other";

export interface MapExit {
  direction: Direction;
  to: MapRef;
  portals: string[];
  hidden: boolean;
}

export interface MapDetails {
  map: MapRef;
  region: string | null;
  isTown: boolean;
  returnMap: MapRef | null;
  levelRange: [number, number] | null;
  mobs: MobRow[];
  mobTotal: number;
  /** Base EXP for killing every monster once. */
  expPerClear: number;
  npcs: { id: number; name: string; quests: QuestRef[] }[];
  quests: QuestNeed[];
  exits: MapExit[];
  links: { label: string; url: string }[];
}

export interface MapMatch {
  /** How closely the OCR'd name matched, 0 to 1. */
  score: number;
  card: MapDetails;
  alternatives: MapRef[];
}

export interface RouteStep {
  from: MapRef;
  to: MapRef;
  direction: Direction;
  portal: string | null;
  hidden: boolean;
  /** Set when the step is a ride or scroll rather than a portal. */
  travel: TravelStep | null;
}

export interface MapRoute {
  from: MapRef;
  to: MapRef;
  steps: RouteStep[];
  /** For a route that rides or scrolls: the portals the walk alone takes, or null when no portals lead there. */
  walking: number | null;
}

/** Taxis and return scrolls (see src-tauri/src/mapdata/travel.rs). */
export type TravelKind = "cab" | "vipCab" | "scroll";

export interface TravelStep {
  kind: TravelKind;
  service: string;
  /** Mesos at level 20 and up, as the wiki records it. */
  fare: number | null;
}

/** A trip that rides or scrolls part of the way, shorter than walking. */
export interface Shortcut {
  /** Steps, counting each ride or scroll as one. */
  hops: number;
  /** Where the first ride or scroll goes. */
  via: MapRef;
}

/** The travel file: connections the map data lacks, from mapleclassic.wiki, editable by the player. */
export interface TravelFile {
  about: string;
  checked: string | null;
  services: {
    kind: TravelKind;
    name: string;
    operator?: string;
    note?: string;
    source: string;
    routes: { from?: string; to: string; fare?: number }[];
  }[];
}

export interface TravelInfo {
  file: TravelFile;
  /** The player's own copy is in use. */
  custom: boolean;
  path: string | null;
  /** Why the player's copy could not be used. */
  error: string | null;
  /** Map names the map data has no map for; null before map data is downloaded. */
  missing: string[] | null;
}

/** Which travel the player turned on. */
export interface TravelChoice {
  enabled: boolean;
  cabs: boolean;
  vipCabs: boolean;
  scrolls: boolean;
}

/** Drops players logged on mapleclassic.wiki. */
export interface MonsterDrops {
  monster: string;
  sampleKills: number | null;
  gameVersion: string | null;
  fetchedAt: number;
  drops: {
    item: string;
    category: string | null;
    rate: number | null;
    rateLow: number | null;
    rateHigh: number | null;
    minQuantity: number | null;
    maxQuantity: number | null;
    sampleKills: number | null;
    sampleDrops: number | null;
    gameVersion: string | null;
    quest: boolean;
  }[];
}

// Quest guide (see src-tauri/src/mapdata/guide.rs): where to go for each Quest Helper objective.

/** A quest as the Quest Helper showed it, for the guide. */
export interface QuestQuery {
  title: string;
  objectives: string[];
}

export interface HuntingGround {
  map: MapRef;
  /** How many of the monster the map spawns. */
  count: number;
  /** Portals from the current map, when it is known and a route exists. */
  hops: number | null;
  /** Fewer steps with the travel the player turned on, when that is shorter. */
  travel: Shortcut | null;
  /** The player's EXP rate measured here, as kills of this monster. */
  pace: Pace | null;
}

/**
 * A rough kill rate from the EXP per hour the player measured on a map, assuming they defeat its
 * monsters in proportion to how many of each spawn, at the map data's base EXP.
 */
export interface Pace {
  expPerHour: number;
  activeSeconds: number;
  killsPerHour: number;
}

export interface Hunt {
  monster: { name: string; known: boolean; level: number; hp: number; exp: number; boss: boolean };
  /** For an item: how often players saw this monster drop it (mapleclassic.wiki). */
  drop: {
    rate: number | null;
    rateLow: number | null;
    rateHigh: number | null;
    sampleKills: number | null;
    gameVersion: string | null;
    quest: boolean;
  } | null;
  /** Most of the monster first. */
  maps: HuntingGround[];
}

export interface ObjectiveGuide {
  target: string;
  kind: "monster" | "item";
  /** The wiki's name for an item, once looked up and found. */
  item: string | null;
  /** Whether the wiki has been asked who drops this item. */
  lookedUp: boolean;
  hunts: Hunt[];
}

export interface QuestGuide {
  title: string;
  quest: { quest: QuestRef; giver: string | null; giverMaps: MapRef[] } | null;
  objectives: ObjectiveGuide[];
}

export interface Guide {
  /** The map portals are counted from, when the minimap was read and matched. */
  from: MapRef | null;
  quests: QuestGuide[];
}

// Training spots (see src-tauri/src/mapdata/training.rs).

/** EXP per hour the session tracker measured on a map, keyed by its minimap label. */
export interface MeasuredRate {
  label: string;
  expPerHour: number;
  activeSeconds: number;
}

export interface TrainingSpot {
  map: MapRef;
  region: string | null;
  levelRange: [number, number];
  /** Monster level averaged over everything the map spawns. */
  averageLevel: number;
  mobTotal: number;
  /** Base EXP for killing every monster once. */
  expPerClear: number;
  /** Total HP of every monster on the map. */
  hpPerClear: number;
  mobs: MobRow[];
  hops: number | null;
  /** Fewer steps with the travel the player turned on, when that is shorter. */
  travel: Shortcut | null;
  /** Portals lead here from some town; party quest stages and other instances are not. */
  reachable: boolean;
  measured: { expPerHour: number; activeSeconds: number } | null;
}

export interface TrainingSpots {
  from: MapRef | null;
  spots: TrainingSpot[];
}

// Price log (see src-tauri/src/userdata/prices.rs): only prices the player logged.

export type PriceKind = "traded" | "asking";

export interface NewPrice {
  item: string;
  upgrades: number | null;
  slots: number | null;
  stats: string;
  price: number;
  kind: PriceKind;
  note: string | null;
}

export interface PriceEntry extends NewPrice {
  id: number;
  /** Unix seconds. */
  recordedAt: number;
}

export interface PriceStats {
  count: number;
  traded: number;
  median: number;
  low: number;
  high: number;
  firstAt: number;
  lastAt: number;
}

export interface PriceLog {
  item: string | null;
  /** Newest first. */
  entries: PriceEntry[];
  all: PriceStats | null;
  /** Records with the same upgrades and slots as the item asked about. */
  likeThis: PriceStats | null;
}

/** Display settings for the overlay and pinned panels (see src-tauri/src/app/settings.rs). */
export interface DisplaySettings {
  /** 0.4 to 1. */
  opacity: number;
  /** 0.85 to 1.3. */
  scale: number;
  /** Taxis and return scrolls in routes; off by default. */
  travel: TravelChoice;
}

// Session history (see src-tauri/src/userdata/sessions.rs).

export interface SavedSession {
  id: number;
  label: string;
  /** Unix seconds. */
  savedAt: number;
  startedAt: number | null;
  level: number | null;
  levelUps: number;
  activeSeconds: number;
  elapsedSeconds: number;
  expGained: number;
  expPerHour: number | null;
  mesosGained: number;
  mesosPerHour: number | null;
  /** Most active time first; time that couldn't be placed on a map is left out. */
  maps: { map: string; activeSeconds: number; expGained: number; mesosGained: number }[];
}

/** One map's rates over every saved session that spent time there. */
export interface MapRate {
  map: string;
  sessions: number;
  activeSeconds: number;
  expPerHour: number;
  mesosPerHour: number;
}

export interface History {
  /** Newest first. */
  sessions: SavedSession[];
  maps: MapRate[];
}
