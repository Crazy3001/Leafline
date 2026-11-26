<script lang="ts">
  // A card with a coloured icon beside its title, and a link on the right to where it goes in full.
  // The desk view's Now is built of these; the colour says what kind of thing the card is about.
  import { ChevronRight } from "@lucide/svelte";
  import type { Component, Snippet } from "svelte";

  let {
    title,
    icon,
    tone = "leaf",
    more = null,
    onmore,
    aside,
    children,
    class: extra = "",
  }: {
    title: string;
    icon: Component<{ size?: number }>;
    /** The icon's colour: gold for EXP and you, leaf for steps, blue for time, rose for quests, violet for places. */
    tone?: "gold" | "leaf" | "blue" | "rose" | "violet";
    /** The link's words, e.g. "Train". */
    more?: string | null;
    onmore?: () => void;
    /** Instead of the link: something quiet on the right. */
    aside?: Snippet;
    children: Snippet;
    class?: string;
  } = $props();

  const Icon = $derived(icon);
</script>

<section class="tcard card {extra}" aria-label={title}>
  <header class="t">
    <span class="badge-ic {tone}" aria-hidden="true"><Icon size={17} /></span>
    <h2>{title}</h2>
    {#if aside}<span class="aside">{@render aside()}</span>{:else if more && onmore}<button class="more" onclick={onmore}>{more}<ChevronRight size={15} /></button>{/if}
  </header>
  {@render children()}
</section>

<style>
  .tcard {
    container: tcard / inline-size;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    min-width: 0;
    padding: var(--space-4) var(--space-5) var(--space-5);
    border-radius: var(--radius-lg);
  }

  .t {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
  }

  h2 {
    margin: 0;
    overflow: hidden;
    font-family: var(--font-display);
    font-size: var(--text-lg);
    font-weight: var(--weight-strong);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .badge-ic {
    display: grid;
    flex: none;
    place-items: center;
    width: 32px;
    height: 32px;
    border-radius: var(--radius);
  }

  .gold {
    background: color-mix(in srgb, var(--exp) 20%, transparent);
    color: var(--exp-text);
  }

  .leaf {
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .blue {
    background: color-mix(in srgb, var(--ref-item) 18%, transparent);
    color: var(--ref-item);
  }

  .rose {
    background: color-mix(in srgb, var(--ref-mob) 18%, transparent);
    color: var(--ref-mob);
  }

  .violet {
    background: color-mix(in srgb, var(--ref-map) 18%, transparent);
    color: var(--ref-map);
  }

  .aside {
    margin-left: auto;
    color: var(--faint);
    font-size: var(--text-xs);
    white-space: nowrap;
  }

  .more {
    display: inline-flex;
    flex: none;
    align-items: center;
    gap: var(--space-0-5);
    margin-left: auto;
    padding: var(--space-1) var(--space-1-5);
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--accent-text);
    font: inherit;
    font-size: var(--text-xs);
    font-weight: var(--weight-strong);
    cursor: pointer;
  }

  .more:hover {
    background: var(--hover);
  }

  .more:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }
</style>
