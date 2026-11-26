// Next up: a short list of what's worth doing now, the best first. Built only from what Leafline
// already reads and knows: quests whose steps are all done (to hand in), AP the Character Stat
// window shows unspent, and the next quest objective with the map that has most of it. Shared by
// Now's Next up card and the desk's compass, which heads for the first of them with a place.
// Suggestions to act on, never alerts.

import { untrack } from "svelte";

import { backend } from "./api";
import { damage, followDamage } from "./damage.svelte";
import { mapPlace, npcPlace } from "./destination.svelte";
import { groundDistance, questTitle } from "./format";
import { mapData, watchMapData } from "./mapdata.svelte";
import { questLog, readyToHandIn, watchQuestLog } from "./questlog.svelte";
import { huntOrder, orderGrounds } from "./quests.svelte";
import { live } from "./readings.svelte";
import type { Destination, Guide, MapRef, QuestSheet } from "./types";

export type NextStep = {
  key: string;
  kind: "handin" | "ap" | "hunt";
  title: string;
  npc?: string | null;
  map?: MapRef | null;
  detail?: string;
  /** Where to go for it; none when it is here, or for AP. */
  place?: Destination | null;
};

/** Where each tracked quest's objectives are, and who takes it in; and the quest log's ready ones. */
const known = $state<{ guide: Guide | null; sheets: Record<number, QuestSheet> }>({ guide: null, sheets: {} });

const ready = () => mapData.status?.ready ?? false;
const tracked = () => (live.analysis?.quest?.quests ?? []).filter((q) => q.title);
const logReady = () => (questLog.log?.quests ?? []).filter(readyToHandIn).slice(0, 2);
const shownName = () => live.analysis?.hud?.name ?? live.session?.name ?? null;

let watching = false;

/** Starts looking up what the steps need. Call from any component that shows them. */
export function watchNextUp() {
  if (watching) return;
  watching = true;
  watchMapData();
  watchQuestLog();
  $effect.root(() => {
    $effect(() => {
      const who = shownName();
      untrack(() => followDamage(who));
    });

    $effect(() => {
      const map = live.analysis?.map ?? null;
      const quests = tracked();
      const key = JSON.stringify([quests.map((q) => [q.title, q.objectives.map((o) => o.target)]), map?.name, map?.street, map?.twin]);
      void mapData.status?.dataUpdatedAt;
      if (!ready() || !quests.length) {
        known.guide = null;
        return;
      }
      const asked = quests.map((q) => ({ title: q.title, objectives: q.objectives.map((o) => o.target) }));
      untrack(() =>
        backend
          .questGuide(asked, map?.name ?? null, map?.street ?? null, map?.twin ?? null, [])
          .then((g) => {
            if (key === lastGuideKey) known.guide = g;
          })
          .catch(() => (known.guide = null)),
      );
      lastGuideKey = key;
    });

    $effect(() => {
      const list = logReady();
      const asked = list.map((q) => q.questId ?? "").join(",");
      if (!ready()) return;
      lastReadyKey = asked;
      untrack(() => {
        const ids = list.map((q) => q.questId).filter((id): id is number => id != null && !known.sheets[id]);
        Promise.all(ids.map((id) => backend.questSheet(id, null, null, null)))
          .then((found) => {
            if (asked !== lastReadyKey) return;
            const next = { ...known.sheets };
            found.forEach((w) => {
              if (w) next[w.sheet.row.id] = w.sheet;
            });
            known.sheets = next;
          })
          .catch(() => {});
      });
    });
  });
}

let lastGuideKey = "";
let lastReadyKey = "";

/** What's worth doing now, the best first, at most four. Reactive: read it inside `$derived`. */
export function nextSteps(): NextStep[] {
  const out: NextStep[] = [];
  const seen = new Set<string>();
  const quests = tracked();
  const guide = known.guide;

  // Quests the Quest Helper shows with every step done.
  quests.forEach((q, i) => {
    const done = q.objectives.length > 0 && q.objectives.every((o) => !o.unread && o.current >= o.required);
    if (!done) return;
    const info = guide?.quests[i]?.quest ?? null;
    const npc = info?.turnIn ?? info?.giver ?? null;
    const maps = info?.turnIn ? info.turnInMaps : (info?.giverMaps ?? []);
    seen.add(q.title.toLowerCase());
    out.push({ key: `h:${q.title}`, kind: "handin", title: `Hand in ${questTitle(info?.quest.name ?? q.title).name}`, npc, map: maps[0] ?? null, place: npc ? npcPlace(npc, maps) : null });
  });
  // And the quest log's, as of the last look at the Quest window.
  for (const q of logReady()) {
    if (seen.has(q.title.toLowerCase())) continue;
    const info = q.questId != null ? known.sheets[q.questId]?.info : null;
    const npc = info?.turnIn ?? info?.giver ?? null;
    const maps = info?.turnIn ? info.turnInMaps : (info?.giverMaps ?? []);
    out.push({ key: `l:${q.key}`, kind: "handin", title: `Hand in ${questTitle(q.title).name}`, npc, map: maps[0] ?? null, place: npc ? npcPlace(npc, maps) : null });
  }

  // AP the Character Stat window last showed unspent.
  const name = shownName();
  const stats = damage.name && name && damage.name.toLowerCase() === name.toLowerCase() ? damage.stats : null;
  const ap = stats?.sheet?.abilityPoints ?? stats?.base?.abilityPoints ?? null;
  if (ap && ap > 0) out.push({ key: "ap", kind: "ap", title: `Spend ${ap} AP`, detail: "Your build on Me says where" });

  // The next objective still to do, with the map that has the most of it.
  outer: for (const [i, q] of quests.entries()) {
    for (const o of q.objectives) {
      if (o.unread || o.current >= o.required) continue;
      const g = guide?.quests[i]?.objectives.find((x) => x.target === o.target);
      const hunt = g?.hunts.find((h) => h.monster.known && h.maps.length);
      if (!g || !hunt) continue;
      const best = orderGrounds(hunt.maps, huntOrder.value)[0];
      const left = o.required - o.current;
      const verb = g.kind === "item" ? "Collect" : "Defeat";
      out.push({
        key: `o:${q.title}:${o.target}`,
        kind: "hunt",
        title: `${verb} ${left} more ${o.target}`,
        map: best.map,
        detail: best.hops === 0 ? "On this map" : groundDistance(best, guide?.from != null),
        place: best.hops === 0 ? null : mapPlace(best.map),
      });
      break outer;
    }
  }
  return out.slice(0, 4);
}
