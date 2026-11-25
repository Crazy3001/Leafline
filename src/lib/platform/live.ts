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
import { emit, listen } from "./bus";
import { canRead, cantRead, shareWindow, type SharedWindow } from "./capture";
import type { FromLive, ReadingChoice, ToLive, WindowReads } from "./live.worker";
import { invoke } from "./storage.svelte";

/** Active seconds before a session is worth keeping (a quick login to sell isn't training). */
const SAVE_AFTER_ACTIVE = 300;
const SAVE_EVERY_MS = 30_000;
/** A break this long with no EXP ends the session; the next gain starts a new one. */
const BREAK_ENDS_SESSION_MS = 30 * 60_000;
/** Where the reference data the readers settle against comes from (the known maps for the minimap
 *  title; quests, items and monsters for the Quest Helper's titles and the Pickup Log's items):
 *  Leafline's mirror of OSMS first, OSMS second, as the site's own data download goes
 *  (crates/leafline-core/src/mapdata/fetch.rs). */
const DATA_SOURCES = [
  "https://raw.githubusercontent.com/Crazy3001/Leafline-data/main/osms/v1/",
  "https://osmsdataexplorer.com/data/api/v1/",
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
/** Stops following the player's settings once reading stops. */
let unlistenSettings: (() => void) | null = null;
/** The session's log id once saved, and when it was last saved. */
let keeper: { id: number | null; savedAt: number } = { id: null, savedAt: 0 };
/** The session's active time when it last grew, and when that was. */
let lastActive = { seconds: 0, at: Date.now() };
/** The name to save sessions under while the HUD's name hasn't been read yet. */
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

type LiveSettings = {
  live?: {
    interval?: number;
    pauseAfter?: number;
    pickupLog?: boolean;
    itemTooltips?: boolean;
    characterWindows?: boolean;
    bagCheck?: boolean;
    questWindow?: boolean;
  };
};

function choiceOf(s: LiveSettings): ReadingChoice {
  return {
    interval: s.live?.interval ?? 2,
    idleAfter: s.live?.pauseAfter ?? 15,
    pickupLog: s.live?.pickupLog ?? true,
    itemTooltips: s.live?.itemTooltips ?? true,
    characterWindows: s.live?.characterWindows ?? true,
    bagCheck: s.live?.bagCheck ?? true,
    questWindow: s.live?.questWindow ?? true,
  };
}

async function settings(): Promise<ReadingChoice> {
  try {
    return choiceOf(await invoke<LiveSettings>("get_settings"));
  } catch {
    return choiceOf({});
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

/**
 * What to say while the shared picture has no status bar in it: what was shared and its size,
 * then what to check, most likely first. The website can't ask Windows whether HDR is on for the
 * game's display, as the program does; the browser only knows about its own display.
 */
function notFound(width: number, height: number): string {
  const size = width && height ? ` (${width}×${height})` : "";
  if (shared?.surface === "monitor")
    return `You shared a whole screen${size}. Stop reading, then share the MapleStory window itself.`;
  if (shared?.surface === "browser")
    return `You shared a browser tab${size}. Stop reading, then share the MapleStory window itself.`;
  const hdr = typeof matchMedia === "function" && matchMedia("(dynamic-range: high)").matches;
  return (
    `Can't find the game's status bar in the shared window${size}. ` +
    (hdr ? "Windows HDR looks on for this screen: " : "") +
    "Turn Windows HDR off and restart the game, set UI Size to Ideal Ratio, and check that the window you shared is MapleStory."
  );
}

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
          : { state: "waiting", reason: notFound(data.width, data.height) },
      );
      break;
    }
    case "other":
      void freshKeeper(data.before);
      break;
    case "summary":
      publishSession(data.session);
      break;
    case "items":
      // In place on the card, between full reads; a new item brings the item check up, as the
      // program's Read item hotkey does.
      if (analysis) {
        analysis = { ...analysis, tooltips: data.tooltips };
        void emit("analysis", analysis);
      }
      if (data.tooltips.length) void emit("item-read", { found: data.tooltips.length, problem: null });
      break;
    case "windows":
      void keepWindows(data.reads);
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

/**
 * Hands what the game's windows showed to the records, as the program's live loop does: the build
 * moved along from the stat and skill windows, the open inventory tab checked (sell or keep), the
 * Quest window kept as the character's quest log. Each command tells the page what changed by its
 * events ("character-read", "bag", "quest-log"). Nothing leaves this PC.
 */
async function keepWindows(reads: WindowReads) {
  const run = async (what: string, command: string, args: Record<string, unknown>) => {
    try {
      await invoke(command, args);
    } catch (err) {
      console.error(`Couldn't keep the ${what}:`, err);
    }
  };
  if (reads.character) await run("character's windows", "catch_up_character", { scan: reads.character });
  if (reads.questLog) await run("quest log", "follow_quest_log", reads.questLog);
  if (reads.bag) {
    const bag = reads.bag;
    await run("bag check", "check_bag", { bag, ...(await fetchBagPictures({ bag })) });
  }
}

/** Pictures fetched for a bag check at once. */
const PICTURES_AT_ONCE = 8;

/**
 * Fetches the item pictures a bag check will try, several at a time, so the storage worker finds
 * them in the browser's cache (OSMS lets it keep them for hours) instead of downloading each in
 * turn. Shows the check as under way meanwhile, as the program does. Returns nothing: the args
 * spread into the check's are empty.
 */
export async function fetchBagPictures(args: { bag?: unknown; wide?: boolean }): Promise<Record<string, never>> {
  let urls: string[] = [];
  try {
    urls = await invoke<string[]>("bag_wants", args);
  } catch {
    return {};
  }
  if (!urls.length) return {};
  const last = await invoke<Record<string, unknown> | null>("last_bag").catch(() => null);
  void emit("bag", { at: Date.now() / 1000, tab: null, character: null, items: [], unknown: [], wide: false, more: 0, problem: null, ...last, busy: true });
  let next = 0;
  const one = async () => {
    while (next < urls.length) {
      const url = urls[next++];
      try {
        await (await fetch(url)).arrayBuffer();
      } catch {
        // The check goes on without it.
      }
    }
  };
  await Promise.all(Array.from({ length: PICTURES_AT_ONCE }, one));
  return {};
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
    ...choice,
    saved: recovered?.tracker ?? null,
    textModel: `${base}/ocr/text-recognition.onnx`,
    dataSources: DATA_SOURCES,
  });
  setStatus({ state: "waiting", reason: "Waiting for the first picture of the game." });
  unlistenSettings?.();
  unlistenSettings = await listen<LiveSettings>("settings", (e) => send({ op: "settings", ...choiceOf(e.payload) }));
}

/** Stops reading. The session stays on Now, saved, and carries on if reading starts again soon. */
export async function stopLive(): Promise<void> {
  if (!worker) return;
  const w = worker;
  worker = null;
  unlistenSettings?.();
  unlistenSettings = null;
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
