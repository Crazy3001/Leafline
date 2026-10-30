// The latest readings, overlay status and pins, shared by the overlay and pinned panels. Each
// window asks for the current state once and then follows the backend's events.

import { backend } from "./api";
import type { Analysis, OverlayStatus, PinStatus, SessionSummary } from "./types";

export const live = $state<{
  analysis: Analysis | null;
  session: SessionSummary | null;
  status: OverlayStatus;
  pins: PinStatus;
}>({
  analysis: null,
  session: null,
  status: { visible: true, clickThrough: false },
  pins: { pinned: [], hidden: false },
});

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
}
