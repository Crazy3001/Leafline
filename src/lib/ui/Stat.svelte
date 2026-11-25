<script lang="ts">
  // One number with its label: the tile used for rates and totals.
  let {
    label,
    value,
    sub,
    size = "md",
    title,
    faint = false,
  }: {
    label: string;
    value: string;
    sub?: string | null;
    size?: "sm" | "md" | "lg";
    title?: string;
    /** The value is a rough one: drawn greyed out. */
    faint?: boolean;
  } = $props();
</script>

<div class="stat {size}" class:faint {title}>
  <span class="label">{label}</span>
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
    font-size: var(--text-xs);
    color: var(--muted);
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
