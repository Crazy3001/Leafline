<script lang="ts">
  // The box filters open into. Each `label` or `.levels` row inside lines up on one grid.
  import type { Snippet } from "svelte";

  let { children }: { children: Snippet } = $props();
</script>

<div class="filters">{@render children()}</div>

<style>
  .filters {
    display: grid;
    gap: var(--space-1);
    margin-bottom: var(--space-2);
    padding: var(--space-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    font-size: var(--text-md);
  }

  .filters :global(label),
  .filters :global(.levels) {
    display: grid;
    grid-template-columns: 60px 1fr;
    align-items: center;
    gap: var(--space-1-5);
    color: var(--muted);
  }

  .filters :global(.levels) {
    grid-template-columns: 60px 1fr auto 1fr;
  }

  .filters :global(.to) {
    color: var(--faint);
  }

  .filters :global(select),
  .filters :global(input:not([type="checkbox"])) {
    min-width: 0;
    height: 24px;
    padding: 0 var(--space-1);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--text);
    font: inherit;
  }

  .filters :global(.check) {
    display: flex;
  }
</style>
