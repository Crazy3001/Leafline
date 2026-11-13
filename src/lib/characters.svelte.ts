// Which character the player is on, for everything that is kept per character: training
// estimates, best maps, potions, goals. It follows the name the HUD shows. With none on screen
// (the game is closed) it is the character played last, and the player can pick another to look
// at; that pick stands until the HUD shows a different name.

import { history } from "./history.svelte";
import { live } from "./readings.svelte";
import type { SavedSession } from "./types";

const same = (a: string | null | undefined, b: string | null | undefined) =>
  a != null && b != null && a.trim().toLowerCase() === b.trim().toLowerCase();

/** Every character with a saved session, the one played last first. */
export function charactersOf(sessions: readonly SavedSession[]): string[] {
  const out: string[] = [];
  for (const s of sessions) if (s.character && !out.some((c) => same(c, s.character))) out.push(s.character);
  return out;
}

/**
 * The sessions that belong to a character. With one character, or none picked, every session
 * counts, those that never read a name included. With two or more, a session counts for the
 * character it names, and one with no name for nobody: there is no telling whose it was.
 */
export function sessionsOf<T extends { character: string | null }>(
  sessions: readonly T[],
  character: string | null,
  known: readonly string[],
): T[] {
  if (!character || known.length < 2) return [...sessions];
  return sessions.filter((s) => same(s.character, character));
}

/** A pick made by hand, and the name the HUD showed then. The HUD showing another name wins. */
const picked = $state<{ name: string | null; liveWas: string | null }>({ name: null, liveWas: null });

const liveName = () => live.session?.name ?? null;

export const characters = {
  /** Everyone with a saved session, plus the one on screen now. */
  get known(): string[] {
    const saved = charactersOf(history.value?.sessions ?? []);
    const now = liveName();
    return now && !saved.some((c) => same(c, now)) ? [now, ...saved] : saved;
  },
  /** The character everything per-character shows, or null before any is known. */
  get active(): string | null {
    const now = liveName();
    if (picked.name && same(picked.liveWas, now)) return picked.name;
    return now ?? this.known[0] ?? null;
  },
  /** Looks at another character; null goes back to following the HUD. */
  choose(name: string | null) {
    if (name == null || same(name, liveName())) {
      picked.name = null;
      picked.liveWas = null;
      return;
    }
    picked.name = name;
    picked.liveWas = liveName();
  },
  /** Whether a session read for this name (or none) is the active character's. */
  isActive(name: string | null): boolean {
    return name == null || this.active == null || same(name, this.active);
  },
  /** The sessions of the active character. */
  sessions<T extends { character: string | null }>(sessions: readonly T[]): T[] {
    return sessionsOf(sessions, this.active, this.known);
  },
};
