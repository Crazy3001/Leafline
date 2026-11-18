// The session being typed in, shared by the Session card on Now and the website's pop-out, and
// kept in this browser between visits. What it works out to is asked of the shared Rust whenever
// an input changes, so both show the same numbers.

import { backend } from "./api";
import { characters } from "./characters.svelte";
import type { HudAt, TypedOutcome, TypedSession } from "./types";

const KEY = "leafline.typed-session";

export interface Side {
  level: string;
  percent: string;
  points: string;
  mesos: string;
}

export interface Draft {
  start: Side;
  end: Side;
  /** When Start now was pressed, in ms. */
  startedAt: number | null;
  minutes: string;
  map: string;
}

export const blankSide = (): Side => ({ level: "", percent: "", points: "", mesos: "" });
const blankDraft = (): Draft => ({ start: blankSide(), end: blankSide(), startedAt: null, minutes: "", map: "" });

function load(): Draft {
  try {
    const saved = JSON.parse(localStorage.getItem(KEY) ?? "null");
    if (saved?.start && saved?.end) return saved;
  } catch {
    // A missing or unreadable draft starts empty.
  }
  return blankDraft();
}

export const typed = $state<{
  draft: Draft;
  outcome: TypedOutcome | null;
  problem: string | null;
  /** Ms, ticking each second, for the clock. */
  now: number;
}>({ draft: load(), outcome: null, problem: null, now: Date.now() });

const number = (text: string): number | null => {
  const n = Number(text.replace(/[,\s%]/g, ""));
  return text.trim() === "" || !Number.isFinite(n) ? null : n;
};

function hud(side: Side): HudAt | null {
  const level = number(side.level);
  const percent = number(side.percent);
  if (level == null || percent == null) return null;
  return { level, percent, points: number(side.points), mesos: number(side.mesos) };
}

/** Minutes played: the running clock's, else the minutes typed. */
export function playedMinutes(): number | null {
  const d = typed.draft;
  return d.startedAt ? (typed.now - d.startedAt) / 60_000 : number(d.minutes);
}

/** The session as typed so far, or null while something it needs is missing. */
export function typedSession(minutes: number | null = playedMinutes()): TypedSession | null {
  const d = typed.draft;
  const start = hud(d.start);
  const end = hud(d.end);
  if (!start || !end || minutes == null) return null;
  return {
    character: characters.viewing,
    map: d.map.trim() || null,
    start,
    end,
    minutes,
    startedAt: d.startedAt ? d.startedAt / 1000 : null,
    note: null,
  };
}

export function resetTyped() {
  typed.draft = blankDraft();
  typed.outcome = null;
  typed.problem = null;
}

/** Saves the session to the training log; the next one starts where it ended. */
export async function saveTyped(): Promise<void> {
  const s = typedSession();
  if (!s) return;
  s.minutes = Math.max(1, Math.round(s.minutes * 10) / 10);
  await backend.saveTypedSession(s);
  const ended = { ...typed.draft.end };
  resetTyped();
  typed.draft.start = ended;
}

let started = false;

/** Keeps the draft saved, the clock ticking and the numbers worked out. Call from any user of it. */
export function followTyped() {
  if (started) return;
  started = true;
  setInterval(() => (typed.now = Date.now()), 1000);
  $effect.root(() => {
    $effect(() => {
      try {
        localStorage.setItem(KEY, JSON.stringify($state.snapshot(typed.draft)));
      } catch {
        // The draft is a convenience; without storage it lasts as long as the page.
      }
    });
    // Asked again when an input changes or the clock passes a whole minute, not every tick.
    const wholeMinutes = $derived.by(() => {
      const m = playedMinutes();
      return m == null ? null : Math.max(1, Math.floor(m));
    });
    let asked = 0;
    $effect(() => {
      const s = typedSession(wholeMinutes);
      const ask = ++asked;
      if (!s) {
        typed.outcome = null;
        typed.problem = null;
        return;
      }
      backend.typedSessionOutcome(s).then(
        (o) => {
          if (ask !== asked) return;
          typed.outcome = o;
          typed.problem = null;
        },
        (e) => {
          if (ask !== asked) return;
          typed.outcome = null;
          typed.problem = e instanceof Error ? e.message : String(e);
        },
      );
    });
  });
}
