<script lang="ts">
  // The frame of the capture test's pages: a plain document page in Leafline's colours.
  import type { Snippet } from "svelte";

  let { title, children }: { title: string; children: Snippet } = $props();
</script>

<svelte:head><title>{title}</title></svelte:head>

<div class="test">
  <main>
    {@render children()}
  </main>
</div>

<style>
  .test {
    min-height: 100vh;
    background: var(--bg);
    color: var(--text);
    font-family: var(--font-sans);
    font-size: var(--text-md);
    line-height: 1.5;
  }

  main {
    max-width: 920px;
    margin: 0 auto;
    padding: var(--space-6) var(--space-4) 64px;
  }

  .test :global(h1) {
    margin: 0 0 var(--space-1-5);
    font-family: var(--font-display);
    font-size: 22px;
  }

  .test :global(h2) {
    margin: 0 0 var(--space-1);
    font-family: var(--font-display);
    font-size: 16px;
  }

  .test :global(section) {
    margin: var(--space-4) 0 0;
    padding: var(--space-4);
    border: var(--line-w) solid var(--border);
    border-radius: var(--radius, 10px);
    background: var(--surface);
  }

  .test :global(.lead) {
    margin: 0 0 var(--space-2);
    color: var(--muted);
  }

  .test :global(.controls) {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
    margin: var(--space-2) 0;
  }

  .test :global(select) {
    padding: var(--space-1) var(--space-1-5);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm, 6px);
    background: var(--surface-raised);
    color: var(--text);
  }

  .test :global(ol) {
    margin: var(--space-1-5) 0;
    padding-left: var(--space-5);
  }

  .test :global(li) {
    margin: var(--space-0-5) 0;
  }

  .test :global(.status) {
    margin: var(--space-1-5) 0;
  }

  .test :global(.good) {
    color: var(--good);
  }

  .test :global(.warn) {
    color: var(--warn);
  }

  .test :global(.bad) {
    color: var(--bad);
  }

  .test :global(.muted) {
    color: var(--muted);
  }

  .test :global(code) {
    font-family: var(--font-mono);
    font-size: 12px;
  }
</style>
