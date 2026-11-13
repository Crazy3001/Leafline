<script lang="ts">
  // One result of a lookup: a name with its picture, a line under it, and a short fact at the right.
  import Pic from "$lib/ui/Pic.svelte";
  import type { PictureKind } from "$lib/pictures";

  let {
    name,
    picture = null,
    line = null,
    aside = null,
    small = null,
    goal = false,
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
    onclick: () => void;
  } = $props();
</script>

<button class="row" {onclick}>
  <span class="name" class:goal
    >{#if picture}<Pic kind={picture.kind} id={picture.id} size={18} class="row-pic" />{/if}{name}{#if small}<small>{small}</small>{/if}</span
  >
  {#if aside}<span class="aside tabular">{aside}</span>{/if}
  {#if line}<span class="line">{line}</span>{/if}
</button>

<style>
  .row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    column-gap: 8px;
    align-items: baseline;
    width: 100%;
    padding: 5px 4px;
    border: 0;
    background: transparent;
    text-align: left;
    cursor: pointer;
  }

  .row:hover,
  .row:focus-visible {
    background: var(--surface-3);
    outline: none;
  }

  .name :global(.row-pic) {
    margin-right: 6px;
  }

  .name {
    overflow: hidden;
    font-size: var(--text-md);
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .name.goal {
    color: var(--accent-strong);
  }

  small {
    margin-left: 5px;
    color: var(--faint);
    font-size: var(--text-xs);
    font-weight: 500;
  }

  .aside {
    color: var(--muted);
    font-size: var(--text-xs);
    white-space: nowrap;
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
