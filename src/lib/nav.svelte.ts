// Lets a panel ask the overlay to open another tab, such as the Character tab's link to Train.

import type { Panel } from "./types";

export const tabRequest = $state<{ panel: Panel | null; seq: number }>({ panel: null, seq: 0 });

export function openPanel(panel: Panel) {
  tabRequest.panel = panel;
  tabRequest.seq += 1;
}
