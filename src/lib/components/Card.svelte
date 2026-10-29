<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    title,
    aside,
    children,
  }: {
    title: string;
    aside?: Snippet;
    children: Snippet;
  } = $props();
</script>

<section class="card">
  <header>
    <span class="eyebrow">{title}</span>
    {#if aside}<span class="aside">{@render aside()}</span>{/if}
  </header>
  {@render children()}
</section>

<style>
  .card {
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 10px 12px 12px;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 8px;
  }

  .aside {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--muted);
    font-size: 11.5px;
  }
</style>
