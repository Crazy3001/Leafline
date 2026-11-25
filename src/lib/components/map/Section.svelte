<script lang="ts" module>
  /** Set to "cards" by a page whose sections each stand as a card of their own: the Map tab. */
  export const SECTION_LOOK = Symbol("section look");
</script>

<script lang="ts">
  // A collapsible part of the map card. Opening or closing one is remembered by its title;
  // until then `open` decides. Inside a link card every section starts folded, so the card opens
  // as a short list of headings, every time: nothing opened in a card is remembered.
  import { ChevronRight } from "@lucide/svelte";
  import { getContext, setContext, type Snippet } from "svelte";

  import { SECTION_SCOPE } from "$lib/peek.svelte";

  let {
    title,
    count,
    open: startOpen = true,
    memory,
    anchor,
    aside,
    children,
  }: {
    title: string;
    count?: number | string;
    open?: boolean;
    /** What the open state is remembered by, when the title alone would share it too widely. */
    memory?: string;
    /** An id to jump to, where a row of chips leads to the sections. */
    anchor?: string;
    aside?: Snippet;
    children: Snippet;
  } = $props();

  const inCard = getContext(SECTION_SCOPE) === "card";
  /** A card of its own, not a band inside one. Sections inside it are its parts again. */
  const own = !inCard && getContext(SECTION_LOOK) === "cards";
  if (own) setContext(SECTION_LOOK, "parts");
  // svelte-ignore state_referenced_locally
  const key = `leafline.section.${memory ?? title}`;

  function remembered() {
    try {
      const value = localStorage.getItem(key);
      return value === null ? null : value === "true";
    } catch {
      return null;
    }
  }

  // svelte-ignore state_referenced_locally
  let open = $state(inCard ? false : (remembered() ?? startOpen));

  function toggle() {
    open = !open;
    if (inCard) return;
    try {
      localStorage.setItem(key, String(open));
    } catch {
      // A convenience only.
    }
  }
</script>

<section class="section" class:card={own} class:own id={anchor}>
  <button class="head" aria-expanded={open} onclick={toggle}>
    <span class="chevron" class:open><ChevronRight size={12} /></span>
    <span class="title">{title}</span>
    {#if count != null}<span class="count tabular">{count}</span>{/if}
    {#if aside}<span class="aside">{@render aside()}</span>{/if}
  </button>
  {#if open}
    <div class="body">{@render children()}</div>
  {/if}
</section>

<style>
  .section {
    margin-top: 12px;
  }

  /* A part inside a section: a band like the section's own, set in from its edges. */
  .head {
    display: flex;
    align-items: center;
    gap: 5px;
    width: 100%;
    min-height: 28px;
    padding: 3px 10px 3px 6px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    cursor: pointer;
    text-align: left;
    transition: background 0.12s ease;
  }

  .head:hover {
    background: var(--surface-3);
  }

  .chevron {
    display: grid;
    color: var(--faint);
    transition: transform 0.15s ease;
  }

  .chevron.open {
    transform: rotate(90deg);
  }

  .title {
    font-family: var(--font-display);
    font-size: var(--text-md);
    font-weight: var(--weight-strong);
    letter-spacing: 0.01em;
    color: var(--text);
  }

  .head:hover .title,
  .head:focus-visible .title {
    color: var(--accent-text-strong);
  }

  .head:focus-visible {
    outline: 1px solid var(--accent);
    outline-offset: -1px;
  }

  .count {
    padding: 0 7px;
    border-radius: var(--radius-pill);
    background: var(--hover);
    font-size: var(--text-xs);
    color: var(--faint);
  }

  .aside {
    margin-left: auto;
    font-size: var(--text-xs);
    color: var(--faint);
  }

  .body {
    margin-top: 8px;
  }

  /* Standing as its own card: the heading is the card's title row, with no band around it. */
  .own {
    margin: 0 10px 10px;
    scroll-margin-top: 50px;
  }

  .own > .head {
    min-height: 42px;
    padding: 8px 14px 8px 10px;
    border: 0;
    border-radius: calc(var(--radius-lg) - var(--line-w));
    background: none;
  }

  .own > .head:hover {
    background: var(--hover);
  }

  .own > .head .title {
    font-size: var(--text-lg);
    font-weight: var(--weight-strong);
  }

  .own > .body {
    margin: 0;
    padding: 2px 14px 14px;
  }
</style>
