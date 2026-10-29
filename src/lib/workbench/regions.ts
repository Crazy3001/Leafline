import type { Analysis, Rect } from "$lib/types";

export type RegionKind = "hud" | "tooltip" | "quest" | "mesos" | "text";

export interface Region {
  label: string;
  kind: RegionKind;
  rect: Rect;
}

/** Everything the pipeline located, for drawing over the screenshot. */
export function regionsOf(analysis: Analysis, includeText: boolean): Region[] {
  const regions: Region[] = [];
  const hud = analysis.hudLayout;
  if (hud) {
    regions.push(
      { label: "HP gauge", kind: "hud", rect: hud.hp },
      { label: "MP gauge", kind: "hud", rect: hud.mp },
      { label: "EXP gauge", kind: "hud", rect: hud.exp },
      { label: "Gauge values", kind: "hud", rect: hud.labels },
      { label: "Job and name", kind: "hud", rect: hud.identity },
    );
    if (hud.level) regions.push({ label: "Level", kind: "hud", rect: hud.level });
  }
  analysis.tooltips.forEach((item, i) =>
    regions.push({ label: `Tooltip ${i + 1}: ${item.name || "?"}`, kind: "tooltip", rect: item.panel }),
  );
  if (analysis.quest) regions.push({ label: "Quest Helper", kind: "quest", rect: analysis.quest.panel });
  if (analysis.mesos) regions.push({ label: "Mesos", kind: "mesos", rect: analysis.mesos.rect });
  if (includeText) {
    for (const line of analysis.overview) regions.push({ label: line.text, kind: "text", rect: line.rect });
  }
  return regions;
}
