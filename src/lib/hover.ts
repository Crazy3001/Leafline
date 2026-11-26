// What a link's hover card shows, looked up once per link and kept: resting the mouse on the same
// name again shows it at once. Each lookup asks for the least the card needs; the full card a
// click opens asks for the rest.

import { backend } from "./api";
import { splitMapLabel } from "./format";
import type { PeekKind, PeekTarget } from "./peek.svelte";
import type { ItemRecord, ItemWhere, MapDetails, MonsterSheet, NpcSheet, QuestSheet } from "./types";

export type HoverData =
  | { kind: "item"; record: ItemRecord; origin: ItemWhere["origin"] | null }
  | { kind: "mob"; sheet: MonsterSheet }
  | { kind: "npc"; sheet: NpcSheet }
  | { kind: "map"; details: MapDetails }
  | { kind: "quest"; sheet: QuestSheet };

/** Kinds whose hover card is looked up here. Skills come from the skill list already loaded. */
export type LookedUp = Exclude<PeekKind, "skill">;

const cache = new Map<string, Promise<HoverData | null>>();
/** The most lookups kept: enough for a session of reading, small enough not to matter. */
const KEEP = 300;

export function hoverData(target: PeekTarget & { kind: LookedUp }): Promise<HoverData | null> {
  const key = `${target.kind}:${target.id ?? `n:${target.name ?? ""}`}`;
  let found = cache.get(key);
  if (!found) {
    found = lookUp(target).catch(() => null);
    // A failed lookup is tried again next time, in case the data was still loading.
    found.then((data) => data ?? cache.delete(key));
    if (cache.size >= KEEP) cache.delete(cache.keys().next().value!);
    cache.set(key, found);
  }
  return found;
}

async function lookUp(target: PeekTarget & { kind: LookedUp }): Promise<HoverData | null> {
  const { id, name } = target;
  switch (target.kind) {
    case "item": {
      const record = id != null ? await backend.itemRecord(id) : name ? await backend.findItem(name, null) : null;
      if (!record) return null;
      const origin = await backend.itemOrigin(record.id, null, null, null).catch(() => null);
      return { kind: "item", record, origin: origin?.origin ?? null };
    }
    case "mob": {
      const mob = id ?? (name ? await backend.findMonster(name) : null);
      const where = mob == null ? null : await backend.monsterSheet(mob, null, null, null, []);
      return where ? { kind: "mob", sheet: where.sheet } : null;
    }
    case "npc": {
      const npc = id ?? (name ? await backend.findNpc(name) : null);
      const where = npc == null ? null : await backend.npcSheet(npc, null, null, null);
      return where ? { kind: "npc", sheet: where.sheet } : null;
    }
    case "map": {
      let map = id;
      if (map == null && name) {
        const label = splitMapLabel(name);
        map = (await backend.findMap(label.name, label.street, null))?.card.map.id ?? null;
      }
      const details = map == null ? null : await backend.mapCard(map);
      return details ? { kind: "map", details } : null;
    }
    case "quest": {
      const where = id == null ? null : await backend.questSheet(id, null, null, null);
      return where ? { kind: "quest", sheet: where.sheet } : null;
    }
  }
}
