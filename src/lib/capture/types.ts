// What the capture test's Rust sends back (crates/leafline-core/src/capturetest), as JSON.

export type Rgb = [number, number, number];

/** The pattern window's size and scale, from its code strip. */
export interface WindowInfo {
  width: number;
  height: number;
  scaleMilli: number;
}

export interface ChannelError {
  mean: number;
  largest: number;
}

export type Part = "code" | "greyRamp" | "colourRamps" | "swatches" | "lines" | "glyphs";

export interface Found {
  x: number;
  y: number;
  window: WindowInfo;
  channels: [ChannelError, ChannelError, ChannelError];
  parts: { part: Part; channels: [ChannelError, ChannelError, ChannelError] }[];
  ramp: { black: number; white: number; largest: number; squeezed: boolean };
  swatches: { name: string; expected: Rgb; seen: Rgb; off: number }[];
  lines: { colour: Rgb; ground: Rgb; vertical: boolean; seen: Rgb; off: number; bleed: number }[];
  bleed: { largest: number; typical: number };
  glyphs: {
    name: string;
    readBy: "questReader" | "pickupReader" | "standIn";
    expected: string;
    read: string | null;
    fontPixels: number;
    flipped: number;
  }[];
}

/** A frame measured against the pattern. */
export interface Measure {
  frameWidth: number;
  frameHeight: number;
  found: Found | null;
  problem: string | null;
}

/** The program's frame recorder (part B). */
export interface RecorderStatus {
  recording: boolean;
  folder: string | null;
  perSecond: number;
  window: string | null;
  saved: number;
  missed: number;
  problem: string | null;
}

export interface ReaderScore {
  reader: string;
  gate: boolean;
  agree: number;
  differ: number;
  onlyProgram: number;
  onlyWebsite: number;
  neither: number;
  share: number | null;
  passes: boolean | null;
  examples: { programFrame: string; websiteFrame: string; program: string | null; website: string | null }[];
}

/** Part B's comparison of the program's and the website's frames. */
export interface Agreement {
  programFrames: number;
  websiteFrames: number;
  pairs: number;
  websiteOffset: [number, number] | null;
  websiteResized: boolean;
  readers: ReaderScore[];
  pickup: { programLines: number; websiteLines: number; matched: number; share: number | null; passes: boolean | null };
  passes: boolean;
}

/** One frame read by the website's readers (part C). */
export interface ReaderRead {
  full: boolean;
  ms: number;
  timings: { stage: string; ms: number }[];
  hud: boolean;
  level: number | null;
  expPercent: number | null;
  pickupLog: boolean;
  pickupLines: number;
}

/** The pattern's size in screen pixels. */
export const PATTERN = { width: 1280, height: 720 };

export const percent = (share: number | null | undefined, digits = 1) =>
  share == null ? "–" : `${(share * 100).toFixed(digits)}%`;

export const rgb = (c: Rgb) => `rgb(${c[0]}, ${c[1]}, ${c[2]})`;
