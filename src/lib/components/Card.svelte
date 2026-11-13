<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    title,
    aside,
    plain = false,
    children,
  }: {
    title: string;
    aside?: Snippet;
    /** Content only, without the frame and title: for a pinned panel, whose window names it. */
    plain?: boolean;
    children: Snippet;
  } = $props();
</script>

{#if plain}
  <section class="plain" aria-label={title}>
    {#if aside}<div class="plain-aside">{@render aside()}</div>{/if}
    {@render children()}
  </section>
{:else}
  <section class="card">
    <header class="strip">
      <span class="strip-title">{title}</span>
      {#if aside}<span class="aside">{@render aside()}</span>{/if}
    </header>
    {@render children()}
  </section>
{/if}

<style>
  .card {
    background: transparent;
    border-radius: 0;
    padding: 10px 10px 14px;
  }

  header {
    justify-content: space-between;
    margin: 0 0 12px;
  }

  .plain-aside {
    display: flex;
    justify-content: flex-end;
    gap: 6px;
    margin-bottom: 6px;
    color: var(--muted);
    font-size: var(--text-xs);
  }

  .aside {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--muted);
    font-size: var(--text-sm);
  }
</style>
