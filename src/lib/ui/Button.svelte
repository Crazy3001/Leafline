<script lang="ts">
  // The one button style. Primary is for the single main action on a view; the rest are
  // secondary, ghost (no frame) or danger.
  import type { Snippet } from "svelte";

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

<button class="btn {variant} {size}" {title} {disabled} aria-pressed={pressed} {onclick}>{@render children()}</button>

<style>
  .btn {
    display: inline-flex;
    flex: none;
    align-items: center;
    justify-content: center;
    gap: 6px;
    height: var(--control-h);
    padding: 0 10px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    color: var(--text);
    font-size: var(--text-sm);
    white-space: nowrap;
    text-decoration: none;
    cursor: pointer;
  }

  .btn:hover:not(:disabled) {
    background: var(--surface-3);
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
    border-color: var(--accent);
    background: var(--accent);
    color: var(--bg);
    font-weight: 600;
  }

  .primary:hover:not(:disabled) {
    border-color: var(--accent-strong);
    background: var(--accent-strong);
  }

  .ghost {
    border-color: transparent;
    background: transparent;
    color: var(--muted);
  }

  .ghost:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.07);
    color: var(--text);
  }

  .danger {
    border-color: color-mix(in srgb, var(--bad) 45%, transparent);
    background: transparent;
    color: var(--bad);
  }

  .danger:hover:not(:disabled) {
    background: var(--bad-soft);
  }

  .btn[aria-pressed="true"] {
    background: var(--surface-3);
  }
</style>
