// What the training estimates take as given because the map data doesn't say: how soon a
// defeated monster is back, how much EXP a kill gives over its base value, and, optionally, how
// hard the player hits. None is a known fact about Classic, so the player can change them. Kept
// in this PC's browser storage and shared by every Leafline window.

import type { Assumptions } from "./types";

const KEY = "leafline.train.assumptions";

export const DEFAULT_ASSUMPTIONS: Assumptions = { respawnSeconds: 8, expRate: 1, attackMin: null, attackMax: null, skillFactor: 1.4 };

export const RESPAWN_RANGE: [number, number] = [1, 600];
export const EXP_RATE_RANGE: [number, number] = [0.1, 20];
export const ATTACK_RANGE: [number, number] = [1, 99999];
export const SKILL_FACTOR_RANGE: [number, number] = [0.5, 5];

function clamp(value: number, [low, high]: [number, number], fallback: number) {
  return Number.isFinite(value) ? Math.min(high, Math.max(low, value)) : fallback;
}

/** An attack number, or null when it was left empty or isn't one. */
function attack(value: unknown): number | null {
  const n = Number(value);
  return value == null || value === "" || !Number.isFinite(n) || n <= 0 ? null : clamp(n, ATTACK_RANGE, 1);
}

function clean(raw: Partial<Assumptions> | null | undefined, base: Assumptions): Assumptions {
  const low = attack(raw?.attackMin === undefined ? base.attackMin : raw.attackMin);
  const high = attack(raw?.attackMax === undefined ? base.attackMax : raw.attackMax);
  return {
    respawnSeconds: clamp(Number(raw?.respawnSeconds ?? base.respawnSeconds), RESPAWN_RANGE, DEFAULT_ASSUMPTIONS.respawnSeconds),
    expRate: clamp(Number(raw?.expRate ?? base.expRate), EXP_RATE_RANGE, DEFAULT_ASSUMPTIONS.expRate),
    // The range counts only when both ends are there and in order.
    attackMin: low != null && high != null && high >= low ? low : null,
    attackMax: low != null && high != null && high >= low ? high : null,
    skillFactor: clamp(Number(raw?.skillFactor ?? base.skillFactor), SKILL_FACTOR_RANGE, DEFAULT_ASSUMPTIONS.skillFactor),
  };
}

function read(): Assumptions {
  try {
    return clean(JSON.parse(localStorage.getItem(KEY) ?? "null"), DEFAULT_ASSUMPTIONS);
  } catch {
    return { ...DEFAULT_ASSUMPTIONS };
  }
}

export const assumptions = $state<{ value: Assumptions }>({ value: read() });

let watching = false;

/** Follows changes made in other Leafline windows. */
export function watchAssumptions() {
  if (watching || typeof window === "undefined") return;
  watching = true;
  window.addEventListener("storage", (e) => {
    if (e.key === KEY) assumptions.value = read();
  });
}

export function setAssumptions(next: Partial<Assumptions>) {
  assumptions.value = clean(next, assumptions.value);
  try {
    localStorage.setItem(KEY, JSON.stringify(assumptions.value));
  } catch {
    // The estimates still use the values for this window.
  }
}

export function isDefault(a: Assumptions) {
  const d = DEFAULT_ASSUMPTIONS;
  return (
    a.respawnSeconds === d.respawnSeconds &&
    a.expRate === d.expRate &&
    a.attackMin === d.attackMin &&
    a.attackMax === d.attackMax &&
    a.skillFactor === d.skillFactor
  );
}
