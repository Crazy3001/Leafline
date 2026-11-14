// The one tooltip: `use:tip={"words"}`, or `use:tip={snippet}` for a richer one. Each window's
// layout holds a `TipHost` that shows it beside the element under the mouse, after a short wait.
// A coloured name already says it opens a card, so a tooltip never needs to say that.
import type { Snippet } from "svelte";

/** What a tooltip shows: words (a newline starts a new line), or a snippet. */
export type TipBody = string | Snippet;
/** Shown only while `when` holds, checked as the mouse arrives: while a name is cut short, say. */
export interface TipWhen {
  body: TipBody;
  when: (node: HTMLElement) => boolean;
}
/** Nothing, an empty string or false: no tooltip. */
export type Tip = TipBody | TipWhen | null | undefined | false;

/** How long the mouse rests on an element before its tooltip shows. */
const DELAY = 420;
/** Moving straight on to another element while one shows, the next one shows at once. */
const WARM = 350;

// Plain state: an element and a snippet are kept as they are, not wrapped.
export const tipState = $state<{ body: TipBody | null; anchor: HTMLElement | null }>({ body: null, anchor: null });

let timer: ReturnType<typeof setTimeout> | undefined;
let hiddenAt = 0;

export function hideTip() {
  clearTimeout(timer);
  if (tipState.anchor) hiddenAt = performance.now();
  tipState.body = null;
  tipState.anchor = null;
}

function usable(body: Tip): body is TipBody | TipWhen {
  if (body && typeof body === "object") return usable(body.body);
  return typeof body === "function" || (typeof body === "string" && body.trim() !== "");
}

/** What shows for `node` now, if anything. */
function bodyFor(node: HTMLElement, tip: Tip): TipBody | null {
  if (!usable(tip)) return null;
  if (typeof tip === "object") return tip.when(node) ? tip.body : null;
  return tip;
}

/** The element's text runs past its box, cut off with an ellipsis. */
export function isClipped(node: HTMLElement) {
  return node.scrollWidth > node.clientWidth + 1;
}

/**
 * The element's own text as its tooltip, only while it is cut short: `use:clipped` on a name
 * that ends in an ellipsis in a narrow window. `text` stands in for the element's text.
 */
export function clipped(node: HTMLElement, text?: string | null) {
  const body = (t?: string | null): Tip => ({ body: (t ?? node.textContent ?? "").trim() || " ", when: isClipped });
  const action = tip(node, body(text));
  return {
    update: (next?: string | null) => action.update(body(next)),
    destroy: action.destroy,
  };
}

export function tip(node: HTMLElement, body: Tip) {
  let current = body;

  const show = () => {
    const body = bodyFor(node, current);
    if (!body || !node.isConnected) return;
    tipState.anchor = node;
    tipState.body = body;
  };
  const enter = (e: PointerEvent) => {
    if (e.pointerType === "touch" || !usable(current)) return;
    clearTimeout(timer);
    const warm = tipState.anchor != null || performance.now() - hiddenAt < WARM;
    timer = setTimeout(show, warm ? 40 : DELAY);
  };
  const leave = () => {
    clearTimeout(timer);
    if (tipState.anchor === node) hideTip();
  };
  // A click is the answer to the element; its tooltip goes, and doesn't come back on that hover.
  const press = () => {
    leave();
    hiddenAt = 0;
  };

  node.addEventListener("pointerenter", enter);
  node.addEventListener("pointerleave", leave);
  node.addEventListener("pointerdown", press);

  return {
    update(next: Tip) {
      current = next;
      if (tipState.anchor !== node) return;
      const body = bodyFor(node, next);
      if (body) tipState.body = body;
      else hideTip();
    },
    destroy() {
      leave();
      node.removeEventListener("pointerenter", enter);
      node.removeEventListener("pointerleave", leave);
      node.removeEventListener("pointerdown", press);
    },
  };
}
