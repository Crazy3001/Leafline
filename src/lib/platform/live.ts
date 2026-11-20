// The website's live reading, behind the same backend members the program's Now uses (startLive,
// stopLive, onSession…): the player shares the game window, its frames are read in a worker
// (live.worker.ts) by the program's own readers and session tracker, and the readings go out as
// the program's "live", "analysis" and "session" events. The session saves itself to the site's
// records as it goes, as the program's does (src-tauri/src/app/live.rs: after 5 active minutes,
// then every half minute; a 30-minute break or another character starts a new one), and the
// tracker's state is kept in this browser so a reload carries on with it. Nothing read leaves
// this PC.

import { base } from "$app/paths";
import type { Analysis, LiveStatus, SessionSummary } from "../types";
import { emit } from "./bus";
import { canRead, cantRead, shareWindow, type SharedWindow } from "./capture";
import type { FromLive, ToLive } from "./live.worker";
import { invoke } from "./storage.svelte";

/** Active seconds before a session is worth keeping (a quick login to sell isn't training). */
const SAVE_AFTER_ACTIVE = 300;
const SAVE_EVERY_MS = 30_000;
/** A break this long with no EXP ends the session; the next gain starts a new one. */
const BREAK_ENDS_SESSION_MS = 30 * 60_000;
/** The known maps, for settling the minimap title: Leafline's mirror of OSMS first, OSMS second,
 *  as the site's own data download goes (crates/leafline-core/src/mapdata/fetch.rs). */
const MAP_LIST_URLS = [
  "https://raw.githubusercontent.com/Crazy3001/Leafline-data/main/osms/v1/maps.json",
  "https://osmsdataexplorer.com/data/api/v1/maps.json",
];
/** Where the tracker's state is kept between visits, in this site's own browser storage. */
const RECOVERY_FILE = "live-session.json";
const RECOVERY_VERSION = 1;

let status: LiveStatus = { state: "off" };
let analysis: Analysis | null = null;
let session: SessionSummary | null = null;
let worker: Worker | null = null;
let shared: SharedWindow | null = null;
let held = false;
/** The session's log id once saved, and when it was last saved. */
let keeper: { id: number | null; savedAt: number } = { id: null, savedAt: 0 };
/** The session's active time when it last grew, and when that was. */
let lastActive = { seconds: 0, at: Date.now() };
/** The name to save sessions under: the website can't read the HUD's name yet. */
let character: () => string | null = () => null;

export const liveCanRead = canRead;
export const liveCantRead = cantRead;

/** Where the player picked a character (Now's character picker), to save sessions under it. */
export function setLiveCharacter(get: () => string | null) {
  character = get;
}

function setStatus(next: LiveStatus) {
  status = next;
  void emit("live", next);
}

export const liveStatus = () => status;
export const liveReadings = () => ({ analysis, session });

function publishSession(next: SessionSummary) {
  session = next;
  void emit("session", next);
}

async function settings(): Promise<{ interval: number; idleAfter: number }> {
  try {
    const s = await invoke<{ live?: { interval?: number; pauseAfter?: number } }>("get_settings");
    return { interval: s.live?.interval ?? 2, idleAfter: s.live?.pauseAfter ?? 15 };
  } catch {
    return { interval: 2, idleAfter: 15 };
  }
}

async function recoveryFile(create: boolean): Promise<FileSystemFileHandle | null> {
  try {
    return await (await navigator.storage.getDirectory()).getFileHandle(RECOVERY_FILE, { create });
  } catch {
    return null;
  }
}

async function writeRecovery(tracker: string) {
  const file = await recoveryFile(true);
  if (!file) return;
  try {
    const text = `{"version":${RECOVERY_VERSION},"savedAt":${Date.now()},"id":${keeper.id ?? "null"},"tracker":${tracker}}`;
    const writable = await file.createWritable();
    await writable.write(text);
    await writable.close();
  } catch {
    // Not worth stopping for: the log has the session's totals.
  }
}

async function clearRecovery() {
  try {
    await (await navigator.storage.getDirectory()).removeEntry(RECOVERY_FILE);
  } catch {
    // None kept.
  }
}

/** The tracker and log id to carry on with, if they were kept less than a break ago. */
async function readRecovery(): Promise<{ tracker: string; id: number | null } | null> {
  const file = await recoveryFile(false);
  if (!file) return null;
  try {
    const saved = JSON.parse(await (await file.getFile()).text()) as {
      version: number;
      savedAt: number;
      id: number | null;
      tracker: unknown;
    };
    if (saved.version !== RECOVERY_VERSION || Date.now() - saved.savedAt > BREAK_ENDS_SESSION_MS) return null;
    return { tracker: JSON.stringify(saved.tracker), id: saved.id };
  } catch {
    return null;
  }
}

/** Saves the session to the records once it is long enough, as the program's keeper does. */
async function save(summary: SessionSummary | null, force = false) {
  if (!summary || summary.activeSeconds < SAVE_AFTER_ACTIVE) return;
  if (!force && Date.now() - keeper.savedAt < SAVE_EVERY_MS) return;
  keeper.savedAt = Date.now();
  const named = summary.name ? summary : { ...summary, name: character() };
  try {
    keeper.id = await invoke<number>("save_live_session", { id: keeper.id, session: named });
  } catch (err) {
    console.error("Couldn't save the session:", err);
  }
}

/** Puts the session aside and starts a fresh one: saved first, its recovery forgotten. */
async function freshKeeper(before: SessionSummary | null) {
  await save(before, true);
  keeper = { id: null, savedAt: 0 };
  lastActive = { seconds: 0, at: Date.now() };
  await clearRecovery();
}

const send = (message: ToLive, transfer: Transferable[] = []) => worker?.postMessage(message, transfer);

function onWorker({ data }: MessageEvent<FromLive>) {
  switch (data.type) {
    case "read": {
      const { read } = data;
      analysis = read.analysis;
      void emit("analysis", read.analysis);
      publishSession(read.session);
      if (read.session.activeSeconds > lastActive.seconds) {
        lastActive = { seconds: read.session.activeSeconds, at: Date.now() };
      }
      // A long break, game open or not, ends the session so tonight's play isn't added to
      // this afternoon's.
      if (!held && lastActive.seconds > 0 && Date.now() - lastActive.at >= BREAK_ENDS_SESSION_MS) {
        void freshKeeper(read.session);
        send({ op: "reset" });
      } else {
        void save(read.session);
      }
      setStatus(
        read.hud
          ? {
              state: "reading",
              window: shared?.label ?? "",
              width: data.width,
              height: data.height,
              frames: data.frames,
              readMs: read.ms,
              hdr: false,
              held,
            }
          : { state: "waiting", reason: "Reading the shared window. The game's status bar isn't in view." },
      );
      break;
    }
    case "other":
      void freshKeeper(data.before);
      break;
    case "summary":
      publishSession(data.session);
      break;
    case "snapshot":
      void writeRecovery(data.tracker);
      break;
    case "ended":
      void stopLive();
      break;
    case "error":
      console.error("Live reading:", data.message);
      break;
  }
}

/** Asks the player for the game window and starts reading it. Null back means they cancelled. */
export async function startLive(): Promise<void> {
  if (!canRead) throw new Error(cantRead ?? "This browser can't read a shared window.");
  if (worker) return;
  const w = new Worker(new URL("./live.worker.ts", import.meta.url), { type: "module" });
  w.onmessage = onWorker;
  worker = w;
  setStatus({ state: "waiting", reason: "Pick the MapleStory window in your browser's list." });
  let got: SharedWindow | null = null;
  try {
    got = await shareWindow((message, transfer) => w.postMessage(message, transfer), () => void stopLive());
  } catch (err) {
    w.terminate();
    worker = null;
    setStatus({ state: "failed", message: `The browser wouldn't share a window: ${err instanceof Error ? err.message : err}` });
    return;
  }
  if (!got) {
    w.terminate();
    worker = null;
    setStatus({ state: "off" });
    return;
  }
  shared = got;
  held = false;
  const recovered = await readRecovery();
  keeper = { id: recovered?.id ?? null, savedAt: 0 };
  lastActive = { seconds: 0, at: Date.now() };
  const choice = await settings();
  send({
    op: "start",
    idleAfter: choice.idleAfter,
    interval: choice.interval,
    saved: recovered?.tracker ?? null,
    titleModel: `${base}/ocr/text-recognition.onnx`,
    maps: MAP_LIST_URLS,
  });
  setStatus({ state: "waiting", reason: "Waiting for the first picture of the game." });
}

/** Stops reading. The session stays on Now, saved, and carries on if reading starts again soon. */
export async function stopLive(): Promise<void> {
  if (!worker) return;
  const w = worker;
  worker = null;
  shared?.stop();
  shared = null;
  // The worker's last snapshot, then let it go.
  const last = new Promise<void>((resolve) => {
    const timer = setTimeout(resolve, 1000);
    w.onmessage = (message: MessageEvent<FromLive>) => {
      if (message.data.type === "snapshot") {
        clearTimeout(timer);
        void writeRecovery(message.data.tracker).then(resolve);
      }
    };
  });
  w.postMessage({ op: "stop" } satisfies ToLive);
  await last;
  w.terminate();
  await save(session, true);
  setStatus({ state: "off" });
}

export function setLiveHeld(next: boolean) {
  held = next;
  send({ op: "held", held: next });
  if (status.state === "reading") setStatus({ ...status, held: next });
}

/** Starts a new session: the one so far saved, then put aside. */
export async function resetLiveSession() {
  await freshKeeper(session);
  if (worker) send({ op: "reset" });
  else {
    session = null;
    analysis = null;
    void emit("session", null);
    void emit("analysis", null);
  }
}

if (typeof window !== "undefined") {
  // A closed tab saves what it can; the tracker's state was kept within the last half minute.
  window.addEventListener("pagehide", () => {
    if (worker) void save(session, true);
  });
}
