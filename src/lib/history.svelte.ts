// Saved sessions, shared by the Character and Train tabs. Follows saves and
// deletes made in any window.

import { backend } from "./api";
import { characters } from "./characters.svelte";
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
 * EXP per hour measured on each map, one entry per map, level and party size of a session: every
 * saved session, plus the current one unless it has been saved already. The level and party let
 * the kill speed behind the training estimates keep to solo play near the character's level.
 */
export function measuredRates(session: SessionSummary | null): MeasuredRate[] {
  // The active character's own play: kill speed and best rates differ with the class and level.
  const lines = [
    ...characters.sessions(history.value?.sessions ?? []).flatMap((s) => s.lines),
    ...(session && !isSaved(session) && characters.isActive(session.name) ? session.lines : []),
  ];
  return lines
    .filter((l) => l.map != null && !l.town && l.activeSeconds > 0 && l.expGained > 0)
    .map((l) => ({
      label: l.map!,
      expPerHour: (l.expGained * 3600) / l.activeSeconds,
      activeSeconds: l.activeSeconds,
      level: l.level,
      party: l.party,
      // Kills count only when nearly every gain matched the map's monsters.
      kills: l.kills != null && l.kills > 0 && (l.gainsMissed ?? 0) * 10 <= l.kills ? l.kills : null,
      fighter: l.fighter ?? null,
      deaths: l.deaths ?? null,
    }));
}
