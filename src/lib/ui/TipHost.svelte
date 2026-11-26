<script lang="ts">
  // The one tooltip a window shows, just below and right of the mouse, following it across the
  // element; flipped to the other side where there's no room, and always inside the window. Without
  // a mouse place it sits under the element. See `$lib/tip.svelte`.
  import { untrack } from "svelte";

  import { hideTip, tipState } from "$lib/tip.svelte";
  import { pageZoom } from "$lib/zoom";

  const MARGIN = 6;
  const GAP = 6;
  /** How far from the mouse's tip: clear of the pointer, close enough to read as its own. */
  const FROM_MOUSE = { x: 14, y: 18 };

  let box = $state<HTMLElement | null>(null);
  let place = $state<{ left: number; top: number } | null>(null);

  /** Where the tooltip goes, wholly inside the window, in the page's units (see `pageZoom`). */
  function placeFor(anchor: HTMLElement, mouse: { x: number; y: number } | null, tipBox: HTMLElement) {
    const z = pageZoom();
    const width = tipBox.offsetWidth;
    const height = tipBox.offsetHeight;
    const vw = window.innerWidth / z;
    const vh = window.innerHeight / z;
    const at = mouse && { x: mouse.x / z, y: mouse.y / z };
    let left: number;
    let top: number;
    if (at) {
      // Right of the mouse, or left of it when that runs off; below, or above when that does.
      left = at.x + FROM_MOUSE.x + width <= vw - MARGIN ? at.x + FROM_MOUSE.x : at.x - FROM_MOUSE.x - width;
      top = at.y + FROM_MOUSE.y + height <= vh - MARGIN ? at.y + FROM_MOUSE.y : at.y - GAP - height;
    } else {
      const r = anchor.getBoundingClientRect();
      const a = { left: r.left / z, top: r.top / z, bottom: r.bottom / z, width: r.width / z };
      left = a.left + a.width / 2 - width / 2;
      const below = a.bottom + GAP;
      top = below + height <= vh - MARGIN || a.top - GAP - height < MARGIN ? below : a.top - GAP - height;
    }
    return {
      left: Math.max(Math.min(left, vw - width - MARGIN), MARGIN),
      top: Math.max(Math.min(top, vh - height - MARGIN), MARGIN),
    };
  }

  $effect(() => {
    const anchor = tipState.anchor;
    void tipState.body;
    place = null;
    if (!anchor || !box) return;
    const tipBox = box;
    const measure = () => {
      if (!anchor.isConnected) return hideTip();
      place = placeFor(anchor, tipState.at, tipBox);
    };
    const frame = requestAnimationFrame(measure);
    // A hover card fills in once its lookup answers, and grows: placed again to stay in the window.
    const resized = new ResizeObserver(() => place && measure());
    resized.observe(tipBox);
    return () => {
      cancelAnimationFrame(frame);
      resized.disconnect();
    };
  });

  // Following the mouse: moved without hiding first, so it doesn't flicker.
  $effect(() => {
    const at = tipState.at;
    const anchor = tipState.anchor;
    if (!at || !anchor || !box || !untrack(() => place)) return;
    place = placeFor(anchor, at, box);
  });

  $effect(() => {
    if (!tipState.anchor) return;
    const away = () => hideTip();
    window.addEventListener("scroll", away, true);
    window.addEventListener("wheel", away, { passive: true });
    window.addEventListener("keydown", away);
    window.addEventListener("resize", away);
    window.addEventListener("blur", away);
    return () => {
      window.removeEventListener("scroll", away, true);
      window.removeEventListener("wheel", away);
      window.removeEventListener("keydown", away);
      window.removeEventListener("resize", away);
      window.removeEventListener("blur", away);
    };
  });
</script>

{#if tipState.anchor && tipState.body}
  {@const body = tipState.body}
  <div
    class="tip"
    class:words={typeof body === "string"}
    role="tooltip"
    bind:this={box}
    style:left="{place?.left ?? 0}px"
    style:top="{place?.top ?? 0}px"
    style:visibility={place ? "visible" : "hidden"}
  >
    {#if typeof body === "string"}{body}{:else}{@render body()}{/if}
  </div>
{/if}

<style>
  .tip {
    position: fixed;
    z-index: 2000;
    max-width: min(280px, calc(100vw - 12px));
    padding: var(--space-1-5) var(--space-2);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface-raised);
    box-shadow: var(--shadow-lg);
    color: var(--text);
    font-size: var(--text-xs);
    line-height: 1.45;
    pointer-events: none;
    animation: appear 90ms ease-out;
  }

  .tip.words {
    white-space: pre-line;
  }

  /* A tooltip with a picture or a list reads as a small card. */
  .tip:not(.words) {
    min-width: 160px;
    max-width: min(300px, calc(100vw - 12px));
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius);
  }

  @keyframes appear {
    from {
      opacity: 0;
    }
  }
</style>
