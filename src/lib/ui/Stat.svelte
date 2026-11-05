<script lang="ts">
  // One number with its label: the tile used for rates and totals.
  let {
    label,
    value,
    sub,
    size = "md",
    title,
  }: {
    label: string;
    value: string;
    sub?: string | null;
    size?: "sm" | "md" | "lg";
    title?: string;
  } = $props();
</script>

<div class="stat {size}" {title}>
  <span class="label">{label}</span>
  <span class="value num">{value}</span>
  {#if sub}<span class="sub tabular">{sub}</span>{/if}
</div>

<style>
  .stat {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    padding: var(--space-2);
    background: var(--surface-2);
  }

  .lg {
    padding: 10px var(--space-3);
    border: 1px solid var(--border);
    background: var(--surface);
  }

  .label {
    font-size: var(--text-xs);
    color: var(--muted);
  }

  .value {
    overflow: hidden;
    font-size: var(--text-lg);
    font-weight: 600;
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
</style>
