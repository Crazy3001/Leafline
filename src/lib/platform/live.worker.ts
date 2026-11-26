// The website's live reading worker: frames of the shared game window come here (see capture.ts),
// and as often as the player's reading interval asks, one is copied to RGBA and read by the
// program's own readers and session tracker, in WebAssembly (crates/leafline-capture, live.rs).
// Every other frame is closed at once. Nothing read leaves this worker but the readings, which go
// to the page; frames are never kept.

// Built by tools/build-wasm.mjs; not in the repository, so the type check sees it only once built.
// @ts-ignore
import init, * as built from "../capture/wasm/leafline_capture.js";

import type { Analysis, ItemTooltip, SessionSummary, Sight } from "../types";
import { followTrack, frameRgba, type FrameMessage } from "./capture";

interface Live {
  read(rgba: Uint8Array, width: number, height: number, at: number): string;
  look(rgba: Uint8Array, width: number, height: number, at: number): string;
  summary(): string;
  snapshot(): string;
  set_held(held: boolean): void;
  set_text_model(recognition: Uint8Array): void;
  set_data(file: string, json: Uint8Array): void;
  set_idle_after(seconds: number): void;
  set_pickup_log(on: boolean): void;
  set_item_tooltips(on: boolean): void;
  take_items(): string;
  set_windows(character: boolean, bag: boolean, questLog: boolean): void;
  take_windows(): string;
  reset(): void;
}
const wasm = built as { Live: new (idleAfter: number, saved?: string | null) => Live };

export type ToLive =
  | FrameMessage
  | ({ op: "start"; saved: string | null; textModel: string; dataSources: string[] } & ReadingChoice)
  | ({ op: "settings" } & ReadingChoice)
  | { op: "held"; held: boolean }
  | { op: "reset" }
  | { op: "stop" };

/** What the player chose in Settings about reading. */
export interface ReadingChoice {
  interval: number;
  idleAfter: number;
  /** Count the Pickup Log's lines. */
  pickupLog: boolean;
  /** Read an item tooltip the mouse rests on. */
  itemTooltips: boolean;
  /** Update the build from the stat and skill windows while they're open. */
  characterWindows: boolean;
  /** Check the inventory tab that's open (sell or keep). */
  bagCheck: boolean;
  /** Keep the quest log from the Quest window while it's open. */
  questWindow: boolean;
}

/** What the game's windows gave at a full read (crates/leafline-capture, live.rs WindowReads),
 *  for the page to hand to the records: each read once its window has held still. */
export interface WindowReads {
  character?: unknown;
  bag?: unknown;
  questLog?: { name: string; reading: unknown; fresh: boolean };
}

export interface LiveRead {
  analysis: Analysis;
  session: SessionSummary;
  /** What the HUD shows of the character, for the records to remember between visits. */
  seen: unknown | null;
  /** What each reader has read, and what was left out. */
  sight: Sight;
  hud: boolean;
  ms: number;
}

export type FromLive =
  | { type: "read"; read: LiveRead; width: number; height: number; frames: number; other: boolean; dark: boolean; still: boolean }
  /** The session as it was before another character's frames started a new one. */
  | { type: "other"; before: SessionSummary }
  | { type: "summary"; session: SessionSummary }
  /** The item tooltips read from a held hover, when they change: the last ones stay until then. */
  | { type: "items"; tooltips: ItemTooltip[] }
  /** A window held open was read. */
  | { type: "windows"; reads: WindowReads }
  | { type: "snapshot"; tracker: string }
  | { type: "ended" }
  | { type: "error"; message: string };

const post = (message: FromLive) => self.postMessage(message);

const ready = (init as () => Promise<unknown>)();
let live: Live | null = null;
let interval = 2000;
let nextRead = 0;
/** Between full reads the Pickup Log alone is looked at this often, as the program does
 *  (src-tauri/src/app/live.rs, PICKUP_EVERY): a line stays readable for only a second or two. */
const LOOK_EVERY = 250;
let nextLook = 0;
let reads = 0;
let busy = false;
let reading: ReturnType<typeof followTrack> | null = null;

/** Milliseconds since 1970, by this PC's clock. */
const now = () => performance.timeOrigin + performance.now();

/** The last full read's picture signature, and how many full reads in a row have had it. */
let lastSig = -1;
let stillFor = 0;

/**
 * About 4,000 pixels spread over a picture: whether it is (nearly) all black, and a signature
 * that stays the same only while the picture does. Some browsers send black for a game in Full
 * Screen, and Firefox keeps sending the first picture it took of a game's window.
 */
function glance(rgba: Uint8Array): { dark: boolean; sig: number } {
  const step = Math.max(4, Math.floor(rgba.length / 4 / 4000) * 4);
  let sum = 0;
  let n = 0;
  let sig = 0;
  for (let i = 0; i + 2 < rgba.length; i += step) {
    sum += Math.max(rgba[i], rgba[i + 1], rgba[i + 2]);
    sig = (Math.imul(sig, 31) + rgba[i] * 65536 + rgba[i + 1] * 256 + rgba[i + 2]) | 0;
    n++;
  }
  return { dark: n > 0 && sum / n < 8, sig };
}

async function handle(frame: VideoFrame) {
  try {
    const at = now();
    // One frame at a time: a frame arriving while one is read is let go.
    if (!live || busy) return;
    const full = at >= nextRead;
    if (!full && at < nextLook) return;
    busy = true;
    // On fixed grids, so uneven frame times don't drift the rates down.
    nextLook = nextLook + LOOK_EVERY > at ? nextLook + LOOK_EVERY : at + LOOK_EVERY;
    if (full) nextRead = nextRead + interval > at ? nextRead + interval : at + interval;
    const { data, width, height } = frameRgba(frame);
    frame.close();
    if (!full) {
      const looked = live.look(data, width, height, at / 1000);
      const items = live.take_items();
      busy = false;
      if (looked) post({ type: "summary", session: JSON.parse(looked) as SessionSummary });
      if (items) post({ type: "items", tooltips: JSON.parse(items) as ItemTooltip[] });
      return;
    }
    const before = live.summary();
    const text = live.read(data, width, height, at / 1000);
    const items = live.take_items();
    const windows = live.take_windows();
    busy = false;
    if (items) post({ type: "items", tooltips: JSON.parse(items) as ItemTooltip[] });
    if (windows) post({ type: "windows", reads: JSON.parse(windows) as WindowReads });
    if (text === "null") return;
    const { read, other } = JSON.parse(text) as { read: LiveRead; other: boolean };
    reads++;
    if (other) post({ type: "other", before: JSON.parse(before) as SessionSummary });
    const look = glance(data);
    stillFor = look.sig === lastSig ? stillFor + 1 : 0;
    lastSig = look.sig;
    post({ type: "read", read, width, height, frames: reads, other, dark: !read.hud && look.dark, still: !read.hud && stillFor >= 3 });
  } catch (err) {
    busy = false;
    post({ type: "error", message: err instanceof Error ? err.message : String(err) });
  } finally {
    // Closing twice is harmless; leaving a frame open stalls the stream.
    frame.close();
  }
}

self.onmessage = async ({ data }: MessageEvent<ToLive>) => {
  switch (data.op) {
    case "start":
      await ready;
      interval = data.interval * 1000;
      live = new wasm.Live(data.idleAfter, data.saved);
      live.set_pickup_log(data.pickupLog);
      live.set_item_tooltips(data.itemTooltips);
      live.set_windows(data.characterWindows, data.bagCheck, data.questWindow);
      post({ type: "summary", session: JSON.parse(live.summary()) as SessionSummary });
      void loadTextReading(live, data.textModel, data.dataSources);
      break;
    case "settings":
      interval = data.interval * 1000;
      live?.set_idle_after(data.idleAfter);
      live?.set_pickup_log(data.pickupLog);
      live?.set_item_tooltips(data.itemTooltips);
      live?.set_windows(data.characterWindows, data.bagCheck, data.questWindow);
      if (live) {
        const items = live.take_items();
        if (items) post({ type: "items", tooltips: JSON.parse(items) as ItemTooltip[] });
      }
      break;
    case "track":
      reading = followTrack(data.track, handle);
      void reading.done.then(() => post({ type: "ended" }), (err) => post({ type: "error", message: String(err) }));
      break;
    case "frame":
      void handle(data.frame);
      break;
    case "held":
      live?.set_held(data.held);
      if (live) post({ type: "summary", session: JSON.parse(live.summary()) as SessionSummary });
      break;
    case "reset":
      live?.reset();
      if (live) post({ type: "summary", session: JSON.parse(live.summary()) as SessionSummary });
      break;
    case "stop":
      reading?.cancel();
      reading = null;
      if (live) post({ type: "snapshot", tracker: live.snapshot() });
      break;
  }
};

/** The bytes at the first of `urls` that answers. */
async function download(urls: string[]): Promise<Uint8Array> {
  let last: unknown = null;
  for (const url of urls) {
    try {
      const response = await fetch(url);
      if (response.ok) return new Uint8Array(await response.arrayBuffer());
      last = new Error(`${url}: ${response.status}`);
    } catch (err) {
      last = err;
    }
  }
  throw last;
}

/** The reference data files the readers settle against, the maps first: without them no map is
 *  read, while the others only steady the Quest Helper's titles and name the Pickup Log's items. */
const DATA_FILES = ["maps.json", "quests.json", "items.json", "monsters.json"];

/**
 * What reading text needs: ocrs's recognition model (this site's own copy, about 10 MB, kept by the
 * browser's cache after the first time) for the map, the character's name and job and the Quest
 * Helper, and the reference data (the same community data the rest of the site uses). Until they
 * are in, and if one fails, reading goes on without what it serves.
 */
async function loadTextReading(target: Live, modelUrl: string, sources: string[]) {
  const files = DATA_FILES.map((file) => download(sources.map((source) => source + file)));
  try {
    target.set_text_model(await download([modelUrl]));
  } catch (err) {
    post({ type: "error", message: `Text in the game can't be read: ${err instanceof Error ? err.message : String(err)}` });
  }
  for (const [i, file] of DATA_FILES.entries()) {
    try {
      const bytes = await files[i];
      if (live !== target) return;
      target.set_data(file, bytes);
    } catch (err) {
      const what = file === "maps.json" ? "The map name" : "Quest and item names";
      post({ type: "error", message: `${what} can't be settled: ${err instanceof Error ? err.message : String(err)}` });
    }
  }
}

/** The tracker's state every half minute, so a reload carries on with the session. */
setInterval(() => {
  if (live && reads) post({ type: "snapshot", tracker: live.snapshot() });
}, 30_000);
