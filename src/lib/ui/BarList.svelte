<script lang="ts">
  import type { Bar } from "./types";
  // Horizontal bars from one baseline, value at each tip: maps by rate, levels by time. Rows can
  // be picked.
  let {
    rows,
    color = "var(--exp)",
    selected = null,
    onselect,
    label,
  }: {
    rows: Bar[];
    color?: string;
    selected?: string | null;
    onselect?: (id: string) => void;
    label: string;
  } = $props();

  const top = $derived(Math.max(1, ...rows.map((r) => r.value)));
</script>

<div class="bars" aria-label={label} role="group">
  {#each rows as row (row.id)}
    {#snippet body()}
      <span class="names">
        <span class="name">{row.label}</span>
        {#if row.sub}<span class="sub tabular">{row.sub}</span>{/if}
      </span>
      <span class="track">
        <span class="bar" class:faded={row.faded} style:width="{(row.value / top) * 82}%" style:background={color}></span>
        <span class="value tabular">{row.display}</span>
      </span>
    {/snippet}
    {#if onselect}
      <button class="row pickable" aria-pressed={row.id === selected} onclick={() => onselect(row.id)}>{@render body()}</button>
    {:else}
      <div class="row">{@render body()}</div>
    {/if}
  {/each}
</div>

<style>
  .bars {
    display: flex;
    flex-direction: column;
  }

  .row {
    display: grid;
    grid-template-columns: minmax(90px, 160px) minmax(0, 1fr);
    gap: var(--space-3);
    align-items: center;
    width: 100%;
    padding: 6px var(--space-2);
    border: 0;
    background: transparent;
    text-align: left;
    cursor: default;
  }

  .pickable {
    cursor: pointer;
  }

  .pickable:hover {
    background: var(--surface-2);
  }

  .row[aria-pressed="true"] {
    background: var(--surface-3);
  }

  .names {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .name {
    overflow: hidden;
    font-size: var(--text-sm);
    font-weight: 600;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .sub {
    font-size: var(--text-xs);
    color: var(--faint);
  }

  .track {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
  }

  .bar {
    height: 16px;
    min-width: 2px;
    border-radius: 0 4px 4px 0;
  }

  .faded {
    opacity: 0.4;
  }

  .value {
    font-size: var(--text-sm);
    font-weight: 600;
    white-space: nowrap;
  }
</style>
