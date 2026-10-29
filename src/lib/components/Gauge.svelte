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
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.06em;
    color: var(--muted);
  }

  .value {
    font-size: 12px;
    color: var(--text);
  }

  .unread .value {
    color: var(--faint);
  }

  .track {
    height: 6px;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.06);
    overflow: hidden;
  }

  .fill {
    height: 100%;
    border-radius: inherit;
    background: linear-gradient(90deg, color-mix(in srgb, var(--color) 70%, black), var(--color));
    box-shadow: 0 0 10px color-mix(in srgb, var(--color) 45%, transparent);
    transition: width 0.4s ease;
  }
</style>
