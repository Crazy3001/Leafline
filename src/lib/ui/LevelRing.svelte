<script lang="ts">
  // The level inside a ring that fills with the level's EXP: the header's gauge. With no level
  // read yet it shows whatever is put inside it (the leaf mark) in an empty ring.
  import type { Snippet } from "svelte";

  let {
    level,
    percent,
    size = 48,
    children,
  }: {
    level: number | null;
    /** EXP through the level, 0 to 100. */
    percent: number | null;
    size?: number;
    /** Shown in place of the level when there is none. */
    children?: Snippet;
  } = $props();

  const R = 20.5;
  const LENGTH = 2 * Math.PI * R;
  const filled = $derived(percent == null ? 0 : (Math.min(100, Math.max(0, percent)) / 100) * LENGTH);
  const label = $derived(
    level == null ? "No level read yet" : percent == null ? `Level ${level}` : `Level ${level}, ${percent.toFixed(1)}% EXP`,
  );
</script>

<div class="ring" style:width="{size}px" style:height="{size}px">
  <svg width={size} height={size} viewBox="0 0 48 48" role="img" aria-label={label}>
    <circle class="track" cx="24" cy="24" r={R} />
    {#if filled > 0}
      <circle class="fill" cx="24" cy="24" r={R} stroke-dasharray="{filled} {LENGTH}" transform="rotate(-90 24 24)" />
    {/if}
  </svg>
  <div class="inside" aria-hidden="true">
    {#if level != null}
      <span class="lv">LV</span>
      <span class="n" class:long={level >= 100}>{level}</span>
    {:else}
      {@render children?.()}
    {/if}
  </div>
</div>

<style>
  .ring {
    position: relative;
    flex: none;
  }

  svg {
    display: block;
  }

  circle {
    fill: none;
    stroke-width: 4.5;
  }

  .track {
    stroke: var(--header-track);
  }

  .fill {
    stroke: var(--exp);
    stroke-linecap: round;
    transition: stroke-dasharray 0.4s var(--ease);
  }

  .inside {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    font-family: var(--font-display);
    line-height: 1;
    color: var(--header-text);
  }

  .lv {
    font-size: 8.5px;
    font-weight: var(--weight-strong);
    color: var(--header-faint);
  }

  .n {
    font-size: 17px;
    font-weight: var(--weight-strong);
    font-variant-numeric: tabular-nums;
  }

  .n.long {
    font-size: 14px;
  }
</style>
