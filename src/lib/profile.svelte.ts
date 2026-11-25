// Who the character being looked at was when Leafline last read them: level, job, EXP, last map
// and mesos, each kept with when it was read (records `character_profiles`, written as live
// reading goes). Shown dated and faded until the game is read again, when a live reading replaces
// each value at once. For showing only: nothing here feeds a rate, a forecast or a session.
//
// The character is the one `characters.active` names: the one on screen, the one picked in the
// character picker, or with neither, the one played last.

import { backend } from "./api";
import { characters } from "./characters.svelte";
import { formatAgo } from "./format";
import { history, watchHistory } from "./history.svelte";
import { live } from "./readings.svelte";
import type { KnownCharacter } from "./types";

const same = (a: string | null | undefined, b: string | null | undefined) =>
  a != null && b != null && a.trim().toLowerCase() === b.trim().toLowerCase();

const remembered = $state<{ value: KnownCharacter | null }>({ value: null });

/** The time now, moved on every half minute, so "3h ago" keeps up by itself. */
export const ticking = $state({ now: Date.now() / 1000 });

let asked = 0;

async function load(name: string | null) {
  const ask = ++asked;
  try {
    const found = await backend.knownCharacter(name);
    if (ask === asked) remembered.value = found;
  } catch {
    if (ask === asked) remembered.value = null;
  }
}

let watching = false;

/** Loads what was last read of the character looked at, and follows changes. Safe to call often. */
export function watchProfile() {
  if (watching) return;
  watching = true;
  watchHistory();
  void backend.onKnownCharacter(() => void load(characters.active));
  setInterval(() => (ticking.now = Date.now() / 1000), 30_000);
  $effect.root(() => {
    $effect(() => {
      const name = characters.active;
      void load(name);
    });
  });
}

const onScreen = () => live.session?.name ?? live.analysis?.hud?.name ?? null;

/**
 * What the records know of `name`, filled in from their last saved session where the HUD was never
 * written down for them (records from before Leafline kept it).
 */
function knownOf(name: string | null): KnownCharacter | null {
  const kept = remembered.value && (name == null || same(remembered.value.name, name)) ? remembered.value : null;
  if (kept?.level != null) return kept;
  const who = name ?? kept?.name ?? null;
  const last = (history.value?.sessions ?? []).find((s) => who == null || same(s.character, who));
  if (!last || last.level == null) return kept;
  const base: KnownCharacter = kept ?? {
    name: last.character ?? who ?? "",
    level: null,
    job: null,
    expPercent: null,
    expPoints: null,
    map: null,
    seenAt: null,
    mesos: null,
    mesosSeenAt: null,
  };
  if (!base.name) return null;
  return { ...base, level: last.level, job: base.job ?? last.job, seenAt: base.seenAt ?? last.savedAt };
}

export const profile = {
  /** Whose profile this is: the character looked at, else the one seen last. */
  get name(): string | null {
    return characters.active ?? remembered.value?.name ?? null;
  },
  /** The game on screen shows this character, or no name is read yet: live readings are theirs. */
  get live(): boolean {
    const now = onScreen();
    const looked = characters.active;
    return now == null || looked == null || same(now, looked);
  },
  /** What was last read of them, or null when nothing was. */
  get known(): KnownCharacter | null {
    return knownOf(characters.active);
  },
};

/** A value to show: the one read now when there is one, else the one remembered, with when. */
export type Shown<T> = { value: T | null; at: number | null };

/**
 * `now` when this character is on screen (`onScreen`, by default whether the game shows them) and it
 * was read; else `known`, dated `at`.
 */
export function lastKnown<T>(
  now: T | null | undefined,
  known: T | null | undefined,
  at: number | null | undefined,
  onScreen = profile.live,
): Shown<T> {
  if (onScreen && now != null) return { value: now, at: null };
  if (known != null) return { value: known, at: at ?? null };
  return { value: null, at: null };
}

/** "3h ago" for a time remembered, kept up to date. */
export function ago(at: number): string {
  return formatAgo(ticking.now - at);
}
