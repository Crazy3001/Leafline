// Saved sessions, shared by the Stats and Train tabs and the workbench. Follows saves and
// deletes made in any window.

import { backend } from "./api";
import type { History, MeasuredRate, SessionSummary } from "./types";

export const history = $state<{ value: History | null }>({ value: null });

let watching = false;

export function watchHistory() {
  if (watching) return;
  watching = true;
  refresh();
  backend.onHistory(refresh);
}

function refresh() {
  backend
    .sessionHistory()
    .then((h) => (history.value = h))
    .catch(() => (history.value = null));
}

/** Whether this session is already in the history. A live session is saved as it goes. */
export function isSaved(session: SessionSummary | null): boolean {
  return session?.startedAt != null && (history.value?.sessions.some((s) => s.startedAt === session.startedAt) ?? false);
}

/**
 * EXP per hour measured on each map: every saved session, plus the current one unless it has
 * been saved already.
 */
export function measuredRates(session: SessionSummary | null): MeasuredRate[] {
  const saved = (history.value?.maps ?? []).map((m) => ({
    label: m.map,
    expPerHour: m.expPerHour,
    activeSeconds: m.activeSeconds,
  }));
  const current =
    session && !isSaved(session)
      ? session.maps
          .filter((m) => m.map != null && m.expPerHour != null && m.activeSeconds > 0)
          .map((m) => ({ label: m.map!, expPerHour: m.expPerHour!, activeSeconds: m.activeSeconds }))
      : [];
  return [...saved, ...current];
}
