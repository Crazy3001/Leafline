<script lang="ts">
  // One section of a panel: a heading with an optional aside, then its content, ruled off from
  // the next. Panels are a stack of these.
  import type { Snippet } from "svelte";

  let {
    title,
    aside,
    last = false,
    children,
  }: { title?: string; aside?: Snippet; last?: boolean; children: Snippet } = $props();
</script>

<section class="block" class:last>
  {#if title || aside}
    <header>
      {#if title}<h2>{title}</h2>{/if}
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
    padding: var(--space-3);
    border-bottom: 1px solid var(--border);
  }

  .last {
    border-bottom: 0;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    min-height: 20px;
  }

  h2 {
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--text-md);
    font-weight: 400;
    color: var(--muted);
  }

  .aside {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: var(--text-xs);
    color: var(--muted);
  }
</style>
