<script lang="ts">
  // One number with its label: the tile used for rates and totals. A help line, when given,
  // sits behind a small info mark beside the label.
  import { Info } from "@lucide/svelte";

  import { tip } from "$lib/tip.svelte";

  let {
    label,
    value,
    sub,
    size = "md",
    title,
    faint = false,
    help = null,
  }: {
    label: string;
    value: string;
    sub?: string | null;
    size?: "sm" | "md" | "lg";
    title?: string;
    /** The value is a rough one: drawn greyed out. */
    faint?: boolean;
    /** How the figure is worked out, shown on hover. */
    help?: string | null;
  } = $props();
</script>

<div class="stat {size}" class:faint {title}>
  <span class="label">{label}{#if help}<span class="help" use:tip={help}><Info size={11} aria-label="How it's counted" /></span>{/if}</span>
  <span class="value num">{value}</span>
  {#if sub}<span class="sub tabular">{sub}</span>{/if}
</div>

<style>
  .stat {
    display: flex;
    flex-direction: column;
    gap: var(--space-0-5);
    min-width: 0;
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius);
    background: var(--surface-2);
  }

  .lg {
    padding: var(--space-2) var(--space-3);
    border: var(--line-w) solid var(--line);
    border-radius: var(--radius);
    background: var(--surface-2);
  }

  .label {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    font-size: var(--text-xs);
    color: var(--muted);
  }

  .help {
    display: inline-flex;
    color: var(--faint);
    cursor: help;
  }

  .value {
    overflow: hidden;
    font-size: var(--text-lg);
    font-weight: var(--weight-strong);
    line-height: 1.2;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .sm .value {
    font-size: var(--text-md);
  }

  .lg .value {
    font-size: var(--text-2xl);
  }

  .sub {
    font-size: var(--text-xs);
    color: var(--muted);
  }

  .faint .value {
    color: var(--faint);
  }
</style>
