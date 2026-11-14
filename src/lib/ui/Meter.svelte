<script lang="ts">
  // A flat fill bar: EXP towards a level, quest counters, how one rate compares with another.
  let {
    value,
    color = "var(--exp)",
    height = 6,
    label,
  }: { value: number; color?: string; height?: number; label?: string } = $props();

  const share = $derived(Math.max(0, Math.min(1, Number.isFinite(value) ? value : 0)));
</script>

<div
  class="meter"
  style:height="{height}px"
  role={label ? "meter" : undefined}
  aria-label={label}
  aria-valuemin={label ? 0 : undefined}
  aria-valuemax={label ? 100 : undefined}
  aria-valuenow={label ? Math.round(share * 100) : undefined}
>
  <div class="fill" style:width="{share * 100}%" style:background={color}></div>
</div>

<style>
  .meter {
    width: 100%;
    overflow: hidden;
    border-radius: var(--radius-pill);
    background: rgba(255, 255, 255, 0.06);
  }

  .fill {
    height: 100%;
    border-radius: inherit;
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.25);
  }
</style>
