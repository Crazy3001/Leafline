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
  /** The EXP percentage, when the HUD shows it with no points beside it, as it does at high levels. */
  expPercent: number | null;
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
  /** Upgraded, going by its amber name, even if the count couldn't be read. */
  upgraded: boolean;
  itemType: string | null;
  requirements: Requirements;
  jobs: JobFlag[];
  stats: Stat[];
  properties: Property[];
  remainingEnhancements: number | null;
  panel: Rect;
  /** An item with no stats: only the name and `itemType`, which holds any tags, are read. */
  plain: boolean;
}

export interface Objective {
  target: string;
  current: number;
  required: number;
  /** A line the quest data says is there that no reading has counted yet; shown without a count. */
  unread?: boolean;
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

/** A character's mesos the last time they were on screen, remembered between runs. */
export interface KnownMesos {
  name: string;
  mesos: number;
  /** Unix seconds. */
  seenAt: number;
}

export interface MapReading {
  /** The street or area above the map's name, such as `Victoria Road`, when it was read. */
  street: string | null;
  name: string;
  /** Read from the collapsed minimap's one-line title. */
  collapsed: boolean;
  rect: Rect;
  /** Which map this is, when the header alone can't say and the live loop worked it out. */
  twin: number | null;
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
  /** An NPC shop, storage or trade window is open, going by its buttons. */
  shopOpen: boolean;
  /** How many are in the party, the player included; 1 with no party list in view. */
  partySize: number;
  map: MapReading | null;
  overview: Line[];
  timings: Timing[];
  /** Parts that hit an error this time. The rest were still read. */
  failed: string[];
}

/** How an item check from the hotkey went. */
export interface ItemRead {
  /** Item tooltips found, the compared item included. */
  found: number;
  /** Why no picture could be read. */
  problem: string | null;
}

/** The overlay's tabs, each of which can be pinned into a window of its own. */
export type Panel = "character" | "map" | "train" | "quests" | "item";

/** What can be pinned: any tab, and the "Go here" route, which is not a tab. */
export type PinId = Panel | "route";

export interface PinStatus {
  pinned: PinId[];
  /** Pinned panels are hidden for now, without being unpinned. */
  hidden: boolean;
}

/** The desk view, the panels side by side in one window of their own. */
export interface DeskStatus {
  /** Open when Leafline last quit, so it opens again at launch. */
  open: boolean;
  onTop: boolean;
}

export interface OverlayStatus {
  visible: boolean;
  clickThrough: boolean;
  /** Showing only EXP per hour and the quest counters, in a small window. */
  compact: boolean;
}

// Live reading (see src-tauri/src/app/live.rs and src-tauri/src/capture).

export type LiveStatus =
  | { state: "off" }
  /** Running, with nothing to read right now. */
  | { state: "waiting"; reason: string }
  | {
      state: "reading";
      window: string;
      width: number;
      height: number;
      frames: number;
      readMs: number;
      /** Windows HDR is on for the game's display, which washes the picture out. */
      hdr: boolean;
      /** Tracking is paused by hand. */
      held: boolean;
    }
  | { state: "failed"; message: string };

/** How the chosen game window is recognised again. */
export interface WindowChoice {
  title: string;
  class: string;
}

export interface GameWindow extends WindowChoice {
  id: number;
  width: number;
  height: number;
  /** Looks like the MapleStory client. */
  likelyGame: boolean;
}

export interface LiveChoice {
  /** Off until the player has read the warning and turned it on. */
  enabled: boolean;
  /** Seconds between readings, 1 to 10. */
  interval: number;
  /** Seconds without EXP before the clock pauses, 5 to 300. */
  pauseAfter: number;
  window: WindowChoice | null;
}

export interface ShortcutInfo {
  action: string;
  /** Empty when this action has no hotkey. */
  keys: string;
  /** The key setup offers, or "" for none. */
  suggestedKeys: string;
  label: string;
  /** Setup asks about it. */
  inSetup: boolean;
  /** Its keys are set but another program on this PC holds them, so they do nothing. */
  taken: boolean;
}

/** A stretch of play on one map at one level, party size and job. */
export interface SessionLine {
  map: string | null;
  town: boolean;
  level: number | null;
  party: number | null;
  job: string | null;
  activeSeconds: number;
  expGained: number;
  percentGained: number;
  mesosGained: number;
  mesosSeconds: number;
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
  /** The player paused tracking by hand. */
  held: boolean;
  level: number | null;
  exp: Exp | null;
  /** The latest EXP percentage, read with or without points. */
  percent: number | null;
  /** The HUD shows only a percentage, so EXP amounts are unknown. */
  percentOnly: boolean;
  /** The EXP the current level needs, as measured from the HUD. */
  expNeeded: number | null;
  expGained: number;
  expPerHour: number | null;
  percentPerHour: number | null;
  secondsToLevel: number | null;
  /** Rates over the last five minutes of active time. */
  recentExpPerHour: number | null;
  recentPercentPerHour: number | null;
  recentMesosPerHour: number | null;
  /** The recent EXP rate (or percent rate at high levels) over time, oldest first. */
  trend: number[];
  /** The recent rates over the whole session, oldest first, for graphs. */
  timeline: TimelinePoint[];
  /** Level-ups and deaths, in order. */
  events: SessionEvent[];
  levelUps: number;
  deaths: number;
  /** EXP lost to deaths, left out of the rates. */
  expLost: number;
  /** Percent of a level lost to deaths. */
  percentLost: number;
  mesos: number | null;
  /** Seconds since the mesos were last on screen. */
  mesosAge: number | null;
  mesosGained: number;
  mesosPerHour: number | null;
  /**
   * The active time the meso rate is over: from one look at the mesos to a later one showing a
   * change. Less than `activeSeconds` when the mesos went unlooked-at for a while.
   */
  mesosSeconds: number;
  /** Active seconds since the mesos were last on screen; mesos gained since then aren't counted yet. */
  mesosUnseenSeconds: number;
  /** Net change while a shop, storage or trade window was open, left out of the rates. */
  mesosExcluded: number;
  map: string | null;
  /** The map is a town. */
  town: boolean;
  /** The minimap's name has not been seen for a while, so the map may be out of date. */
  mapHidden: boolean;
  /** The character's name, once read the same a few times in a row. */
  name: string | null;
  job: string | null;
  party: number | null;
  maps: MapSummary[];
  /** Per map, level, party size and job, most active time first. */
  lines: SessionLine[];
  /** Quest counter gains per objective and map, most active time first. */
  counters: CounterRate[];
  rejected: { exp: number; mesos: number };
}

/** The recent rates at one moment of a session (see src-tauri/src/session.rs). */
export interface TimelinePoint {
  /** Active seconds into the session. */
  at: number;
  expPerHour: number | null;
  percentPerHour: number | null;
  mesosPerHour: number | null;
  level: number | null;
  map: string | null;
}

export interface SessionEvent {
  /** Active seconds into the session. */
  at: number;
  kind: "levelUp" | "death";
  /** The level reached, for a level-up. */
  level: number | null;
}

/** A saved session's graph data. Sessions saved before timelines were kept have none. */
export interface SessionTimeline {
  points: TimelinePoint[];
  events: SessionEvent[];
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
  /** How much of a weapon blow it shrugs off, when the data says. */
  weaponDef?: number | null;
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
/** Which third of its map a portal stands in, across and down. */
export type Side = "left" | "centre" | "right";
export type Height = "high" | "middle" | "low";

export interface MapExit {
  /** A guess from the portal's name; `side` and `height` outrank it when known. */
  direction: Direction;
  side: Side | null;
  height: Height | null;
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
  /** NPC shops here, from Leafline's shop file. */
  shops: Shop[];
}

// NPC shops (see src-tauri/src/mapdata/shops.rs), from Leafline's own shop file.

export interface ShopItem {
  item: string;
  /** The item in the map data, when its name matched one. */
  itemId: number | null;
  price: number;
  /** A town standing the shop asks for first, as the game words it. */
  needs: string | null;
  /** The day the price was last seen, YYYY-MM-DD. */
  seen: string;
}

export interface Shop {
  npc: string;
  npcId: number | null;
  role: string | null;
  map: MapRef | null;
  /** The map as the shop file names it. */
  mapName: string;
  items: ShopItem[];
}

/** One shop selling an item. */
export interface ShopOffer {
  npc: string;
  npcId: number | null;
  role: string | null;
  map: MapRef | null;
  mapName: string;
  hops: number | null;
  price: number;
  needs: string | null;
  seen: string;
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
  side: Side | null;
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
  /** About how many seconds of walking, when every map on the way has a known layout. */
  walkSeconds?: number | null;
}

/** A place to head for with "Go here" (see src-tauri/src/app/nav.rs): a map, or an NPC on any of their maps. */
export interface Destination {
  kind: "map" | "npc";
  /** The map's or the NPC's name. */
  name: string;
  /** Reaching any of these is arriving; the nearest is headed for. */
  maps: MapRef[];
}

/** The place headed for, shared by every window, and the places headed for lately. */
export interface NavState {
  destination: Destination | null;
  /** The newest first. */
  recent: Destination[];
}

/** How the way to the destination stands, worked out in the overlay and sent to the Route pin. */
export interface Trip {
  /** The map the player was last read on, held through a missed or rough reading. */
  here: MapRef | null;
  /** The last reading was too rough to trust, so `here` may be out of date. */
  unsure: boolean;
  /** The way from where the route was worked out; `next` is the step to take now. */
  route: MapRoute | null;
  next: number;
  /** Working a route out. */
  routing: boolean;
  /** No way there is known from `here`. */
  lost: boolean;
  /** The route was just worked out again after a turn off it. */
  rerouted: boolean;
  /** Said for a moment after arriving, once the destination has cleared. */
  arrived: string | null;
  /** A quest whose counters just filled, and where to hand it in: offered, never set unasked. */
  offer: { title: string; quest: string; destination: Destination; giver: boolean } | null;
}

/** A Route pin's word to the overlay: send the trip as it stands, or drop the turn-in offer. */
export type TripMessage = "ask" | "dismiss-offer";

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

/** The travel file: connections the map data lacks, downloaded from mapleclassic.wiki or written by the player. */
export interface TravelFile {
  about: string;
  /** When they were downloaded, in Unix seconds. */
  fetchedAt?: number;
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
  /** Null until downloaded or written. */
  file: TravelFile | null;
  /** The player's own copy, the wiki's, or nothing yet. */
  origin: "custom" | "wiki" | "none";
  /** Where the player's own copy is, or would be. */
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

/** A monster's known drops, from Leafline's drop list. */
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
  /** How many monsters of any kind the map spawns. */
  mapTotal: number;
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
  /** For an item: that this monster drops it, with a rate where one is known. */
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
  /** The drop list's name for an item, once looked up and found. */
  item: string | null;
  /** Whether the drop list has been searched for who drops this item. */
  lookedUp: boolean;
  hunts: Hunt[];
}

export interface QuestGuide {
  title: string;
  quest: {
    quest: QuestRef;
    giver: string | null;
    giverMaps: MapRef[];
    /** Who takes the quest in, when something shows it. Not always the giver. */
    turnIn: string | null;
    turnInMaps: MapRef[];
  } | null;
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
  /** The character's level then, when read. */
  level?: number | null;
  /** Party size then; none or one is solo. */
  party?: number | null;
}

/**
 * What the training estimates take as given because the map data doesn't say (see
 * src-tauri/src/mapdata/training.rs). The player can change both.
 */
export interface Assumptions {
  /** Seconds before a defeated monster is back. */
  respawnSeconds: number;
  /** EXP a kill gives over the data's base EXP, for events or server rates. */
  expRate: number;
  /** The player's attack range from their stat window; null until entered. With it the
   *  estimates count each monster's weapon defence. */
  attackMin: number | null;
  attackMax: number | null;
  /** How hard a blow of their usual skill hits over a plain attack, when typed. */
  skillFactor: number;
  /** Which skill is the main one, by name: empty to use the best one Leafline read, or
   *  "custom" to use the typed `skillFactor`. */
  skillName: string;
}

/** What Leafline read of a character's damage from the game (src-tauri/src/app/damage.rs). */
export interface PlayerDamage {
  /** The stat window's Attack line. */
  attackMin: number | null;
  attackMax: number | null;
  /** The damaging skills last seen in the skill window. */
  skills: DamageSkill[];
}

export interface DamageSkill {
  name: string;
  level: number;
  /** Its damage at that level: 240 is 240%. */
  percent: number;
}

/** How fast the player gets through monster HP, from their own solo play. */
export interface KillSpeed {
  hpPerHour: number;
  /** HP of the average monster defeated in that play. */
  hpPerKill: number;
  activeSeconds: number;
  maps: number;
  /** Lowest and highest character level in that play, when read. */
  levels: [number, number] | null;
  /** Too little play near the current level, so every solo session counts. */
  widened: boolean;
  /** Seconds of walking between kills on the maps that play was on, from their layouts. */
  walkSecondsPerKill?: number | null;
  /** Weapon defence of the monsters in that play, on average. */
  defence?: number;
}

/** A map's EXP per hour for this player, worked out from their kill speed. */
export interface Estimate {
  expPerHour: number;
  killsPerHour: number;
  /** Kills an hour if every monster were defeated as soon as it came back. */
  ceiling: number;
  /** Held to the ceiling: the player would outpace the respawn. */
  capped: boolean;
  /** HP of the average monster here over the average one in the player's play. */
  hpRatio: number;
  /** Average monster level here minus the character's level. */
  levelGap: number | null;
  /** 3 fairly sure, 2 rough, 1 a guess. */
  sureness: 1 | 2 | 3;
  /** Seconds of walking between kills here; the rest of each kill scales with HP. */
  walkSecondsPerKill?: number | null;
  /** Blows to defeat the average monster here, when the player's attack range is known. */
  hitsPerKill?: number | null;
  /** Weapon defence of the average monster here. */
  weaponDef?: number;
  /** Whether the player's attack range was counted, so defence is too. */
  damageKnown?: boolean;
}

/** A map's size and how its monsters spread, in the game's pixels, from the client. */
export interface MapLayout {
  map: number;
  width: number | null;
  height: number | null;
  /** Summed length of every floor. */
  walkable: number;
  platforms: number;
  spawnPoints: number;
  /** How many platforms monsters spawn on. */
  spawnPlatforms: number;
  ropes: number;
  ladders: number;
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
  /** From the player's kill speed, once enough solo play is saved. */
  estimate: Estimate | null;
  /** The map's size, floors and how its monsters spread, when known. */
  layout?: MapLayout | null;
}

export interface TrainingSpots {
  from: MapRef | null;
  /** Behind every estimate; null until enough solo play is saved. */
  speed: KillSpeed | null;
  spots: TrainingSpot[];
}

/** A training spot the player saved (src-tauri/src/userdata/spots.rs). */
export interface SavedSpot {
  mapId: number;
  /** The map's name when it was saved. */
  name: string;
  /** Unix seconds. */
  savedAt: number;
}

/** The player's own note on an item (src-tauri/src/userdata/notes.rs). */
/** The player's note on a map (src-tauri/src/userdata/notes.rs). */
export interface MapNote {
  mapId: number;
  /** The map's name when the note was written. */
  map: string;
  note: string;
  /** Unix seconds. */
  updatedAt: number;
}

export interface ItemNote {
  item: string;
  note: string;
  /** Unix seconds. */
  updatedAt: number;
}

/** A drop the player is logging (src-tauri/src/userdata/drops.rs). */
export interface NewDrop {
  item: string;
  itemId: number | null;
  monster: string;
  monsterId: number | null;
  map: string | null;
  mapId: number | null;
  character: string | null;
}

/** A drop in the player's log: their own word on which monster dropped it. */
export interface LoggedDrop extends NewDrop {
  id: number;
  /** Unix seconds. */
  seenAt: number;
}

// Price log (see src-tauri/src/userdata/prices.rs): only prices the player logged.

export type CompactShape = "card" | "wide" | "bar";

export interface HotkeyChoice {
  enabled: boolean;
  /** Keys the player changed, by action. Empty turns that hotkey off. */
  keys: Record<string, string>;
}

/** Display settings for the overlay and pinned panels (see src-tauri/src/app/settings.rs). */
export interface DisplaySettings {
  /** 0.4 to 1. */
  opacity: number;
  /** 0.6 to 2. */
  scale: number;
  /** Keep the overlay and pinned panels above the game. */
  alwaysOnTop: boolean;
  /** Clicking the overlay or a pinned panel leaves the game focused. */
  keepGameFocus: boolean;
  compactShape: CompactShape;
  hotkeys: HotkeyChoice;
  /** Taxis and return scrolls in routes; off by default. */
  travel: TravelChoice;
  live: LiveChoice;
  /** How the build planner counts points. The player can correct these. */
  buildRules: BuildRules;
  /** Where saved game pictures go; null is the app's local folder, which never syncs. */
  framesFolder: string | null;
  /** The first-launch setup has been finished. */
  setupDone: boolean;
  /** Ask GitHub for the latest version number at startup. Off until the player turns it on. */
  updateCheck: boolean;
}

/** The optional update check (src-tauri/src/app/update.rs). */
export type UpdateStatus =
  | { state: "off" }
  | { state: "checking" }
  | { state: "upToDate"; current: string; latest: string }
  | { state: "available"; current: string; latest: string; url: string }
  | { state: "unavailable"; current: string; reason: string }
  | { state: "failed"; current: string; message: string };

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
  percentPerHour: number | null;
  job: string | null;
  /** The character's name, when the tracker read it. */
  character: string | null;
  deaths: number;
  /** The player's own note. It stays on this PC. */
  note: string;
  /** Per map, level, party size and job, most active time first. */
  lines: SessionLine[];
  /** Most active time first; time that couldn't be placed on a map is left out. */
  maps: { map: string; activeSeconds: number; expGained: number; mesosGained: number }[];
  /** The potion counts the player typed, in slot order. */
  potions: SessionPotion[];
  /** What the potions used cost, in mesos. */
  potionCost: number;
}

// Crafting (see src-tauri/src/build/crafting.rs): recipes from the map data.

export interface ItemAmount {
  id: number;
  name: string;
  count: number;
}

export interface Recipe {
  id: number;
  discipline: string;
  /** The discipline's skill level the recipe asks for. */
  skillLevel: number;
  craftExp: number;
  mesoCost: number;
  output: ItemAmount;
  ingredients: ItemAmount[];
}

export interface ItemCrafting {
  madeBy: Recipe[];
  usedIn: Recipe[];
}

export interface CraftingQuery {
  discipline?: string | null;
  search?: string | null;
  maxLevel?: number | null;
}

export interface CraftingPage {
  /** Every discipline with how many recipes it has. */
  disciplines: [string, number][];
  recipes: Recipe[];
}

/** The EXP a level takes, as the player's own HUD showed it (src-tauri/src/userdata/sessions.rs). */
export interface LevelExp {
  level: number;
  needed: number;
}

/** A potion the player hunts with (src-tauri/src/userdata/potions.rs). Nothing here is bundled. */
export interface PotionSlot {
  /** Stays with the slot; 0 for a new one. */
  slot: number;
  name: string;
  /** What one costs the player, in mesos. */
  price: number;
}

/** A slot's counts for a session, with the name and price as they were when typed. */
export interface SessionPotion {
  slot: number;
  name: string;
  price: number;
  start: number | null;
  now: number | null;
  /** Start minus now once both are typed, never below zero. */
  used: number;
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

// The build planner (see src-tauri/src/build/).

export type BaseStat = "str" | "dex" | "int" | "luk";

export type BaseStats = Record<BaseStat, number>;

/** A skill and a level: a goal, or what a character already has. */
export interface SkillLevel {
  skillId: number;
  level: number;
}

/** Raise `stat` to `target`, or with no target put all the AP left there. */
export interface StatGoal {
  stat: BaseStat;
  target: number | null;
  /** Waits for this level. */
  fromLevel: number | null;
}

/** Where the character is now, as its windows show it. */
export interface BuildStart {
  level: number;
  /** Null until the player has entered them. */
  stats: BaseStats | null;
  ap: number;
  sp: number;
  skills: SkillLevel[];
  /** `male` or `female`, for gear that is made for one. */
  gender?: string | null;
  /** Equipment the character has, each as an item check read it. Changed only by add/removePiece. */
  pieces?: GearPiece[];
  /** What worn gear added to each stat when the Character Stat window was last read. Shown only. */
  gearBonus?: BaseStats | null;
}

/** One copy of an item, as its tooltip showed it, scrolls and all. */
export interface GearPiece {
  /** Tells two copies apart; given when it is added (0 before). */
  id: number;
  /** The data's item, when the name matched one. */
  item: number | null;
  name: string;
  itemType: string | null;
  level: number | null;
  stats: { label: string; value: number }[];
  /** Successful scrolls, from the name's (+N). */
  upgrades: number | null;
  slotsLeft: number | null;
  /** The data's upgrade slots for the item. */
  slots: number | null;
}

/** The part of a build that can be shared. */
export interface BuildGoals {
  /** The first or second job chosen; 0 before there is one. */
  job: number;
  skills: SkillLevel[];
  stats: StatGoal[];
  /** Equipment to aim for, by item id. */
  gear: number[];
  planTo: number;
}

export interface Build {
  /** 0 until it has been saved. */
  id: number;
  name: string;
  /** Follows this character when the HUD shows its name. */
  character: string | null;
  /** A link to the guide this build follows, typed by the player. */
  guide: string;
  start: BuildStart;
  plan: BuildGoals;
}

/** A ready-made plan, written for Leafline or following one guide. Nobody has played these yet. */
export interface StarterPlan {
  id: string;
  job: number;
  name: string;
  summary: string;
  plan: BuildGoals;
  /** Names the data doesn't have, left out of the plan. */
  missing: string[];
  /** The guide a plan follows, to credit and link. `null` for a plan written for Leafline. */
  source: StarterSource | null;
}

export interface StarterSource {
  name: string;
  /** The exact page the plan was taken from. */
  url: string;
  credit: string;
}

export interface BuildSummary {
  id: number;
  name: string;
  character: string | null;
  updatedAt: number;
}

export interface StatGain {
  stat: BaseStat;
  amount: number;
  total: number;
}

export interface SkillGain {
  skillId: number;
  name: string;
  from: number;
  to: number;
}

/** What happens on reaching a level. */
export interface LevelStep {
  level: number;
  /** The job advancement due at this level. */
  advance: string | null;
  ap: StatGain[];
  sp: SkillGain[];
  /** Goal gear this level can first wear. */
  gear: string[];
  /** Goal gear worn by now whose stats count toward other gear's requirements. */
  worn: { slot: string; bonus: BaseStats }[];
  /** Base stats after this level's AP. */
  stats: BaseStats;
  apLeft: number;
  spLeft: number;
}

export interface BuildProblem {
  severity: "note" | "warning";
  level: number | null;
  message: string;
}

export interface GearItem {
  id: number;
  name: string;
  slot: string;
  gender: string | null;
  /** Who can use it, as tooltips name them. Empty means anyone. */
  classes: string[];
  weaponType: string | null;
  reqLevel: number;
  reqStr: number;
  reqDex: number;
  reqInt: number;
  reqLuk: number;
  stats: { label: string; value: number }[];
}

export type ItemKind = "equip" | "use" | "scroll" | "setup" | "etc";

/** Any item in the map data. Only equipment has a slot, stats and requirements that matter. */
export interface ItemRecord extends GearItem {
  kind: ItemKind;
  reqFame: number;
  /** The data's own number for a weapon's speed. Lower is faster. */
  attackSpeed: number | null;
  /** Upgrade slots on a clean copy. */
  upgradeSlots: number | null;
  questItem: boolean;
  untradable: boolean;
  /** The price the data lists. What it means in Classic isn't confirmed. */
  price: number | null;
  description: string | null;
  scroll: { fits: string; tier: string | null; successRate: number | null } | null;
}

export interface ItemQuery {
  text: string;
  kind: ItemKind | null;
  /** A slot or a weapon type. Scrolls count as what they go on. */
  slot: string | null;
  /** Only equipment this class can use. */
  class: string | null;
  levelMin: number | null;
  levelMax: number | null;
  /** `name`, or a stat's label to put the highest first. By level otherwise. */
  sort: string | null;
  /** Only equipment the character's build can wear at its level. */
  mine: boolean;
  /** Only scrolls that raise this, such as `Attack` or `Weapon Def.`. */
  stat: string | null;
  /** Only scrolls with this chance: `100%`, `60%`, `10%` or `Chaos`. */
  chance: string | null;
  offset: number;
}

export interface ItemPage {
  items: ItemRecord[];
  /** How many matched in all. */
  total: number;
  /** Nothing matched outright, so these are the closest names. */
  closest: boolean;
}

export interface ItemFilters {
  slots: string[];
  weapons: string[];
  /** What scrolls go on, with every weapon type under `Weapon`. */
  scrollSlots: string[];
  /** What scrolls for each of `scrollSlots` raise. */
  scrollStats: Record<string, string[]>;
  scrollChances: string[];
  /** Groups worked out from where an item's number falls, since the data has none for these. */
  useGroups: string[];
  setupGroups: string[];
  etcGroups: string[];
}

/** Where to get an item. */
export interface ItemWhere {
  /** The map portals are counted from, when the minimap was read and matched. */
  from: MapRef | null;
  origin: {
    drops: ObjectiveGuide;
    /** Quests that can give it, the lowest level first. */
    quests: NonNullable<QuestGuide["quest"]>[];
    /** NPC shops that sell it, the cheapest first. */
    shops: ShopOffer[];
  };
}

/** What to bring up in the Lookup tab: one item, or a search when no item was matched. */
export interface ItemLink {
  id: number | null;
  text: string | null;
}

/** How reading the Character Stat and Skill windows went. */
export interface CharacterRead {
  /** Which windows were read. */
  stats: boolean;
  skills: boolean;
  /** The build that was brought up to date. */
  build: string | null;
  result: {
    changes: string[];
    /** Skills in the window that aren't on the build's job path, or didn't read. */
    unknown: string[];
  };
  /** Why nothing was updated. */
  problem: string | null;
  /** It came from a live picture, not from the player asking. */
  automatic: boolean;
}

// The monster search (see src-tauri/src/mapdata/bestiary.rs).

export interface MonsterQuery {
  /** A monster, an item it drops, or a map it lives on. */
  text: string;
  kind: string | null;
  weakTo: string | null;
  region: string | null;
  /** One of `mapAreas`. */
  area: string | null;
  levelMin: number | null;
  levelMax: number | null;
  /** `name`, `hp`, `exp` or `efficiency`. By level otherwise. */
  sort: string | null;
  /** With no `sort`, the monsters nearest this level first. */
  near: number | null;
  offset: number;
}

export interface MonsterRow {
  id: number;
  name: string;
  level: number;
  hp: number;
  exp: number;
  boss: boolean;
  undead: boolean;
  /** Why a search found it, when that isn't its name. */
  note: string | null;
}

export interface MonsterPage {
  monsters: MonsterRow[];
  total: number;
  /** Nothing matched outright, so these are the closest names. */
  closest: boolean;
}

export interface MonsterFilters {
  regions: string[];
  /** The elements some monster is weak to. */
  elements: string[];
}

/** What the data says of a monster beyond its name, level, HP and EXP. Missing stats are null. */
export interface MonsterDetails {
  mp: number | null;
  attack: number | null;
  magic: number | null;
  weaponDef: number | null;
  magicDef: number | null;
  accuracy: number | null;
  evasion: number | null;
  /** Added to its movement speed: below zero is slower. */
  speed: number | null;
  /** The damage one hit must deal to knock it back. */
  knockback: number | null;
  hpRecovery: number | null;
  mpRecovery: number | null;
  /** Comes at players on sight. */
  aggressive: boolean;
  invincible: boolean;
  /** Weak ones first, then strong, then immune. */
  elements: { element: string; effect: "Weak" | "Strong" | "Immune" }[];
  magicAttacks: boolean;
  /** The elements its attacks carry. */
  attackElements: string[];
  /** What its skills do: `self_buff` or `debuff`. */
  casts: string[];
}

/** All the map data says about one monster, and what players logged of its drops. */
export interface MonsterSheet {
  id: number;
  name: string;
  level: number;
  hp: number;
  exp: number;
  boss: boolean;
  undead: boolean;
  details: MonsterDetails;
  /** A boss's longest respawn time as the file lists it, in seconds. Only a rough guide. */
  respawn: number | null;
  /** The best maps first. */
  maps: HuntingGround[];
  quests: { quest: QuestRef; giver: string | null; count: number }[];
  /** Records with the same name and their own numbers, such as a party quest's copy. */
  versions: { id: number; level: number; hp: number; exp: number; maps: MapRef[] }[];
  /** What it turns into when defeated. */
  splits: { id: number; name: string; count: number }[];
  /** Null when the drop table isn't downloaded. */
  drops: MonsterDrops | null;
}

export interface MonsterWhere {
  /** The map portals are counted from, when the minimap was read and matched. */
  from: MapRef | null;
  sheet: MonsterSheet;
}

/** What to bring up in the Lookup tab: one monster, or a search when no monster was matched. */
export interface MonsterLink {
  id: number | null;
  text: string | null;
}

// The map and NPC searches (see src-tauri/src/mapdata/places.rs).

export interface MapQuery {
  /** A map, a street or region, a monster on it or an NPC on it. */
  text: string;
  /** `town` or `field`. */
  kind: string | null;
  region: string | null;
  /** One of `mapAreas`. */
  area: string | null;
  /** Maps with a monster in this range of levels. */
  levelMin: number | null;
  levelMax: number | null;
  /** `level` for the lowest first. By name otherwise. */
  sort: string | null;
  offset: number;
}

export interface MapRow {
  map: MapRef;
  region: string | null;
  isTown: boolean;
  levelRange: [number, number] | null;
  /** Why a search found it, when that isn't its name. */
  note: string | null;
}

export interface MapPage {
  maps: MapRow[];
  total: number;
  /** Nothing matched outright, so these are the closest names. */
  closest: boolean;
}

export interface NpcQuery {
  /** An NPC, a quest they give or a map they stand on. */
  text: string;
  /** `quests` for only those who give some. */
  kind: string | null;
  region: string | null;
  /** One of `mapAreas`. */
  area: string | null;
  offset: number;
}

export interface NpcRow {
  id: number;
  name: string;
  /** The first map they stand on. */
  map: string | null;
  maps: number;
  quests: number;
  /** Why a search found them, when that isn't their name. */
  note: string | null;
}

export interface NpcPage {
  npcs: NpcRow[];
  total: number;
  /** Nothing matched outright, so these are the closest names. */
  closest: boolean;
}

/** All the data says about one NPC. */
export interface NpcSheet {
  id: number;
  name: string;
  /** The closest first. */
  maps: { map: MapRef; hops: number | null; travel: Shortcut | null }[];
  quests: QuestRow[];
  /** Quests someone else gives that go by what shows they are handed in here. */
  takes: QuestRef[];
  /** What the NPC sells, from Leafline's shop file. */
  shop: Shop | null;
}

export interface NpcWhere {
  /** The map portals are counted from, when the minimap was read and matched. */
  from: MapRef | null;
  sheet: NpcSheet;
}

/** What to bring up in the Lookup tab: a map or an NPC. */
export interface PlaceLink {
  mode: "maps" | "npcs";
  id: number;
}

/** What the quest search asks for. */
export interface QuestSearch {
  text: string;
  region: string | null;
  /** `daily`, `weekly` or `repeatable`. */
  repeat: string | null;
  /** Only quests a character of this level can start. */
  atLevel: number | null;
  offset: number;
}

export interface QuestRow {
  id: number;
  name: string;
  region: string | null;
  levelMin: number | null;
  giver: string | null;
  repeat: string | null;
  exp: number;
  mesos: number;
}

export interface QuestPage {
  quests: QuestRow[];
  total: number;
}

/** All the map data says about one quest. */
export interface QuestSheet {
  info: NonNullable<QuestGuide["quest"]>;
  row: QuestRow;
  chain: string | null;
  story: string[];
  /** Quests to finish first. */
  after: QuestRef[];
  next: QuestRef | null;
  objectives: { count: number; guide: ObjectiveGuide }[];
  rewards: {
    id: number;
    name: string;
    count: number;
    /** Always given. Otherwise it is one of a choice, or down to chance. */
    sure: boolean;
  }[];
}

export interface QuestWhere {
  /** The map portals are counted from, when the minimap was read and matched. */
  from: MapRef | null;
  sheet: QuestSheet;
}

/** Gear a build could look for at a level. */
export interface GearSuggestions {
  level: number;
  /** What the ranking counted, the heaviest first. */
  rankedBy: string[];
  slots: {
    slot: string;
    picks: { item: ItemRecord; goal: boolean }[];
    /** How many fit, picked or not. */
    total: number;
  }[];
}

/** Who the gear finder looks for. Whatever is null comes from the character's build. */
export interface GearFor {
  level: number | null;
  job: number | null;
  gender: string | null;
}

/** What the gear finder found, and who for. */
export interface GearFound {
  level: number;
  job: number;
  gender: string | null;
  /** The build whose stats and goals count. With none, only level and job are checked. */
  build: string | null;
  /** Stat requirements were checked. */
  statsKnown: boolean;
  /** Until this level a character wears only what anyone can. */
  firstJobLevel: number;
  gear: GearSuggestions;
}

export interface GearCheck {
  item: GearItem;
  /** The first level the plan can wear it. */
  level: number | null;
  usable: boolean;
}

/** One equipment slot: what is worn, the plan's next goal for it, and a better pick if there is one. */
export interface SlotGear {
  /** As the item data names it: Weapon, Hat, Glove... */
  slot: string;
  wearing: GearPiece | null;
  /** The best the character can wear now, when it beats what they wear. */
  suggested: ItemRecord | null;
  /** `suggested` is one of the plan's goals. */
  suggestedPlanned: boolean;
  /** The next step up, which only a later level can wear: the plan's next goal for the slot, or else the first better piece in the data. */
  next: GearCheck | null;
  /** `next` is one of the plan's goals. */
  planned: boolean;
  /** More goals for the slot after a planned `next`. */
  later: number;
  /** Whether the character, as they are now, meets each piece's level and stat requirements. */
  wearingFit: Fit | null;
  suggestedFit: Fit | null;
  nextFit: Fit | null;
}

/** Whether the character meets an item's requirements now, and what falls short. */
export interface Fit {
  ok: boolean;
  /** Each requirement not met, such as "DEX 35 (you have 28)". */
  short: string[];
}

export interface Outfit {
  level: number;
  /** In the equip window's order. An overall or a shield only when worn or planned. */
  slots: SlotGear[];
}

export interface Expansion {
  steps: LevelStep[];
  problems: BuildProblem[];
  gear: GearCheck[];
  /** Skill levels when the plan ends. */
  skills: SkillLevel[];
}

export interface BuildPlan {
  build: Build;
  expansion: Expansion;
}

export interface JobChoice {
  id: number;
  name: string;
  /** 1 for a first job, 2 for a second. */
  tier: number;
  /** The first job a second job comes from. */
  parent: number | null;
  /** The stat a new build starts by putting AP into. */
  primary: BaseStat | null;
}

export interface SkillInfo {
  id: number;
  name: string;
  jobId: number;
  className: string;
  jobTier: string;
  maxLevel: number;
  passive: boolean;
  description: string | null;
  requiredSkills: SkillLevel[];
  /** What the skill does at each level, from level 1. */
  levelDescriptions: string[];
}

export interface BuildCatalog {
  jobs: JobChoice[];
  skills: SkillInfo[];
}

export interface BuildRules {
  apPerLevel: number;
  /** As a first or second job. The level of an advancement counts as the job before. */
  spPerLevel: number;
  beginnerSpPerLevel: number;
  /** SP from each job advancement. */
  advancementSp: number;
  firstJobLevel: number;
  secondJobLevel: number;
}

/** How a hovered item suits the build. */
export interface ItemFit {
  level: number;
  /** Null when no job flag was read for it. */
  jobOk: boolean | null;
  wearableNow: boolean;
  wearableAt: number | null;
  short: { label: string; amount: number }[];
  /** A requirement could not be read. */
  unread: boolean;
}
