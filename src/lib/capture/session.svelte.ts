// The capture test page's side of a shared window: asks the browser for one, hands its frames to
// the worker (capture.worker.ts) and keeps what the worker reports for the page to show. Website
// only; the program's build never loads it.

import type { FromWorker, MinuteRow, Stats, ToWorker } from "./capture.worker";
import type { Measure, ReaderRead } from "./types";
import { canRead, shareWindow, support as shareSupport, type SharedWindow } from "../platform/capture";

declare class PressureObserver {
  constructor(callback: (records: { state: string }[]) => void);
  observe(source: "cpu", options?: { sampleInterval?: number }): Promise<void>;
  disconnect(): void;
}

/** What the browser offers that the test needs, checked by feature rather than by name: reading a
 * shared window (see platform/capture.ts), and saving frames. */
export const support = {
  ...shareSupport,
  folder: typeof window !== "undefined" && "showDirectoryPicker" in window,
  /** Somewhere to save frames when there is no folder to pick: the browser's own storage. */
  ownStorage: typeof navigator !== "undefined" && !!navigator.storage?.getDirectory,
  pressure: typeof window !== "undefined" && "PressureObserver" in window,
};

export { canRead };

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
  /** Frames from an earlier test still kept in the browser's own storage, found on opening. */
  leftover: 0,
  /** The frames were downloaded and then removed from the browser's own storage. */
  cleared: false,
  perSecond: 2,
  savingOn: false,
  endurance: null as { startedAt: number } | null,
  rows: [] as MinuteRow[],
  error: null as string | null,
});

let worker: Worker | null = null;
let shared: SharedWindow | null = null;
let pressure: PressureObserver | null = null;
/** Counts the frames the page hands over, for the page to show. */
let counting: ReturnType<typeof setInterval> | undefined;

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
  const w = startWorker();
  worker = w;
  let got: SharedWindow | null;
  try {
    got = await shareWindow((message, transfer) => w.postMessage(message, transfer), stopSharing);
  } catch (err) {
    capture.error = String(err);
    got = null;
  }
  if (!got) {
    w.terminate();
    if (worker === w) worker = null;
    return;
  }
  shared = got;
  capture.label = got.label;
  capture.surface = got.surface;
  capture.readIn = got.readIn;
  capture.readInWhy = got.readInWhy;
  capture.sharing = true;
  capture.measure = null;
  capture.read = null;
  capture.stats = null;
  capture.delivered = 0;
  counting = setInterval(() => (capture.delivered = shared?.delivered() ?? capture.delivered), 500);
  sendJobs();
  sendSave();
  sendPage();
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

export function stopSharing() {
  if (!capture.sharing && !worker) return;
  send({ op: "stop" });
  shared?.stop();
  shared = null;
  clearInterval(counting);
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
  capture.cleared = false;
  for (const name of names) {
    const file = await (await dir.getFileHandle(name)).getFile();
    const url = URL.createObjectURL(file);
    Object.assign(document.createElement("a"), { href: url, download: name }).click();
    // The browser starts each download from the link before it is let go.
    await new Promise((resolve) => setTimeout(resolve, 250));
    URL.revokeObjectURL(url);
    capture.downloading = { done: capture.downloading.done + 1, total: names.length };
  }
  // Gigabytes of frames shouldn't stay in the browser once they are in Downloads. The last
  // downloads are given time to finish reading their files first.
  await new Promise((resolve) => setTimeout(resolve, CLEAR_AFTER_MS));
  await clearOwnFrames();
  capture.cleared = true;
}

/** How long after the last download starts the kept frames are removed. */
const CLEAR_AFTER_MS = 10_000;

/** Frames an earlier test left in the browser's own storage (a tab closed before downloading),
 * offered for download or deletion when the page opens. */
export async function findLeftoverFrames() {
  if (support.folder || !support.ownStorage) return;
  try {
    const dir = await (await navigator.storage.getDirectory()).getDirectoryHandle(OWN_FOLDER);
    const count = (await ownNames(dir)).length;
    if (count && !capture.folder) {
      capture.folder = dir;
      capture.folderIsOwn = true;
      capture.leftover = count;
    }
  } catch {
    // None kept.
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
  capture.leftover = 0;
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
