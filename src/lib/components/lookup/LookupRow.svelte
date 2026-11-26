<script lang="ts">
  // One result of a lookup: a name with its picture, a line under it, and a short fact at the right.
  import type { Snippet } from "svelte";
  import { clipped } from "$lib/tip.svelte";
  import { isWorn } from "$lib/builds.svelte";
  import Pic from "$lib/ui/Pic.svelte";
  import type { PictureKind } from "$lib/pictures";

  let {
    name,
    picture = null,
    line = null,
    aside = null,
    small = null,
    goal = false,
    selected = false,
    below,
    onclick,
  }: {
    name: string;
    /** What the row's picture shows and its id, when it has one. */
    picture?: { kind: PictureKind; id: number } | null;
    line?: string | null;
    aside?: string | null;
    /** A mark beside the name, such as M or F. */
    small?: string | null;
    /** One of the build's own goals. */
    goal?: boolean;
    /** The one open beside the list. */
    selected?: boolean;
    /** More under the line, such as a map's monsters. */
    below?: Snippet;
    onclick: () => void;
  } = $props();

  /** Gear the character wears shows in its own colour here too. */
  const wearing = $derived(picture?.kind === "item" && isWorn(picture.id, name));
</script>

<button class="row nav-row" class:selected aria-current={selected} {onclick}>
  <span use:clipped class="name" class:goal class:wearing
    >{#if picture}<Pic kind={picture.kind} id={picture.id} size={18} class="row-pic" />{/if}{name}{#if small}<small>{small}</small>{/if}{#if wearing}<small class="worn-tag">Wearing</small>{/if}</span
  >
  {#if aside}<span class="aside tabular">{aside}</span>{/if}
  {#if line}<span class="line">{line}</span>{/if}
  {#if below}<span class="below">{@render below()}</span>{/if}
</button>

<style>
  .row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    column-gap: var(--space-2);
    align-items: baseline;
    width: 100%;
    padding: var(--space-1-5) var(--space-6) var(--space-1-5) var(--space-1-5);
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    text-align: left;
    cursor: pointer;
  }

  .row:hover,
  .row:focus-visible {
    background: var(--surface-3);
    outline: none;
  }

  .row.selected {
    background: var(--accent-soft);
  }

  .name :global(.row-pic) {
    margin-right: var(--space-1-5);
  }

  .name {
    overflow: hidden;
    font-size: var(--text-md);
    font-weight: var(--weight-strong);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .name.goal {
    color: var(--accent-text-strong);
  }

  .name.wearing {
    color: var(--ref-worn);
  }

  small.worn-tag {
    color: var(--ref-worn);
  }

  small {
    margin-left: var(--space-1-5);
    color: var(--faint);
    font-size: var(--text-xs);
    font-weight: var(--weight-body);
  }

  .aside {
    color: var(--muted);
    font-size: var(--text-xs);
    white-space: nowrap;
  }

  .below {
    grid-column: 1 / -1;
    min-width: 0;
    margin-top: var(--space-0-5);
  }

  .line {
    grid-column: 1 / -1;
    overflow: hidden;
    color: var(--muted);
    font-size: var(--text-xs);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
