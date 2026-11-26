// What Leafline sees: when each reader last read something it kept, or why it isn't reading, in
// plain words with what to do. Fed by the live reading (the program's live loop, or the website's
// capture worker) as `Sight`; everything stays on this PC. It only reports: it never asks the
// player to teach Leafline anything.

import { backend } from "./api";
import { can } from "./platform";
import { live } from "./readings.svelte";
import { display } from "./settings.svelte";
import { formatAgo } from "./format";
import type { Reader, Sight } from "./types";

export const sight = $state<{ value: Sight | null; now: number }>({ value: null, now: Date.now() / 1000 });

let watching = false;

/** Loads what Leafline sees and follows it. Safe to call from every component that shows it. */
export function watchSight() {
  if (watching) return;
  watching = true;
  backend
    .readerSight()
    .then((s) => (sight.value ??= s))
    .catch(() => {});
  void backend.onSight((s) => (sight.value = s));
  setInterval(() => (sight.now = Date.now() / 1000), 1000);
}

type LiveSwitch = "pickupLog" | "itemTooltips" | "characterWindows" | "bagCheck" | "questWindow";

export interface ReaderInfo {
  reader: Reader;
  label: string;
  /** What makes it read, said to the player. */
  need: string;
  /** Read only when the player opens or does something, so a check doesn't count it as missed. */
  optional: boolean;
  /** The player's switch for it, in Settings → Live reading. */
  setting?: LiveSwitch;
}

export const READERS: ReaderInfo[] = [
  { reader: "statusBar", label: "Status bar", need: "Put the game in front, with your character in a map.", optional: false },
  { reader: "level", label: "Level", need: "It shows on the status bar.", optional: false },
  { reader: "exp", label: "EXP", need: "It shows on the status bar.", optional: false },
  { reader: "name", label: "Character name", need: "It shows on the status bar.", optional: false },
  { reader: "map", label: "Map name", need: "Open the minimap: the map's name is its title.", optional: false },
  { reader: "mesos", label: "Mesos", need: "Open your inventory: the meso count is at its foot.", optional: true },
  { reader: "questHelper", label: "Quest Helper", need: "Track a quest so the Quest Helper shows it.", optional: true },
  { reader: "pickupLog", label: "Pickup Log", need: "Pick something up.", optional: true, setting: "pickupLog" },
  { reader: "itemTooltip", label: "Item cards", need: "Rest the mouse on an item.", optional: true, setting: "itemTooltips" },
  {
    reader: "characterWindows",
    label: "Stat and Skill windows",
    need: "Open your Stat or Skill window once.",
    optional: true,
    setting: "characterWindows",
  },
  {
    reader: "inventory",
    label: "Inventory (sell or keep)",
    need: "Open your Item Inventory on a tab and hold it still.",
    optional: true,
    setting: "bagCheck",
  },
  { reader: "questWindow", label: "Quest window", need: "Open your Quest window.", optional: true, setting: "questWindow" },
];

/** A reading this recent counts as reading now. */
const FRESH = 120;

export type Tone = "good" | "stale" | "bad" | "quiet";

export interface Verdict {
  tone: Tone;
  text: string;
}

/**
 * Why nothing is read at all, when that's so: reading off or waiting, or a picture Leafline can't
 * use (black, frozen, washed out by HDR, no status bar in it). Null while the picture is fine.
 */
export function pictureProblem(): string | null {
  const status = live.liveStatus;
  if (status.state === "off") return can.liveReading ? "Live reading is off." : "Share the game with this page to start reading.";
  if (status.state === "failed") return `Live reading stopped: ${status.message}`;
  if (status.state === "waiting") return status.reason;
  const picture = sight.value?.picture;
  if (!picture) return "Waiting for the first picture of the game.";
  if (picture.statusBar) return null;
  if (picture.dark)
    return can.liveReading
      ? "The picture of the game comes through black. Try Windowed in the game's Graphics options, or restart the game."
      : "The shared picture comes through black. Some browsers can't capture the game in Full Screen: try Windowed in the game's Graphics options, then share it again.";
  if (picture.still)
    return "The same picture keeps arriving: the browser isn't following the game. Share the screen the game is on instead, or use Chrome or Edge.";
  if (picture.hdr) return "Windows HDR is on, which washes the picture out. Turn HDR off in Windows display settings, then restart the game.";
  return "Can't find the status bar. Put the game in front with your character in a map, set UI Size to Ideal Ratio, and check that Windows HDR is off.";
}

/** Notes on a picture that reads but less well. */
export function pictureNotes(): string[] {
  const picture = sight.value?.picture;
  if (!picture?.statusBar) return [];
  const out: string[] = [];
  if (picture.hdr) out.push("Windows HDR is on, which washes the picture out: some readings may be missed.");
  if (picture.smoothedUi) out.push("UI Size is Default Ratio, which smooths the game's text. Ideal Ratio reads best.");
  return out;
}

const ago = (at: number) => formatAgo(sight.now - at);

/** Whether a reader read since `since` (a check's start, or reading's start). */
export function readSince(reader: Reader, since: number): boolean {
  const at = sight.value?.seen.find((s) => s.reader === reader)?.at;
  return at != null && at >= since;
}

/** Where one reader stands: read and when, left out and why, or what fills it. */
export function verdict(info: ReaderInfo, since = sight.value?.since ?? 0): Verdict {
  if (info.setting && display.live[info.setting] === false) return { tone: "quiet", text: "Off in Settings." };
  const value = sight.value;
  const last = value?.seen.find((s) => s.reader === info.reader)?.at ?? null;
  const rejected = value?.rejected.find((r) => r.reader === info.reader && r.at >= since && (last == null || r.at > last));
  if (last != null && last >= since) {
    const age = sight.now - last;
    if (rejected && age >= FRESH) return { tone: "bad", text: rejected.reason };
    const reading = live.liveStatus.state === "reading";
    if (age >= FRESH) return { tone: "stale", text: `Last read ${ago(last)}` };
    return { tone: "good", text: reading && age < 10 ? "Reading" : `Read ${ago(last)}` };
  }
  if (rejected) return { tone: "bad", text: rejected.reason };
  const problem = pictureProblem();
  if (problem) return { tone: "bad", text: "Not read." };
  const onBar = info.reader === "level" || info.reader === "exp" || info.reader === "name";
  if (onBar && value?.picture?.smoothedUi)
    return { tone: "bad", text: "Not read yet: UI Size Default Ratio smooths the text. Ideal Ratio reads best." };
  if (onBar && value?.picture?.hdr) return { tone: "bad", text: "Not read yet: Windows HDR washes the text out." };
  return { tone: "quiet", text: `Not seen yet. ${info.need}` };
}
