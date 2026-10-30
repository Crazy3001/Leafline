// The latest readings, overlay status, live status and pins, shared by the overlay and pinned panels. Each
// window asks for the current state once and then follows the backend's events.

import { backend } from "./api";
import type { Analysis, LiveStatus, OverlayStatus, PinStatus, SessionSummary } from "./types";

export const live = $state<{
  analysis: Analysis | null;
  session: SessionSummary | null;
  status: OverlayStatus;
  pins: PinStatus;
  liveStatus: LiveStatus;
  /** The last picture saved with "Save frame", shown briefly. */
  savedFrame: string | null;
}>({
  analysis: null,
  session: null,
  status: { visible: true, clickThrough: false, compact: false },
  pins: { pinned: [], hidden: false },
  liveStatus: { state: "off" },
  savedFrame: null,
});

let savedTimer: ReturnType<typeof setTimeout> | undefined;

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
    live.analysis = a;
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
}
