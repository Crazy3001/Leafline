// The capture test page's worker. Every frame the shared window delivers comes here, driven by
// the stream itself (no timers): it is counted, and as often as each job needs, copied to RGBA
// and measured against the pattern (part A), read by the website's readers (part C) or saved to
// the folder the player picked (part B). Frames are closed at once and never leave this PC.

// Built by tools/build-wasm.mjs; not in the repository, so the type check sees it only once built.
// @ts-ignore
import init, * as built from "./wasm/leafline_capture.js";

import type { ReaderRead } from "./types";

const wasm = built as {
  measure(rgba: Uint8Array, width: number, height: number, hintX: number, hintY: number): string;
  Readers: new () => { read(rgba: Uint8Array, w: number, h: number, at: number, fullEvery: number): string };
};

// Chrome's and Edge's stream reader, not yet in TypeScript's library.
declare class MediaStreamTrackProcessor {
  constructor(init: { track: MediaStreamTrack });
  readable: ReadableStream<VideoFrame>;
}

export type ToWorker =
  | { op: "track"; track: MediaStreamTrack }
  | { op: "frame"; frame: VideoFrame }
  | { op: "jobs"; measure: boolean; read: boolean }
  | { op: "save"; dir: FileSystemDirectoryHandle | null; perSecond: number }
  | { op: "endurance"; on: boolean }
  | { op: "page"; hidden: boolean; focused: boolean }
  | { op: "pressure"; state: string }
  | { op: "stop" };

/** One minute of the endurance log. */
export interface MinuteRow {
  minute: number;
  /** Share of the minute's frames that arrived with the page hidden, and with it unfocused. */
  hidden: number;
  unfocused: number;
  frames: number;
  reads: number;
  readMean: number;
  readP95: number;
  copyMean: number;
  /** Share of the minute this worker spent copying and reading. */
  busy: number;
  pressure: string;
  saved: number;
}

export interface Stats {
  frames: number;
  fps: number;
  format: string | null;
  codedWidth: number;
  codedHeight: number;
  displayWidth: number;
  displayHeight: number;
  colorSpace: string;
  saved: number;
  dropped: number;
  saving: number;
  saveProblem: string | null;
  readMean: number | null;
}

export type FromWorker =
  | { type: "ready" }
  | { type: "stats"; stats: Stats }
  | { type: "measure"; json: string }
  | { type: "read"; read: ReaderRead }
  | { type: "minute"; row: MinuteRow }
  | { type: "ended" }
  | { type: "error"; message: string };

const post = (message: FromWorker) => self.postMessage(message);

const ready = (init as () => Promise<unknown>)();
let readers: InstanceType<typeof wasm.Readers> | null = null;

const jobs = { measure: false, read: false };
const MEASURE_EVERY = 500;
const READ_EVERY = 250;
/** Everything but the Pickup Log is read once a second, as the program does. */
const FULL_EVERY = 1;
let nextMeasure = 0;
let nextRead = 0;
let hint: [number, number] = [-1, -1];

let saveTo: FileSystemDirectoryHandle | null = null;
let savePeriod = 1000;
let nextSave = 0;
let saving = 0;
/** Frames being encoded and written at most at once; more are dropped and counted. */
const MOST_SAVING = 3;

const page = { hidden: false, focused: true, pressure: "unknown" };

let frames = 0;
let saved = 0;
let dropped = 0;
let saveProblem: string | null = null;
let format: string | null = null;
let size = { codedWidth: 0, codedHeight: 0, displayWidth: 0, displayHeight: 0 };
let colorSpace = "";
let secondFrames = 0;
let secondStart = 0;
let fps = 0;
const recentReads: number[] = [];

let endurance: { start: number; minute: number; row: Accumulating } | null = null;
interface Accumulating {
  frames: number;
  hidden: number;
  unfocused: number;
  reads: number[];
  copies: number[];
  busy: number;
  pressure: Set<string>;
  saved: number;
}
const fresh = (): Accumulating => ({
  frames: 0,
  hidden: 0,
  unfocused: 0,
  reads: [],
  copies: [],
  busy: 0,
  pressure: new Set(),
  saved: 0,
});

/** The next slot on a fixed grid `every` ms apart: frames arrive at uneven times, and spacing jobs
 * from each frame's time would drift below the rate. Starts again from `at` after falling behind. */
const nextSlot = (slot: number, at: number, every: number) => (slot + every > at ? slot + every : at + every);

/** Milliseconds since 1970, by this PC's clock, which the program's names use too. */
const now = () => performance.timeOrigin + performance.now();

let canvas: OffscreenCanvas | null = null;
let context: OffscreenCanvasRenderingContext2D | null = null;

/** The frame as RGBA bytes, the colour conversion done the way any page reading it would. */
function rgba(frame: VideoFrame): { data: Uint8Array; width: number; height: number } {
  const width = frame.displayWidth;
  const height = frame.displayHeight;
  if (!canvas || canvas.width !== width || canvas.height !== height) {
    canvas = new OffscreenCanvas(width, height);
    context = canvas.getContext("2d", { willReadFrequently: true, alpha: false });
  }
  context!.drawImage(frame, 0, 0);
  const image = context!.getImageData(0, 0, width, height);
  return { data: new Uint8Array(image.data.buffer), width, height };
}

async function save(bitmap: ImageBitmap, at: number) {
  saving++;
  try {
    const out = new OffscreenCanvas(bitmap.width, bitmap.height);
    out.getContext("2d")!.drawImage(bitmap, 0, 0);
    bitmap.close();
    const blob = await out.convertToBlob({ type: "image/png" });
    const file = await saveTo!.getFileHandle(`web-${Math.round(at)}.png`, { create: true });
    const writable = await file.createWritable();
    await writable.write(blob);
    await writable.close();
    saved++;
    if (endurance) endurance.row.saved++;
  } catch (err) {
    dropped++;
    saveProblem = err instanceof Error ? err.message : String(err);
  } finally {
    saving--;
  }
}

function endMinute(at: number) {
  if (!endurance) return;
  const minute = Math.floor((at - endurance.start) / 60000);
  if (minute === endurance.minute) return;
  const r = endurance.row;
  const sorted = [...r.reads].sort((a, b) => a - b);
  const mean = (list: number[]) => (list.length ? list.reduce((a, b) => a + b, 0) / list.length : 0);
  post({
    type: "minute",
    row: {
      minute: endurance.minute + 1,
      hidden: r.frames ? r.hidden / r.frames : 0,
      unfocused: r.frames ? r.unfocused / r.frames : 0,
      frames: r.frames,
      reads: r.reads.length,
      readMean: mean(r.reads),
      readP95: sorted.length ? sorted[Math.min(sorted.length - 1, Math.floor(sorted.length * 0.95))] : 0,
      copyMean: mean(r.copies),
      busy: r.busy / 60000,
      pressure: [...r.pressure].join(" ") || "unknown",
      saved: r.saved,
    },
  });
  // A minute with no frames at all still gets its row: that is the finding.
  endurance.minute++;
  endurance.row = fresh();
  endMinute(at);
}

async function handle(frame: VideoFrame) {
  const at = now();
  try {
    frames++;
    secondFrames++;
    format = frame.format;
    size = {
      codedWidth: frame.codedWidth,
      codedHeight: frame.codedHeight,
      displayWidth: frame.displayWidth,
      displayHeight: frame.displayHeight,
    };
    const cs = frame.colorSpace;
    colorSpace = [cs.matrix, cs.primaries, cs.transfer, cs.fullRange == null ? null : cs.fullRange ? "full" : "limited"]
      .filter(Boolean)
      .join(" ");
    if (endurance) {
      endMinute(at);
      const r = endurance.row;
      r.frames++;
      if (page.hidden) r.hidden++;
      if (!page.focused) r.unfocused++;
      r.pressure.add(page.pressure);
    }

    if (saveTo && at >= nextSave) {
      nextSave = (Math.floor(at / savePeriod) + 1) * savePeriod;
      if (saving >= MOST_SAVING) dropped++;
      else void save(await createImageBitmap(frame), at);
    }

    const measureNow = jobs.measure && at >= nextMeasure;
    const readNow = jobs.read && at >= nextRead;
    if (measureNow || readNow) {
      await ready;
      const copyStart = performance.now();
      const { data, width, height } = rgba(frame);
      const copied = performance.now() - copyStart;
      frame.close();
      let readMs = 0;
      if (measureNow) {
        nextMeasure = nextSlot(nextMeasure, at, MEASURE_EVERY);
        const json = wasm.measure(data, width, height, hint[0], hint[1]);
        const found = JSON.parse(json).found;
        hint = found ? [found.x, found.y] : [-1, -1];
        post({ type: "measure", json });
      }
      if (readNow) {
        nextRead = nextSlot(nextRead, at, READ_EVERY);
        readers ??= new wasm.Readers();
        const started = performance.now();
        const read = JSON.parse(readers.read(data, width, height, at / 1000, FULL_EVERY)) as ReaderRead | null;
        readMs = performance.now() - started;
        recentReads.push(readMs);
        if (recentReads.length > 20) recentReads.shift();
        if (read) post({ type: "read", read });
        if (endurance) {
          endurance.row.reads.push(readMs);
          endurance.row.copies.push(copied);
        }
      }
      if (endurance) endurance.row.busy += copied + readMs;
    }
  } catch (err) {
    post({ type: "error", message: err instanceof Error ? err.message : String(err) });
  } finally {
    // Closing twice is harmless; leaving a frame open stalls the stream.
    frame.close();
  }
  if (at - secondStart >= 1000) {
    fps = (secondFrames * 1000) / (at - secondStart || 1000);
    secondFrames = 0;
    secondStart = at;
    post({
      type: "stats",
      stats: {
        frames,
        fps,
        format,
        ...size,
        colorSpace,
        saved,
        dropped,
        saving,
        saveProblem,
        readMean: recentReads.length ? recentReads.reduce((a, b) => a + b, 0) / recentReads.length : null,
      },
    });
  }
}

let reading: ReadableStreamDefaultReader<VideoFrame> | null = null;

async function follow(track: MediaStreamTrack) {
  reading = new MediaStreamTrackProcessor({ track }).readable.getReader();
  for (;;) {
    const { value, done } = await reading.read();
    if (done || !value) break;
    await handle(value);
  }
  post({ type: "ended" });
}

self.onmessage = ({ data }: MessageEvent<ToWorker>) => {
  switch (data.op) {
    case "track":
      void follow(data.track).catch((err) => post({ type: "error", message: String(err) }));
      break;
    case "frame":
      void handle(data.frame);
      break;
    case "jobs":
      jobs.measure = data.measure;
      jobs.read = data.read;
      if (!data.read) readers = null;
      break;
    case "save":
      saveTo = data.dir;
      savePeriod = 1000 / Math.max(1, Math.min(4, data.perSecond));
      nextSave = 0;
      saveProblem = null;
      break;
    case "endurance":
      endurance = data.on ? { start: now(), minute: 0, row: fresh() } : null;
      break;
    case "page":
      page.hidden = data.hidden;
      page.focused = data.focused;
      break;
    case "pressure":
      page.pressure = data.state;
      break;
    case "stop":
      void reading?.cancel();
      reading = null;
      break;
  }
};

post({ type: "ready" });
