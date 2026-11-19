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

/** What the browser offers that the test needs: Chrome and Edge on a computer have all of it. */
export const support = {
  share: typeof navigator !== "undefined" && !!navigator.mediaDevices?.getDisplayMedia,
  folder: typeof window !== "undefined" && "showDirectoryPicker" in window,
  pressure: typeof window !== "undefined" && "PressureObserver" in window,
};

export const capture = $state({
  sharing: false,
  /** The shared surface's name, as the browser gives it. */
  label: "",
  surface: "",
  /** Where frames are read: in the worker (the stream handed over) or on the page. */
  readIn: "" as "" | "worker" | "page",
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
  perSecond: 2,
  savingOn: false,
  endurance: null as { startedAt: number } | null,
  rows: [] as MinuteRow[],
  error: null as string | null,
});

let worker: Worker | null = null;
let track: MediaStreamTrack | null = null;
let pageReader: ReadableStreamDefaultReader<VideoFrame> | null = null;
let pressure: PressureObserver | null = null;

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
  } catch (err) {
    if (!(err instanceof Error && err.name === "AbortError")) capture.error = String(err);
  }
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
