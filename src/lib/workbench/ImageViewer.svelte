<script lang="ts">
  import type { Rect } from "$lib/types";
  import type { Region } from "./regions";

  let {
    src,
    width,
    height,
    regions,
    highlight = null,
  }: {
    src: string;
    width: number;
    height: number;
    regions: Region[];
    highlight?: Rect | null;
  } = $props();

  const PADDING = 20;
  let viewportWidth = $state(0);
  let viewportHeight = $state(0);

  // Fit the screenshot inside the viewport without changing its aspect ratio.
  const fit = $derived(
    Math.max(0, Math.min((viewportWidth - 2 * PADDING) / width, (viewportHeight - 2 * PADDING) / height)),
  );
  // Keep outlines about 1.5 screen pixels wide at any zoom.
  const stroke = $derived(1.5 / (fit || 1));
</script>

<div class="viewport" bind:clientWidth={viewportWidth} bind:clientHeight={viewportHeight}>
  <div class="stage" style:width="{width * fit}px" style:height="{height * fit}px">
    <img {src} alt="Screenshot being analysed" draggable="false" />
    <svg viewBox="0 0 {width} {height}" preserveAspectRatio="none">
      {#each regions as region}
        <rect
          class={region.kind}
          x={region.rect.x}
          y={region.rect.y}
          width={region.rect.w}
          height={region.rect.h}
          stroke-width={stroke}
          stroke-dasharray={region.kind === "text" ? `${stroke * 3} ${stroke * 2}` : undefined}
          rx={stroke * 2}
        >
          <title>{region.label}</title>
        </rect>
      {/each}
      {#if highlight}
        <rect
          class="highlight"
          x={highlight.x - stroke * 2}
          y={highlight.y - stroke * 2}
          width={highlight.w + stroke * 4}
          height={highlight.h + stroke * 4}
          stroke-width={stroke * 1.5}
          rx={stroke * 2}
        />
      {/if}
    </svg>
  </div>
</div>

<style>
  .viewport {
    display: grid;
    place-items: center;
    width: 100%;
    height: 100%;
    overflow: hidden;
  }

  .stage {
    position: relative;
    border-radius: var(--radius-sm);
    overflow: hidden;
    box-shadow: var(--shadow);
  }

  img,
  svg {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }

  img {
    object-fit: fill;
    user-select: none;
  }

  rect {
    fill: transparent;
    pointer-events: all;
  }

  rect:hover {
    fill: rgba(255, 255, 255, 0.08);
  }

  .hud {
    stroke: var(--exp);
  }

  .tooltip {
    stroke: var(--accent);
  }

  .quest {
    stroke: var(--mp);
  }

  .mesos {
    stroke: var(--good);
  }

  .map {
    stroke: var(--map);
  }

  .text {
    stroke: rgba(255, 255, 255, 0.7);
  }

  .highlight {
    fill: rgba(243, 177, 63, 0.18);
    stroke: var(--accent-strong);
    pointer-events: none;
  }
</style>
