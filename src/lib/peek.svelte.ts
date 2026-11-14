// Link cards: a coloured name opens a small card beside it instead of jumping to another tab.
// One card shows at a time. A link inside the card opens the next card over it, with a way back;
// Esc or a click outside closes it. Each card has a button to the full view in its tab. A card
// can be dragged by its header, and pinned: a pinned card stays where it is, beside the next ones,
// until its own close button, so several can sit side by side to compare.

import { backend } from "./api";
import { showItemNamed } from "./items.svelte";
import { showMonsterNamed } from "./monsters.svelte";
import { showMapNamed, showNpcNamed } from "./places.svelte";

export type PeekKind = "item" | "mob" | "npc" | "map" | "quest" | "skill";

export interface PeekTarget {
  kind: PeekKind;
  /** Null when the link knows only a name. */
  id: number | null;
  /** The name shown, or for a map a session's `Street : Name` label. */
  name: string | null;
}

export interface PeekAnchor {
  left: number;
  top: number;
  bottom: number;
  width: number;
}

/** A card pinned in place, by where it sits in the window. */
export interface PinnedCard {
  key: number;
  target: PeekTarget;
  left: number;
  top: number;
  /** Stacking order: the card last pressed is on top. */
  z: number;
}

export const peek = $state<{
  stack: PeekTarget[];
  anchor: PeekAnchor | null;
  /** How far the open card has been dragged from where it opened. */
  drag: { x: number; y: number };
  pinned: PinnedCard[];
  /** The open card's stacking order, against the pinned ones. */
  z: number;
}>({ stack: [], anchor: null, drag: { x: 0, y: 0 }, pinned: [], z: 0 });

let topZ = 0;

/** Brings a card to the top: a pinned one by its key, or the open card with null. */
export function raiseCard(key: number | null) {
  topZ += 1;
  if (key == null) peek.z = topZ;
  else {
    const card = peek.pinned.find((p) => p.key === key);
    if (card) card.z = topZ;
  }
}

let pinKey = 0;

/** Pins the open card where it is, and closes it as the open card. */
export function pinPeek(left: number, top: number) {
  const target = peek.stack.at(-1);
  if (!target) return;
  peek.pinned.push({ key: ++pinKey, target, left, top, z: ++topZ });
  closePeek();
}

/** The page a card's own window shows: `card/<kind>/<id>`, or `n-<name>` when only a name is known. */
export function cardPath(target: PeekTarget): string | null {
  if (target.id != null) return `card/${target.kind}/${target.id}`;
  return target.name ? `card/${target.kind}/n-${encodeURIComponent(target.name)}` : null;
}

export function unpinCard(key: number) {
  peek.pinned = peek.pinned.filter((p) => p.key !== key);
}

/** Whether a link sits inside the open card, where it opens the next card instead of a new one. */
function inCard(from: Element | null): boolean {
  return !!from?.closest("[data-peek]");
}

function anchorOf(from: Element | null): PeekAnchor | null {
  if (!from) return null;
  const r = from.getBoundingClientRect();
  return { left: r.left, top: r.top, bottom: r.bottom, width: r.width };
}

/** Opens a card for `target` beside the link `from`, or over the open card when the link is in it. */
export function openPeek(target: PeekTarget, from: Element | null) {
  if (inCard(from) && peek.stack.length) {
    const top = peek.stack.at(-1)!;
    if (top.kind === target.kind && top.id === target.id && top.name === target.name) return;
    peek.stack.push(target);
    return;
  }
  peek.stack = [target];
  peek.anchor = anchorOf(from);
  peek.drag = { x: 0, y: 0 };
  // A new card opens over every pinned one.
  topZ += 1;
  peek.z = topZ;
}

/** Opens the next card over the open one, from a link in it that isn't a coloured name. */
export function pushPeek(target: PeekTarget) {
  if (peek.stack.length) peek.stack.push(target);
  else peek.stack = [target];
}

export function peekBack() {
  peek.stack.pop();
  if (!peek.stack.length) peek.anchor = null;
}

export function closePeek() {
  peek.stack = [];
  peek.anchor = null;
  peek.drag = { x: 0, y: 0 };
}

/** Opens the full view in its tab, as links did before cards, and closes the card. */
export async function openFull(target: PeekTarget) {
  closePeek();
  const { id, name } = target;
  switch (target.kind) {
    case "item":
      return id != null ? backend.showItem(id, null) : name ? showItemNamed(name) : undefined;
    case "mob":
      return id != null ? backend.showMonster(id, null) : name ? showMonsterNamed(name) : undefined;
    case "npc":
      return id != null ? backend.showPlace("npcs", id) : name ? showNpcNamed(name) : undefined;
    case "map":
      return id != null ? backend.showPlace("maps", id) : name ? showMapNamed(name) : undefined;
    case "quest":
      return id != null ? backend.showQuest(id) : undefined;
    case "skill": {
      // Skills have no tab of their own in the overlay: Lookup → Skills shows them.
      const { lookup } = await import("./items.svelte");
      const { loadSkills, skillNamed, skillView } = await import("./skills.svelte");
      await loadSkills();
      skillView.id = id ?? skillNamed(name)?.id ?? null;
      lookup.mode = "skills";
      const { openPanel } = await import("./nav.svelte");
      return openPanel("item");
    }
  }
}

/** The context key a link card sets, so the sections inside it start folded. */
export const SECTION_SCOPE = Symbol("section scope");
