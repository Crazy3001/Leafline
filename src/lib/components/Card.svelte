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
  /* The frame itself is the shared .card in app.css. */
  .card {
    margin: 0 var(--space-3) var(--space-2);
    padding: var(--space-3) var(--space-4) var(--space-3);
  }

  header {
    justify-content: space-between;
    margin: 0 0 var(--space-2);
  }

  .plain-aside {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-1-5);
    margin-bottom: var(--space-1-5);
    color: var(--muted);
    font-size: var(--text-xs);
  }

  .aside {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1-5);
    color: var(--muted);
    font-size: var(--text-md);
  }
</style>
