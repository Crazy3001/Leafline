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
    font-size: var(--text-sm, 13px);
    line-height: 1.5;
  }

  main {
    max-width: 920px;
    margin: 0 auto;
    padding: 24px 16px 64px;
  }

  .test :global(h1) {
    margin: 0 0 6px;
    font-family: var(--font-display);
    font-size: 22px;
  }

  .test :global(h2) {
    margin: 0 0 4px;
    font-family: var(--font-display);
    font-size: 16px;
  }

  .test :global(section) {
    margin: 18px 0 0;
    padding: 16px;
    border: var(--line-w) solid var(--border);
    border-radius: var(--radius, 10px);
    background: var(--surface);
  }

  .test :global(.lead) {
    margin: 0 0 10px;
    color: var(--muted);
  }

  .test :global(.controls) {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    margin: 10px 0;
  }

  .test :global(.button) {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 5px 12px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm, 6px);
    background: transparent;
    color: var(--text);
    cursor: pointer;
  }

  .test :global(.button.primary) {
    border-color: var(--accent);
    background: var(--accent-soft);
  }

  .test :global(.button:hover:not(:disabled)) {
    border-color: var(--accent);
  }

  .test :global(.button:disabled) {
    opacity: 0.5;
    cursor: default;
  }

  .test :global(select) {
    padding: 4px 6px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm, 6px);
    background: var(--surface-raised);
    color: var(--text);
  }

  .test :global(ol) {
    margin: 6px 0;
    padding-left: 20px;
  }

  .test :global(li) {
    margin: 3px 0;
  }

  .test :global(.status) {
    margin: 6px 0;
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
