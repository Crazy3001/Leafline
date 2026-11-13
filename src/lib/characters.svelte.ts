// Which character the player is on, for everything that is kept per character: training
// estimates, best maps, potions, goals. It follows the name the HUD shows. With none on screen
// (the game is closed) it is the character played last, and the player can pick another to look
// at; that pick stands until the HUD shows a different name. Or look at all of them at once:
// the views that blend (the log, records, best maps, where to train) then use every session, while
// a goal or a set of potions still belongs to one character, the one on screen or played last.

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
const picked = $state<{ name: string | null; liveWas: string | null; all: boolean }>({
  name: null,
  liveWas: null,
  all: false,
});

const liveName = () => live.session?.name ?? null;

export const characters = {
  /** Everyone with a saved session, plus the one on screen now. */
  get known(): string[] {
    const saved = charactersOf(history.value?.sessions ?? []);
    const now = liveName();
    return now && !saved.some((c) => same(c, now)) ? [now, ...saved] : saved;
  },
  /** The one character a goal or a set of potions belongs to, or null before any is known. */
  get active(): string | null {
    const now = liveName();
    if (picked.name && !picked.all && same(picked.liveWas, now)) return picked.name;
    return now ?? this.known[0] ?? null;
  },
  /** Looking at every character at once. Ends when the HUD shows a different name. */
  get all(): boolean {
    return picked.all && same(picked.liveWas, liveName());
  },
  /** The character the blended views are narrowed to: null for all of them. */
  get viewing(): string | null {
    return this.all ? null : this.active;
  },
  /** Looks at another character; null goes back to following the HUD. */
  choose(name: string | null) {
    picked.all = false;
    if (name == null || same(name, liveName())) {
      picked.name = null;
      picked.liveWas = null;
      return;
    }
    picked.name = name;
    picked.liveWas = liveName();
  },
  /** Looks at every character's sessions together. */
  chooseAll() {
    picked.name = null;
    picked.all = true;
    picked.liveWas = liveName();
  },
  /** Whether a session read for this name (or none) is the active character's. */
  isActive(name: string | null): boolean {
    return name == null || this.viewing == null || same(name, this.viewing);
  },
  /** The sessions of the active character. */
  sessions<T extends { character: string | null }>(sessions: readonly T[]): T[] {
    return sessionsOf(sessions, this.viewing, this.known);
  },
};
