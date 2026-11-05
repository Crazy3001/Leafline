<script lang="ts">
  import type { Column } from "./types";
  // Columns over time, one per day: the best one labelled, any other on hover, with a line
  // under the chart saying what the hovered column holds.
  let {
    columns,
    color = "var(--exp)",
    format,
    label,
    height = 180,
    hint = "Hover a day for what it holds.",
  }: {
    columns: Column[];
    color?: string;
    format: (value: number) => string;
    label: string;
    height?: number;
    hint?: string;
  } = $props();

  let hover = $state<number | null>(null);

  const top = $derived(niceMax(Math.max(1, ...columns.map((c) => c.value)) * 1.05));
  const best = $derived(columns.reduce((b, c, i) => (c.value > (columns[b]?.value ?? -1) ? i : b), 0));
  const ticks = $derived([3, 2, 1].map((i) => (top * i) / 3));

  function niceMax(value: number) {
    const magnitude = 10 ** Math.floor(Math.log10(value));
    const steps = [1, 1.5, 2, 3, 4, 5, 6, 8, 10];
    return (steps.find((s) => s * magnitude >= value) ?? 10) * magnitude;
  }
</script>

<div class="columns" role="img" aria-label={label}>
  <div class="ticks tabular" style:height="{height}px">
    {#each ticks as tick (tick)}<span>{format(tick)}</span>{/each}
    <span>0</span>
  </div>
  <div class="plot" style:height="{height}px" onpointerleave={() => (hover = null)} role="presentation">
    {#each ticks as tick (tick)}<span class="grid" style:bottom="{(tick / top) * 100}%"></span>{/each}
    <div class="cols">
      {#each columns as column, i (i)}
        <div class="col" onpointerenter={() => (hover = i)} role="presentation">
          <span class="cap tabular" class:shown={hover === i || (hover == null && i === best)}>{column.display}</span>
          <span class="bar" style:height="{(column.value / top) * 100}%" style:background={color}></span>
        </div>
      {/each}
    </div>
  </div>
  <span></span>
  <div class="labels">
    {#each columns as column, i (i)}<span class="tabular">{column.label}</span>{/each}
  </div>
</div>
<p class="detail tabular">{hover == null ? hint : columns[hover].detail}</p>

<style>
  .columns {
    display: grid;
    grid-template-columns: 44px minmax(0, 1fr);
    gap: 4px var(--space-2);
  }

  .ticks {
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    align-items: flex-end;
    font-size: 10px;
    line-height: 1;
    color: var(--faint);
  }

  .plot {
    position: relative;
    border-bottom: 1px solid var(--border-strong);
  }

  .grid {
    position: absolute;
    left: 0;
    right: 0;
    height: 1px;
    background: rgba(236, 230, 216, 0.07);
  }

  .cols {
    position: absolute;
    inset: 0;
    display: flex;
    gap: 2px;
  }

  .col {
    display: flex;
    flex: 1;
    flex-direction: column;
    justify-content: flex-end;
    align-items: center;
    min-width: 0;
  }

  .cap {
    margin-bottom: 4px;
    font-size: var(--text-xs);
    white-space: nowrap;
    visibility: hidden;
  }

  .cap.shown {
    visibility: visible;
  }

  .bar {
    width: min(24px, 70%);
    border-radius: 4px 4px 0 0;
  }

  .labels {
    display: flex;
    gap: 2px;
  }

  .labels span {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    font-size: 10px;
    color: var(--faint);
    text-align: center;
    white-space: nowrap;
  }

  .detail {
    margin: 6px 0 0;
    min-height: 18px;
    font-size: var(--text-sm);
    color: var(--muted);
  }
</style>
