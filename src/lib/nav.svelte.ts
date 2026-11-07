// Lets a panel ask the overlay to open another tab, such as the Character tab's link to Train.

import { backend } from "./api";
import type { Panel } from "./types";

export const tabRequest = $state<{ panel: Panel | null; seq: number }>({ panel: null, seq: 0 });

export function openPanel(panel: Panel) {
  tabRequest.panel = panel;
  tabRequest.seq += 1;
}

/** Set by the desk view's page: in that window the training log opens in a column. */
export const deskWindow = { active: false };

export const logRequest = $state({ seq: 0 });

/** Shows the training log: in the desk view when the link is in it, else in its own window. */
export function openTrainingLog() {
  if (deskWindow.active) logRequest.seq += 1;
  else void backend.showTraining();
}
