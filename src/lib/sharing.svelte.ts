// Where the website's live reading stands, said one way for the desk's sidebar pill and the banner
// over the views: not reading (red), shared but not reading the game (amber), reading (green), or
// sharing stopped by the browser (red). And the one way to start it.
import { backend } from "$lib/api";
import { support } from "$lib/platform/capture";
import { live } from "$lib/readings.svelte";
import { pictureNotes, pictureProblem, sight } from "$lib/sight.svelte";

export type ShareTone = "off" | "picking" | "stuck" | "reading" | "failed";

/** The first step, while the browser's list of windows is open. */
const PICKING = "Pick the MapleStory window";

/** A browser without a frame-by-frame stream reader (Firefox's route) shares screens, not a game's own window. */
export const screensOnly = support.share && !support.trackFrames;

export const SHARE_TIP =
  "Leafline reads your level, EXP, mesos, map, quests and loot from the game window. The picture stays on this computer.\n" +
  "Keep the HUD and minimap in view." +
  (screensOnly ? "\nThis browser shares a whole screen, not one window: pick the screen the game is on." : "");

const READING_TIP =
  "Keep the HUD and minimap in view. Mesos are read while your inventory is open, quests from the Quest Helper and loot from the lines at the bottom right. The session saves itself after 5 minutes.";

/** Why starting failed before the browser got as far as a status. */
const started = $state<{ problem: string | null }>({ problem: null });

export type ShareState = {
  tone: ShareTone;
  /** A word or two for the pill, beside its button in the sidebar's width. */
  label: string;
  /** What to know or do, for a tooltip or the banner. */
  detail: string;
  /** Reading, but less well (Windows HDR, Default Ratio): the pill turns amber. */
  note: string | null;
};

export function shareState(): ShareState {
  const status = live.liveStatus;
  if (started.problem) return { tone: "failed", label: "Stopped", detail: started.problem, note: null };
  if (status.state === "failed") return { tone: "failed", label: "Stopped", detail: status.message, note: null };
  if (status.state === "off") return { tone: "off", label: "Not reading", detail: SHARE_TIP, note: null };
  if (status.state === "waiting") {
    if (status.reason.startsWith(PICKING)) return { tone: "picking", label: "Picking…", detail: status.reason, note: null };
    return { tone: "stuck", label: "Can't read it", detail: status.reason, note: null };
  }
  const unusable = sight.value?.picture ? pictureProblem() : null;
  if (unusable) return { tone: "stuck", label: "Can't read it", detail: unusable, note: null };
  const note = pictureNotes()[0] ?? null;
  return { tone: "reading", label: "Reading live", detail: note ?? READING_TIP, note };
}

/** Asks the browser for the game window; a refusal is kept to say. */
export async function startSharing() {
  started.problem = null;
  try {
    await backend.startLive();
  } catch (err) {
    started.problem = err instanceof Error ? err.message : String(err);
  }
}

export function stopSharing() {
  started.problem = null;
  void backend.stopLive();
}
