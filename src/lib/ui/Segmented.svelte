<script lang="ts" generics="T extends string | number">
  // A row of choices where exactly one is picked: the one control for modes and sorts.
  import { tip } from "$lib/tip.svelte";
  let {
    options,
    value,
    label,
    size = "md",
    stretch = false,
    onchange,
  }: {
    options: { id: T; label: string; title?: string }[];
    value: T;
    /** What the choice is about, for screen readers. */
    label: string;
    size?: "md" | "sm";
    /** Fill the width, each choice the same share. */
    stretch?: boolean;
    onchange: (value: T) => void;
  } = $props();
</script>

<div class="seg {size}" class:stretch role="group" aria-label={label}>
  {#each options as option (option.id)}
    <button aria-pressed={option.id === value} use:tip={option.title} onclick={() => onchange(option.id)}>
      {option.label}
    </button>
  {/each}
</div>

<style>
  /* Chips: each choice its own rounded button, the picked one filled with the accent. */
  .seg {
    display: inline-flex;
    flex: none;
    flex-wrap: wrap;
    gap: 6px;
  }

  .stretch {
    display: flex;
    flex: 1;
    flex-wrap: nowrap;
    gap: 4px;
  }

  .stretch button {
    flex: 1;
    min-width: 0;
    padding: 0 4px;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  button {
    height: calc(var(--control-h) - 2px);
    padding: 0 12px;
    border: var(--line-w) solid var(--control-line);
    border-radius: var(--radius-pill);
    background: var(--surface);
    color: var(--muted);
    font-size: var(--text-md);
    font-weight: var(--weight-body);
    white-space: nowrap;
    cursor: pointer;
    transition:
      background 0.12s ease,
      color 0.12s ease;
  }

  .sm button {
    height: var(--control-h-sm);
    padding: 0 9px;
    font-size: var(--text-xs);
  }

  button:hover:not([aria-pressed="true"]) {
    background: var(--surface-2);
    color: var(--text);
  }

  button:focus-visible {
    outline: 2px solid var(--accent-text);
    outline-offset: 1px;
  }

  button[aria-pressed="true"] {
    border-color: var(--control-line);
    background: var(--accent);
    color: var(--accent-ink);
    font-weight: var(--weight-strong);
  }

  :global(:root:not([data-theme^="sprout"])) button[aria-pressed="true"] {
    border-color: transparent;
  }
</style>
