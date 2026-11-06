// The latest readings, overlay status, live status and pins, shared by the overlay and pinned panels. Each
// window asks for the current state once and then follows the backend's events.

import { backend } from "./api";
import type {
  Analysis,
  CharacterRead,
  HudReading,
  ItemRead,
  LiveStatus,
  OverlayStatus,
  PinStatus,
  SessionSummary,
} from "./types";

export const live = $state<{
  analysis: Analysis | null;
  session: SessionSummary | null;
  status: OverlayStatus;
  pins: PinStatus;
  liveStatus: LiveStatus;
  /** The last picture saved with "Save frame", shown briefly. */
  savedFrame: string | null;
  /** How the last item check went, shown briefly. */
  itemRead: ItemRead | null;
  /** How the last read of the stat and skill windows went, for a moment. */
  characterRead: CharacterRead | null;
}>({
  analysis: null,
  session: null,
  status: { visible: true, clickThrough: false, compact: false },
  pins: { pinned: [], hidden: false },
  liveStatus: { state: "off" },
  savedFrame: null,
  itemRead: null,
  characterRead: null,
});

let savedTimer: ReturnType<typeof setTimeout> | undefined;
let itemTimer: ReturnType<typeof setTimeout> | undefined;
let characterTimer: ReturnType<typeof setTimeout> | undefined;

let watching = false;

export function watchLive() {
  if (watching) return;
  watching = true;
  backend.currentReadings().then((current) => {
    live.analysis ??= current.analysis;
    live.session ??= current.session;
  });
  backend.overlayStatus().then((s) => {
    live.status = s;
  });
  backend.pinStatus().then((p) => {
    live.pins = p;
  });
  backend.onAnalysis((a) => {
    live.analysis = a && { ...a, hud: steadyHud(a.hud, live.analysis?.hud ?? null) };
  });
  backend.onSession((s) => {
    live.session = s;
  });
  backend.onOverlayStatus((s) => {
    live.status = s;
  });
  backend.onPins((p) => {
    live.pins = p;
  });
  backend.liveStatus().then((s) => {
    live.liveStatus = s;
  });
  backend.onLive((s) => {
    live.liveStatus = s;
  });
  backend.onFrameSaved((path) => {
    live.savedFrame = path;
    clearTimeout(savedTimer);
    savedTimer = setTimeout(() => (live.savedFrame = null), 4000);
  });
  backend.onItemRead((read) => {
    live.itemRead = read;
    clearTimeout(itemTimer);
    itemTimer = setTimeout(() => (live.itemRead = null), 4000);
  });
  backend.onCharacterRead((read) => {
    live.characterRead = read;
    clearTimeout(characterTimer);
    characterTimer = setTimeout(() => (live.characterRead = null), 6000);
  });
}

/**
 * The HUD with what it last showed filled in where this reading has nothing: loading a map, the
 * Cash Shop or a window over the status bar hides it for a moment, and HP, MP and EXP shouldn't
 * drop to nothing and fill up again. Another character's name starts afresh.
 */
export function steadyHud(next: HudReading | null, last: HudReading | null): HudReading | null {
  if (!last) return next;
  if (!next) return last;
  if (next.name && last.name && next.name !== last.name && next.level != null) return next;
  // EXP shows as points and percent, or as percent alone at high levels; keep one form.
  const exp = next.exp ?? (next.expPercent == null ? last.exp : null);
  return {
    level: next.level ?? last.level,
    job: next.job ?? last.job,
    name: next.name ?? last.name,
    hp: next.hp ?? last.hp,
    mp: next.mp ?? last.mp,
    exp,
    expPercent: next.expPercent ?? (exp == null ? last.expPercent : null),
  };
}
