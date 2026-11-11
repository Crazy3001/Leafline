// What the training estimates take as given because the map data doesn't say: how soon a
// defeated monster is back, and how much EXP a kill gives over its base value. Neither is a known
// fact about Classic, so the player can change both. Kept in this PC's browser storage and
// shared by every Leafline window.

import type { Assumptions } from "./types";

const KEY = "leafline.train.assumptions";

export const DEFAULT_ASSUMPTIONS: Assumptions = { respawnSeconds: 8, expRate: 1 };

export const RESPAWN_RANGE: [number, number] = [1, 600];
export const EXP_RATE_RANGE: [number, number] = [0.1, 20];

function clamp(value: number, [low, high]: [number, number], fallback: number) {
  return Number.isFinite(value) ? Math.min(high, Math.max(low, value)) : fallback;
}

function read(): Assumptions {
  try {
    const raw = JSON.parse(localStorage.getItem(KEY) ?? "null");
    return {
      respawnSeconds: clamp(Number(raw?.respawnSeconds), RESPAWN_RANGE, DEFAULT_ASSUMPTIONS.respawnSeconds),
      expRate: clamp(Number(raw?.expRate), EXP_RATE_RANGE, DEFAULT_ASSUMPTIONS.expRate),
    };
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
  assumptions.value = {
    respawnSeconds: clamp(next.respawnSeconds ?? assumptions.value.respawnSeconds, RESPAWN_RANGE, DEFAULT_ASSUMPTIONS.respawnSeconds),
    expRate: clamp(next.expRate ?? assumptions.value.expRate, EXP_RATE_RANGE, DEFAULT_ASSUMPTIONS.expRate),
  };
  try {
    localStorage.setItem(KEY, JSON.stringify(assumptions.value));
  } catch {
    // The estimates still use the values for this window.
  }
}

export function isDefault(a: Assumptions) {
  return a.respawnSeconds === DEFAULT_ASSUMPTIONS.respawnSeconds && a.expRate === DEFAULT_ASSUMPTIONS.expRate;
}
