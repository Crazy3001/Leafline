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
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: linear-gradient(180deg, var(--surface-3), var(--surface-2));
    box-shadow: inset 0 1px 0 var(--highlight);
    color: var(--text);
    font-size: var(--text-sm);
    font-weight: 500;
    white-space: nowrap;
    text-decoration: none;
    cursor: pointer;
    transition:
      background 0.12s ease,
      border-color 0.12s ease,
      transform 0.08s ease;
  }

  .btn:hover:not(:disabled) {
    border-color: rgba(214, 222, 240, 0.24);
    background: var(--surface-3);
  }

  .btn:active:not(:disabled) {
    transform: translateY(1px);
  }

  .btn:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .sm {
    height: var(--control-h-sm);
    padding: 0 8px;
    font-size: var(--text-xs);
  }

  .primary {
    border-color: transparent;
    background: linear-gradient(180deg, var(--accent-strong), var(--accent));
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.25),
      0 4px 14px color-mix(in srgb, var(--accent) 25%, transparent);
    color: var(--accent-ink);
    font-weight: 600;
  }

  .primary:hover:not(:disabled) {
    border-color: transparent;
    background: linear-gradient(180deg, #ffcd80, var(--accent-strong));
  }

  .ghost {
    border-color: transparent;
    background: transparent;
    box-shadow: none;
    color: var(--muted);
  }

  .ghost:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.07);
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
