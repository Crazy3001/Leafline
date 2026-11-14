<script lang="ts">
  let {
    label,
    color,
    fraction,
    value,
  }: {
    label: string;
    color: string;
    /** 0–1, or null when unread. */
    fraction: number | null;
    value: string;
  } = $props();

  const width = $derived(fraction == null ? 0 : Math.min(1, Math.max(0, fraction)) * 100);
</script>

<div class="gauge" class:unread={fraction == null}>
  <div class="row">
    <span class="label">{label}</span>
    <span class="value tabular">{value}</span>
  </div>
  <div class="track">
    <div class="fill" style:width="{width}%" style:--color={color}></div>
  </div>
</div>

<style>
  .row {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    margin-bottom: 4px;
  }

  .label {
    font-size: var(--text-xs);
    font-weight: 700;
    color: var(--muted);
  }

  .value {
    font-family: var(--font-display);
    font-size: var(--text-md);
    color: var(--text);
  }

  .unread .value {
    color: var(--faint);
  }

  .track {
    height: 8px;
    border-radius: var(--radius-pill);
    background: rgba(255, 255, 255, 0.06);
    box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.35);
    overflow: hidden;
  }

  .fill {
    height: 100%;
    border-radius: inherit;
    background: linear-gradient(180deg, color-mix(in srgb, var(--color) 80%, white), var(--color));
    box-shadow: 0 0 10px color-mix(in srgb, var(--color) 35%, transparent);
    transition: width 0.4s var(--ease);
  }
</style>
