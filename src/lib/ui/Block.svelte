<script lang="ts">
  // One section of a panel: a heading with an optional aside, then its content, ruled off from
  // the next. Panels are a stack of these.
  import type { Snippet } from "svelte";

  let {
    id,
    title,
    aside,
    last = false,
    children,
  }: { id?: string; title?: string; aside?: Snippet; last?: boolean; children: Snippet } = $props();
</script>

<section {id} class="block" class:last>
  {#if title || aside}
    <header class="strip">
      {#if title}<h2 class="strip-title">{title}</h2>{/if}
      {#if aside}<span class="aside">{@render aside()}</span>{/if}
    </header>
  {/if}
  {@render children()}
</section>

<style>
  .block {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 0 var(--space-3) 14px;
  }

  /* A block with no heading still keeps its space from the one above. */
  .block:not(:has(> header)) {
    padding-top: var(--space-3);
    border-top: 1px solid var(--border-strong);
  }

  header {
    justify-content: space-between;
    margin: 0 calc(-1 * var(--space-3));
  }

  h2 {
    margin: 0;
  }

  .aside {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: var(--text-xs);
    color: var(--muted);
  }
</style>
