<script lang="ts">
  // The one button style. Primary is for the single main action on a view; the rest are
  // secondary, ghost (no frame) or danger.
  import type { Snippet } from "svelte";

  import { tip } from "$lib/tip.svelte";

  let {
    variant = "secondary",
    size = "md",
    title,
    disabled = false,
    pressed,
    onclick,
    children,
  }: {
    variant?: "primary" | "secondary" | "ghost" | "danger";
    size?: "md" | "sm";
    title?: string;
    disabled?: boolean;
    pressed?: boolean;
    onclick?: (event: MouseEvent) => void;
    children: Snippet;
  } = $props();
</script>

<button class="btn {variant} {size}" use:tip={title} {disabled} aria-pressed={pressed} {onclick}>{@render children()}</button>

<style>
  .btn {
    display: inline-flex;
    flex: none;
    align-items: center;
    justify-content: center;
    gap: 6px;
    height: var(--control-h);
    padding: 0 12px;
    border: var(--line-w) solid var(--control-line);
    border-radius: var(--radius);
    background: var(--surface-2);
    box-shadow: var(--control-shadow);
    color: var(--text);
    font-size: var(--text-md);
    font-weight: var(--weight-body);
    white-space: nowrap;
    text-decoration: none;
    cursor: pointer;
    transition:
      background 0.12s ease,
      border-color 0.12s ease,
      transform 0.08s ease;
  }

  .btn:hover:not(:disabled) {
    background: var(--surface-3);
  }

  .btn:active:not(:disabled) {
    transform: translateY(1px);
  }

  .btn:focus-visible {
    outline: 2px solid var(--accent-text);
    outline-offset: 1px;
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .sm {
    height: var(--control-h-sm);
    border-radius: var(--radius-sm);
    padding: 0 8px;
    font-size: var(--text-xs);
  }

  :global(:root:not([data-theme^="sprout"])) .primary {
    border-color: transparent;
  }

  .primary {
    background: var(--accent);
    color: var(--accent-ink);
    font-weight: var(--weight-strong);
  }

  .primary:hover:not(:disabled) {
    background: var(--accent-strong);
  }

  .ghost {
    border-color: transparent;
    background: transparent;
    box-shadow: none;
    color: var(--muted);
  }

  .ghost:hover:not(:disabled) {
    background: var(--hover);
    color: var(--text);
  }

  .danger {
    border-color: color-mix(in srgb, var(--bad) 45%, transparent);
    background: transparent;
    box-shadow: none;
    color: var(--bad);
  }

  .danger:hover:not(:disabled) {
    background: var(--bad-soft);
  }

  .btn[aria-pressed="true"] {
    background: var(--surface-3);
  }
</style>
