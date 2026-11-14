// Link cards: a coloured name opens a small card beside it instead of jumping to another tab.
// One card shows at a time. A link inside the card opens the next card over it, with a way back;
// Esc or a click outside closes it. Each card has a button to the full view in its tab.

import { backend } from "./api";
import { showItemNamed } from "./items.svelte";
import { showMonsterNamed } from "./monsters.svelte";
import { showMapNamed, showNpcNamed } from "./places.svelte";

export type PeekKind = "item" | "mob" | "npc" | "map" | "quest";

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

export const peek = $state<{ stack: PeekTarget[]; anchor: PeekAnchor | null }>({ stack: [], anchor: null });

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
  }
}

/** The context key a link card sets, so the sections inside it start folded. */
export const SECTION_SCOPE = Symbol("section scope");
