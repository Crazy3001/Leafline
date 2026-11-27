<script lang="ts">
  // A lookup's results and the one opened from them. Where there's room (the desk's Look up and
  // Quests), they sit side by side: the list on the left, the open card on the right, each filling
  // the height left under what's above them and scrolling on its own, with its own scrollbar. A
  // window too short for that scrolls as one, the card kept in view. In a narrow window the card
  // takes the list's place, with a way back, as before.
  import { ArrowLeft } from "@lucide/svelte";
  import type { Snippet } from "svelte";

  let {
    open,
    onback,
    pinned = false,
    list,
    detail,
    empty,
  }: {
    /** Whether a result is open. */
    open: boolean;
    onback: () => void;
    pinned?: boolean;
    list: Snippet;
    detail: Snippet;
    /** What the right side says before anything is picked. */
    empty?: Snippet;
  } = $props();

  /** The width, in the page's units, from which list and card fit side by side. */
  const SPLIT_FROM = 880;

  let width = $state(0);
  const split = $derived(width >= SPLIT_FROM);

  let host = $state<HTMLElement | null>(null);
  let detailBox = $state<HTMLElement | null>(null);
  /** The height the card side may take: what the scrolling panel shows, so it all can be reached. */
  let paneHeight = $state<number | null>(null);
  /** The height both sides fill, down to the panel's foot, when that leaves them room. */
  let fill = $state<number | null>(null);
  /** Below this the two sides would be too short to scroll on their own: the panel scrolls instead. */
  const FILL_FROM = 320;

  function scrollerOf(node: HTMLElement): HTMLElement | null {
    for (let el = node.parentElement; el; el = el.parentElement) {
      const overflow = getComputedStyle(el).overflowY;
      if (overflow === "auto" || overflow === "scroll") return el;
    }
    return null;
  }

  $effect(() => {
    if (!split || !host) return;
    const scroller = scrollerOf(host);
    if (!scroller) {
      fill = null;
      return;
    }
    const at = host;
    const measure = () => {
      paneHeight = scroller.clientHeight;
      // Rects are in screen pixels and heights in the page's (text size zooms the page): the
      // scroller's two heights give the ratio between them.
      const ratio = scroller.getBoundingClientRect().height / (scroller.offsetHeight || 1) || 1;
      const top = (at.getBoundingClientRect().top - scroller.getBoundingClientRect().top) / ratio + scroller.scrollTop;
      const foot = parseFloat(getComputedStyle(scroller).paddingBottom) || 0;
      const left = Math.floor(scroller.clientHeight - top - foot);
      fill = left >= FILL_FROM ? left : null;
    };
    measure();
    // Whatever is above the sides growing or shrinking moves them, and every box between them and
    // the panel grows with it.
    const resized = new ResizeObserver(measure);
    resized.observe(scroller);
    for (let el = at.parentElement; el && el !== scroller; el = el.parentElement) resized.observe(el);
    return () => resized.disconnect();
  });

  // A new pick shows from its top.
  $effect(() => {
    void open;
    detailBox?.scrollTo?.({ top: 0 });
  });
</script>

<div class="host" bind:this={host} bind:clientWidth={width}>
  {#if split}
    <div class="split" class:fill={fill != null} style:height={fill != null ? `${fill}px` : null}>
      <div class="pane list-side">{@render list()}</div>
      <div class="pane detail-side" bind:this={detailBox} style:max-height={fill == null && paneHeight ? `${paneHeight - 16}px` : null}>
        {#if open}
          {@render detail()}
        {:else if empty}
          {@render empty()}
        {/if}
      </div>
    </div>
  {:else if open}
    <button class="back" class:pinned onclick={onback}><ArrowLeft size={12} />Back</button>
    {@render detail()}
  {:else}
    {@render list()}
  {/if}
</div>

<style>
  .host {
    min-width: 0;
  }

  .split {
    display: grid;
    grid-template-columns: minmax(300px, 2fr) minmax(380px, 3fr);
    align-items: start;
    gap: var(--space-3);
  }

  .pane {
    min-width: 0;
  }

  /* Stays in view while the list scrolls under it; scrolls on its own when taller than the panel. */
  .detail-side {
    position: sticky;
    top: var(--space-2);
    overflow-y: auto;
    overscroll-behavior: contain;
  }

  /* Filling the height: each side scrolls on its own, its scrollbar beside it. */
  .split.fill {
    align-items: stretch;
  }

  .split.fill .pane {
    min-height: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
    scrollbar-gutter: stable;
  }

  .split.fill .detail-side {
    position: static;
  }

  .back {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    margin: var(--space-0-5) var(--space-3) var(--space-1-5);
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--muted);
    font-size: var(--text-md);
    cursor: pointer;
  }

  .back.pinned {
    margin-inline: 0;
  }

  .back:hover,
  .back:focus-visible {
    color: var(--text);
    outline: none;
  }
</style>
