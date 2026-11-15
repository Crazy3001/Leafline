<script lang="ts">
  // EXP towards the next level, drawn like the game's own bar: a bright fill with the points and
  // percent written across it, so it reads the same as the HUD at a glance.
  import { formatNumber } from "$lib/format";

  let {
    percent,
    points = null,
    needed = null,
  }: {
    /** 0 to 100. */
    percent: number;
    /** EXP into the level, when the HUD shows points. */
    points?: number | null;
    /** The EXP the level needs, when measured. */
    needed?: number | null;
  } = $props();

  const share = $derived(Math.max(0, Math.min(100, Number.isFinite(percent) ? percent : 0)));
  const text = $derived(
    points != null
      ? `${formatNumber(points)}${needed ? ` / ${formatNumber(needed)}` : ""} [${share.toFixed(2)}%]`
      : `${share.toFixed(2)}%`,
  );
</script>

<div class="bar" role="meter" aria-label="EXP towards the next level" aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.round(share)}>
  <div class="fill" style:width="{share}%"></div>
  <span class="text tabular"><b>EXP</b> {text}</span>
</div>

<style>
  .bar {
    position: relative;
    height: 16px;
    overflow: hidden;
    border: 1px solid rgba(0, 0, 0, 0.55);
    border-radius: 4px;
    background: #2b2f2c;
    box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.5);
  }

  .fill {
    height: 100%;
    background: var(--exp-bar, #b5e61d);
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.45),
      inset 0 -2px 0 rgba(0, 0, 0, 0.18);
    transition: width 0.4s ease;
  }

  .text {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    color: #fff;
    font-size: 10.5px;
    font-weight: 600;
    letter-spacing: 0.02em;
    text-shadow:
      0 0 2px #000,
      0 1px 1px #000;
    white-space: nowrap;
  }

  .text b {
    margin-right: 3px;
    color: #ffe680;
    font-style: italic;
    font-weight: 800;
  }
</style>
