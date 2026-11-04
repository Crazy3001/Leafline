// The monster search as the Lookup tab shows it. It lives here so that switching tabs keeps a
// search, and so a link from another window or panel can bring a monster up.

import { backend } from "./api";
import { lookup } from "./items.svelte";
import type { MonsterQuery } from "./types";

export const blankMonsterQuery = (): MonsterQuery => ({
  text: "",
  kind: null,
  weakTo: null,
  region: null,
  area: null,
  levelMin: null,
  levelMax: null,
  sort: null,
  near: null,
  offset: 0,
});

export const monsterView = $state<{ id: number | null; trail: number[]; query: MonsterQuery; seen: number }>({
  /** The monster open. */
  id: null,
  /** The monsters opened on the way here, to go back through. */
  trail: [],
  query: blankMonsterQuery(),
  /** Goes up with each link followed, so the overlay can open the tab. */
  seen: 0,
});

let watching = false;

export function watchMonsters() {
  if (watching) return;
  watching = true;
  backend.onShowMonster((link) => {
    if (link.text != null) monsterView.query = { ...blankMonsterQuery(), text: link.text };
    monsterView.id = link.id;
    monsterView.trail = [];
    lookup.mode = "monsters";
    monsterView.seen += 1;
  });
}

/** Opens a monster from another monster or the list, so Back can return. */
export function openMonster(id: number) {
  if (monsterView.id != null && monsterView.id !== id) monsterView.trail.push(monsterView.id);
  monsterView.id = id;
}

export function backFromMonster() {
  monsterView.id = monsterView.trail.pop() ?? null;
}

/** Brings a monster up by name, or searches for the name if no monster goes by it. */
export async function showMonsterNamed(name: string) {
  const id = await backend.findMonster(name);
  await backend.showMonster(id, id == null ? name : null);
}
