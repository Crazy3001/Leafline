// The website's live reading worker: frames of the shared game window come here (see capture.ts),
// and as often as the player's reading interval asks, one is copied to RGBA and read by the
// program's own readers and session tracker, in WebAssembly (crates/leafline-capture, live.rs).
// Every other frame is closed at once. Nothing read leaves this worker but the readings, which go
// to the page; frames are never kept.

// Built by tools/build-wasm.mjs; not in the repository, so the type check sees it only once built.
// @ts-ignore
import init, * as built from "../capture/wasm/leafline_capture.js";

import type { Analysis, SessionSummary } from "../types";
import { followTrack, frameRgba, type FrameMessage } from "./capture";

interface Live {
  read(rgba: Uint8Array, width: number, height: number, at: number): string;
  summary(): string;
  snapshot(): string;
  set_held(held: boolean): void;
  set_title_model(recognition: Uint8Array): void;
  set_maps(mapsJson: Uint8Array): void;
  set_idle_after(seconds: number): void;
  reset(): void;
}
const wasm = built as { Live: new (idleAfter: number, saved?: string | null) => Live };

export type ToLive =
  | FrameMessage
  | { op: "start"; idleAfter: number; interval: number; saved: string | null; titleModel: string; maps: string[] }
  | { op: "settings"; idleAfter: number; interval: number }
  | { op: "held"; held: boolean }
  | { op: "reset" }
  | { op: "stop" };

export interface LiveRead {
  analysis: Analysis;
  session: SessionSummary;
  hud: boolean;
  ms: number;
}

export type FromLive =
  | { type: "read"; read: LiveRead; width: number; height: number; frames: number; other: boolean }
  /** The session as it was before another character's frames started a new one. */
  | { type: "other"; before: SessionSummary }
  | { type: "summary"; session: SessionSummary }
  | { type: "snapshot"; tracker: string }
  | { type: "ended" }
  | { type: "error"; message: string };

const post = (message: FromLive) => self.postMessage(message);

const ready = (init as () => Promise<unknown>)();
let live: Live | null = null;
let interval = 2000;
let nextRead = 0;
let reads = 0;
let busy = false;
let reading: ReturnType<typeof followTrack> | null = null;

/** Milliseconds since 1970, by this PC's clock. */
const now = () => performance.timeOrigin + performance.now();

async function handle(frame: VideoFrame) {
  try {
    const at = now();
    // One frame at a time: a frame arriving while one is read is let go.
    if (!live || busy || at < nextRead) return;
    busy = true;
    // On a fixed grid, so uneven frame times don't drift the rate down.
    nextRead = nextRead + interval > at ? nextRead + interval : at + interval;
    const { data, width, height } = frameRgba(frame);
    frame.close();
    const before = live.summary();
    const text = live.read(data, width, height, at / 1000);
    busy = false;
    if (text === "null") return;
    const { read, other } = JSON.parse(text) as { read: LiveRead; other: boolean };
    reads++;
    if (other) post({ type: "other", before: JSON.parse(before) as SessionSummary });
    post({ type: "read", read, width, height, frames: reads, other });
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
      post({ type: "summary", session: JSON.parse(live.summary()) as SessionSummary });
      void loadTitleReader(live, data.titleModel, data.maps);
      break;
    case "settings":
      interval = data.interval * 1000;
      live?.set_idle_after(data.idleAfter);
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

/**
 * The minimap title's reader: ocrs's recognition model (this site's own copy, about 10 MB, kept by
 * the browser's cache after the first time) and the known maps (the same community data the rest
 * of the site uses). Until both are in, and if either fails, reading goes on without the map.
 */
async function loadTitleReader(target: Live, modelUrl: string, mapUrls: string[]) {
  try {
    const [model, maps] = await Promise.all([download([modelUrl]), download(mapUrls)]);
    if (live !== target) return;
    target.set_maps(maps);
    target.set_title_model(model);
  } catch (err) {
    post({ type: "error", message: `The map name can't be read: ${err instanceof Error ? err.message : String(err)}` });
  }
}

/** The tracker's state every half minute, so a reload carries on with the session. */
setInterval(() => {
  if (live && reads) post({ type: "snapshot", tracker: live.snapshot() });
}, 30_000);
