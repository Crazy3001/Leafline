// The player's potion slots, shared by the Train tab and the training log, and the sum behind
// mesos per hour after potion costs. Names and prices are the player's own; nothing is bundled.

import { backend } from "./api";
import type { PotionSlot } from "./types";

export const potions = $state<{ slots: PotionSlot[]; loaded: boolean }>({ slots: [], loaded: false });

/** As many slots as the backend keeps. */
export const MAX_SLOTS = 8;

let watching = false;

export function watchPotions() {
  if (watching) return;
  watching = true;
  refresh();
  backend.onPotions(refresh);
}

function refresh() {
  backend
    .potionSlots()
    .then((slots) => {
      potions.slots = slots;
      potions.loaded = true;
    })
    .catch(() => (potions.loaded = true));
}

/**
 * Mesos per hour with the potions' cost taken out. The cost is spread over the whole active
 * time, since potions are drunk while hunting whether or not the mesos were in view.
 */
export function netMesosPerHour(mesosPerHour: number | null, cost: number, activeSeconds: number): number | null {
  if (mesosPerHour == null || activeSeconds <= 0) return null;
  return mesosPerHour - (cost * 3600) / activeSeconds;
}
