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

export interface OverlayStatus {
  visible: boolean;
  clickThrough: boolean;
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
  rejected: { exp: number; mesos: number };
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
