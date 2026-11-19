// The capture test page's side of a shared window: asks the browser for one, hands its frames to
// the worker (capture.worker.ts) and keeps what the worker reports for the page to show. Website
// only; the program's build never loads it.

import type { FromWorker, MinuteRow, Stats, ToWorker } from "./capture.worker";
import type { Measure, ReaderRead } from "./types";

declare class MediaStreamTrackProcessor {
  constructor(init: { track: MediaStreamTrack });
  readable: ReadableStream<VideoFrame>;
}
declare class PressureObserver {
  constructor(callback: (records: { state: string }[]) => void);
  observe(source: "cpu", options?: { sampleInterval?: number }): Promise<void>;
  disconnect(): void;
}

/** What the browser offers that the test needs, checked by feature rather than by name. Chrome and
 * Edge on a computer have all of it; Firefox can share and read a window, by another route. */
export const support = {
  share: typeof navigator !== "undefined" && !!navigator.mediaDevices?.getDisplayMedia,
  /** Each frame handed over as the stream delivers it: Chrome and Edge. */
  trackFrames: typeof window !== "undefined" && "MediaStreamTrackProcessor" in window,
  /** Each frame taken from the shared window playing in a video as it arrives: Firefox. */
  videoFrames:
    typeof window !== "undefined" &&
    "VideoFrame" in window &&
    "requestVideoFrameCallback" in HTMLVideoElement.prototype,
  folder: typeof window !== "undefined" && "showDirectoryPicker" in window,
  /** Somewhere to save frames when there is no folder to pick: the browser's own storage. */
  ownStorage: typeof navigator !== "undefined" && !!navigator.storage?.getDirectory,
  pressure: typeof window !== "undefined" && "PressureObserver" in window,
};

/** Whether this browser can read a shared window at all. */
export const canRead = support.share && (support.trackFrames || support.videoFrames);

/** The folder in the browser's own storage that frames go to without a folder picker. */
const OWN_FOLDER = "leafline-capture-frames";

export const capture = $state({
  sharing: false,
  /** The shared surface's name, as the browser gives it. */
  label: "",
  surface: "",
  /** Where frames are read: in the worker (the stream handed over), on the page, or from a
   * video playing the stream (Firefox). */
  readIn: "" as "" | "worker" | "page" | "video",
  /** Why the stream couldn't be handed to the worker, when it couldn't. */
  readInWhy: null as string | null,
  /** Frames the browser delivered to the page, where the page reads them. */
  delivered: 0,
  stats: null as Stats | null,
  /** When the last stats came, by the page's clock. */
  statsAt: 0,
  measure: null as Measure | null,
  read: null as ReaderRead | null,
  measuring: true,
  reading: false,
  folder: null as FileSystemDirectoryHandle | null,
  /** The folder is the browser's own storage (no folder picker), to be downloaded afterwards. */
  folderIsOwn: false,
  /** Frames downloaded so far out of those saved in the browser's own storage. */
  downloading: null as { done: number; total: number } | null,
  perSecond: 2,
  savingOn: false,
  endurance: null as { startedAt: number } | null,
  rows: [] as MinuteRow[],
  error: null as string | null,
});

let worker: Worker | null = null;
let track: MediaStreamTrack | null = null;
let pageReader: ReadableStreamDefaultReader<VideoFrame> | null = null;
let video: HTMLVideoElement | null = null;
let ticker: Worker | null = null;
let pressure: PressureObserver | null = null;

/** How often a hidden page looks for a new frame, where it gets no frame callbacks. */
const HIDDEN_LOOK_MS = 40;

/** Firefox's route: the shared window plays in a video that is never shown, and each new frame
 * becomes a VideoFrame for the worker. The video's frame callback drives it while the page is
 * drawn; a page the browser counts as hidden (minimised, or covered by the game) gets none, so
 * ticks from a worker, which browsers don't slow like a hidden page's timers, keep it going. */
function readFromVideo(stream: MediaStream) {
  const v = document.createElement("video");
  v.muted = true;
  v.playsInline = true;
  v.srcObject = stream;
  video = v;
  let last = -1;
  const take = (mediaTime: number) => {
    if (video !== v || v.readyState < 2 || mediaTime === last) return;
    last = mediaTime;
    let frame: VideoFrame;
    try {
      frame = new VideoFrame(v, { timestamp: Math.round(mediaTime * 1e6) });
    } catch {
      return;
    }
    capture.delivered++;
    send({ op: "frame", frame }, [frame as unknown as Transferable]);
  };
  const onFrame = (_now: number, meta: { mediaTime: number }) => {
    take(meta.mediaTime);
    if (video === v) v.requestVideoFrameCallback(onFrame);
  };
  v.play()
    .then(() => v.requestVideoFrameCallback(onFrame))
    .catch((err) => (capture.error = `The shared window wouldn't play: ${err}`));
  const tick = `setInterval(() => postMessage(0), ${HIDDEN_LOOK_MS});`;
  ticker = new Worker(URL.createObjectURL(new Blob([tick], { type: "text/javascript" })));
  ticker.onmessage = () => {
    if (document.visibilityState === "hidden") take(v.currentTime);
  };
}

const send = (message: ToWorker, transfer: Transferable[] = []) => worker?.postMessage(message, transfer);

function sendPage() {
  send({ op: "page", hidden: document.visibilityState === "hidden", focused: document.hasFocus() });
}

function startWorker(): Worker {
  const w = new Worker(new URL("./capture.worker.ts", import.meta.url), { type: "module" });
  w.onmessage = ({ data }: MessageEvent<FromWorker>) => {
    switch (data.type) {
      case "stats":
        capture.stats = data.stats;
        capture.statsAt = Date.now();
        break;
      case "measure":
        capture.measure = JSON.parse(data.json) as Measure;
        break;
      case "read":
        capture.read = data.read;
        break;
      case "minute":
        capture.rows.push(data.row);
        break;
      case "ended":
        stopSharing();
        break;
      case "error":
        capture.error = data.message;
        break;
    }
  };
  return w;
}

/** Asks the browser for a window to share and starts reading it. */
export async function share() {
  capture.error = null;
  stopSharing();
  let stream: MediaStream;
  try {
    stream = await navigator.mediaDevices.getDisplayMedia({
      // The window's own size: no scaling to a default resolution.
      video: { displaySurface: "window", frameRate: { ideal: 60 }, width: { ideal: 4096 }, height: { ideal: 2304 } },
      audio: false,
      // Chrome's options for what the picker offers.
      ...({ selfBrowserSurface: "exclude", surfaceSwitching: "include", preferCurrentTab: false } as object),
    });
  } catch (err) {
    capture.error = err instanceof Error && err.name === "NotAllowedError" ? null : String(err);
    return;
  }
  track = stream.getVideoTracks()[0];
  capture.label = track.label;
  capture.surface = String((track.getSettings() as { displaySurface?: string }).displaySurface ?? "");
  capture.sharing = true;
  capture.measure = null;
  capture.read = null;
  capture.stats = null;
  worker = startWorker();
  sendJobs();
  sendSave();
  sendPage();
  track.addEventListener("ended", stopSharing);

  // The stream itself handed to the worker where the browser allows it, else read here and each
  // frame handed over. Either way the frames come as the stream delivers them, not on a timer.
  if (!support.trackFrames) {
    // Firefox: no stream reader at all, so the frames come from a video playing the stream.
    capture.readIn = "video";
    capture.readInWhy = null;
    readFromVideo(stream);
  } else {
    try {
      send({ op: "track", track }, [track as unknown as Transferable]);
      capture.readIn = "worker";
      capture.readInWhy = null;
    } catch (err) {
      capture.readIn = "page";
      capture.readInWhy = err instanceof Error ? `${err.name}: ${err.message}` : String(err);
      const reader = new MediaStreamTrackProcessor({ track }).readable.getReader();
      pageReader = reader;
      void (async () => {
        for (;;) {
          const { value, done } = await reader.read();
          if (done || !value) break;
          capture.delivered++;
          send({ op: "frame", frame: value }, [value as unknown as Transferable]);
        }
        stopSharing();
      })();
    }
  }
  await startPressure();
}

async function startPressure() {
  if (support.pressure && !pressure) {
    try {
      pressure = new PressureObserver((records) => {
        const last = records[records.length - 1];
        if (last) send({ op: "pressure", state: last.state });
      });
      await pressure.observe("cpu", { sampleInterval: 1000 });
    } catch {
      pressure = null;
    }
  }
}

/** The shared track, for checking the page by script. */
export const sharedTrack = () => track;

export function stopSharing() {
  if (!capture.sharing && !worker) return;
  send({ op: "stop" });
  void pageReader?.cancel();
  pageReader = null;
  ticker?.terminate();
  ticker = null;
  if (video) {
    video.pause();
    video.srcObject = null;
    video = null;
  }
  track?.stop();
  track = null;
  worker?.terminate();
  worker = null;
  pressure?.disconnect();
  pressure = null;
  capture.sharing = false;
  capture.readIn = "";
  capture.savingOn = false;
  capture.endurance = null;
}

export function sendJobs() {
  send({ op: "jobs", measure: capture.measuring, read: capture.reading || !!capture.endurance });
}

function sendSave() {
  send({ op: "save", dir: capture.savingOn ? capture.folder : null, perSecond: capture.perSecond });
}

/** Lets the player pick the folder frames are saved to. */
export async function pickFolder() {
  try {
    const picker = (window as unknown as {
      showDirectoryPicker(options: object): Promise<FileSystemDirectoryHandle>;
    }).showDirectoryPicker;
    capture.folder = await picker.call(window, { id: "leafline-capture-test", mode: "readwrite" });
    capture.folderIsOwn = false;
  } catch (err) {
    if (!(err instanceof Error && err.name === "AbortError")) capture.error = String(err);
  }
}

/** Where there is no folder picker (Firefox): frames are kept in the browser's own storage for
 * this site, then downloaded one by one into the Downloads folder. */
export async function useOwnStorage() {
  try {
    const root = await navigator.storage.getDirectory();
    capture.folder = await root.getDirectoryHandle(OWN_FOLDER, { create: true });
    capture.folderIsOwn = true;
  } catch (err) {
    capture.error = `This browser wouldn't keep frames for the site: ${err}`;
  }
}

async function ownNames(dir: FileSystemDirectoryHandle): Promise<string[]> {
  const names: string[] = [];
  for await (const name of (dir as unknown as { keys(): AsyncIterable<string> }).keys()) {
    if (name.startsWith("web-") && name.endsWith(".png")) names.push(name);
  }
  return names.sort();
}

/** Downloads the frames kept in the browser's own storage, oldest first. */
export async function downloadOwnFrames() {
  const dir = capture.folderIsOwn ? capture.folder : null;
  if (!dir) return;
  const names = await ownNames(dir);
  capture.downloading = { done: 0, total: names.length };
  for (const name of names) {
    const file = await (await dir.getFileHandle(name)).getFile();
    const url = URL.createObjectURL(file);
    Object.assign(document.createElement("a"), { href: url, download: name }).click();
    // The browser starts each download from the link before it is let go.
    await new Promise((resolve) => setTimeout(resolve, 250));
    URL.revokeObjectURL(url);
    capture.downloading = { done: capture.downloading.done + 1, total: names.length };
  }
}

/** Deletes the frames kept in the browser's own storage. */
export async function clearOwnFrames() {
  if (capture.savingOn) setSaving(false);
  try {
    await (await navigator.storage.getDirectory()).removeEntry(OWN_FOLDER, { recursive: true });
  } catch {
    // Already gone.
  }
  capture.folder = null;
  capture.folderIsOwn = false;
  capture.downloading = null;
}

export function setSaving(on: boolean, folder?: FileSystemDirectoryHandle) {
  if (folder) capture.folder = folder;
  capture.savingOn = on && !!capture.folder;
  sendSave();
}

export function setPerSecond(perSecond: number) {
  capture.perSecond = perSecond;
  sendSave();
}

export function setEndurance(on: boolean) {
  capture.endurance = on ? { startedAt: Date.now() } : null;
  if (on) capture.rows = [];
  send({ op: "endurance", on });
  sendJobs();
}

/** The endurance log as CSV, to keep or paste. */
export function enduranceCsv(): string {
  const head = "minute,hidden,unfocused,frames,frames_per_second,reads,reads_per_second,read_ms_mean,read_ms_p95,copy_ms_mean,busy,cpu_pressure,saved";
  const lines = capture.rows.map((r) =>
    [
      r.minute,
      r.hidden.toFixed(2),
      r.unfocused.toFixed(2),
      r.frames,
      (r.frames / 60).toFixed(1),
      r.reads,
      (r.reads / 60).toFixed(2),
      r.readMean.toFixed(1),
      r.readP95.toFixed(1),
      r.copyMean.toFixed(1),
      r.busy.toFixed(3),
      r.pressure,
      r.saved,
    ].join(","),
  );
  return [head, ...lines].join("\n");
}

if (typeof document !== "undefined") {
  document.addEventListener("visibilitychange", sendPage);
  window.addEventListener("focus", sendPage);
  window.addEventListener("blur", sendPage);
}
