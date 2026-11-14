<script lang="ts">
  import type { ChartPoint } from "./types";
  // A rate over a session: one series, an area wash under a 2 px line, map stretches shaded
  // behind it, level-ups and deaths marked on it, and a crosshair with the value on hover.
  import { formatDuration } from "$lib/format";

  let {
    points,
    xMax,
    color = "var(--exp)",
    format,
    bands = [],
    markers = [],
    detail,
    label,
    height = 140,
  }: {
    points: ChartPoint[];
    xMax: number;
    color?: string;
    /** How a value reads, on the axis and in the tooltip. */
    format: (value: number) => string;
    bands?: { start: number; end: number; label: string }[];
    markers?: { x: number; kind: "levelUp" | "death"; label: string }[];
    /** A second tooltip line for the point at an index. */
    detail?: (index: number) => string | null;
    label: string;
    height?: number;
  } = $props();

  let width = $state(300);
  let hover = $state<number | null>(null);

  const LEFT = 36;
  const RIGHT = 6;
  const TOP = 16;
  const BOTTOM = 18;

  const plotW = $derived(Math.max(10, width - LEFT - RIGHT));
  const plotH = $derived(height - TOP - BOTTOM);
  const valid = $derived(points.filter((p): p is { x: number; y: number } => p.y != null));
  const top = $derived(niceMax(Math.max(1, ...valid.map((p) => p.y)) * 1.08));
  const span = $derived(Math.max(1, xMax));
  const px = (x: number) => LEFT + (x / span) * plotW;
  const py = (y: number) => TOP + plotH - (y / top) * plotH;

  const line = $derived(valid.map((p) => `${px(p.x).toFixed(1)},${py(p.y).toFixed(1)}`).join(" "));
  const area = $derived(
    valid.length > 1
      ? `M${px(valid[0].x).toFixed(1)},${TOP + plotH} L${valid.map((p) => `${px(p.x).toFixed(1)},${py(p.y).toFixed(1)}`).join(" L")} L${px(valid[valid.length - 1].x).toFixed(1)},${TOP + plotH} Z`
      : "",
  );
  const yTicks = $derived([1, 2, 3].map((i) => (top * i) / 3));
  const xTicks = $derived.by(() => {
    const minutes = span / 60;
    const step = [5, 10, 15, 30, 60, 120, 240].find((s) => minutes / s <= 4) ?? 480;
    const ticks: number[] = [];
    for (let m = 0; m <= minutes; m += step) ticks.push(m * 60);
    return ticks;
  });

  /** The value nearest a moment, for markers that sit on the line. */
  function yAt(x: number) {
    let best: { x: number; y: number } | null = null;
    for (const p of valid) if (!best || Math.abs(p.x - x) < Math.abs(best.x - x)) best = p;
    return best?.y ?? 0;
  }

  function niceMax(value: number) {
    const magnitude = 10 ** Math.floor(Math.log10(value));
    const steps = [1, 1.5, 2, 3, 4, 5, 6, 8, 10];
    const step = steps.find((s) => s * magnitude >= value) ?? 10;
    return step * magnitude;
  }

  function move(event: PointerEvent) {
    const rect = (event.currentTarget as SVGElement).getBoundingClientRect();
    const x = ((event.clientX - rect.left - LEFT) / plotW) * span;
    let best = -1;
    let distance = Infinity;
    points.forEach((p, i) => {
      if (p.y == null) return;
      const d = Math.abs(p.x - x);
      if (d < distance) {
        distance = d;
        best = i;
      }
    });
    hover = best < 0 ? null : best;
  }

  const hovered = $derived(hover == null ? null : points[hover]);
  const tipLeft = $derived(hovered ? Math.min(Math.max(0, px(hovered.x) + 8), width - 150) : 0);

  /** A band's name cut to its width, so neighbouring names never run into each other. */
  function fitLabel(text: string, room: number) {
    const fits = Math.floor(room / 6);
    return text.length <= fits ? text : `${text.slice(0, Math.max(fits - 1, 1)).trimEnd()}…`;
  }
</script>

<div class="chart" bind:clientWidth={width} style:height="{height}px">
  <svg {width} {height} role="img" aria-label={label} onpointermove={move} onpointerleave={() => (hover = null)}>
    {#each bands as band, i (i)}
      {@const x = px(band.start)}
      {@const w = px(band.end) - x}
      {#if i % 2 === 1}<rect {x} y={TOP} width={w} height={plotH} class="band" />{/if}
      {#if i > 0}<line x1={x} x2={x} y1={TOP} y2={TOP + plotH} class="divider" />{/if}
      {#if w > 56}<text x={x + 3} y={TOP - 5} class="band-label">{fitLabel(band.label, w - 8)}</text>{/if}
    {/each}
    <line x1={LEFT} x2={LEFT + plotW} y1={TOP + plotH} y2={TOP + plotH} class="axis" />
    {#each yTicks as tick (tick)}
      <line x1={LEFT} x2={LEFT + plotW} y1={py(tick)} y2={py(tick)} class="grid" />
      <text x={LEFT - 6} y={py(tick) + 3.5} text-anchor="end" class="tick">{format(tick)}</text>
    {/each}
    <text x={LEFT - 6} y={TOP + plotH + 3.5} text-anchor="end" class="tick">0</text>
    {#each xTicks as tick (tick)}
      <text x={px(tick)} y={height - 4} text-anchor={tick === 0 ? "start" : "middle"} class="tick">
        {tick === 0 ? "0" : formatDuration(tick)}
      </text>
    {/each}
    {#if area}<path d={area} fill={color} fill-opacity="0.1" />{/if}
    {#if line}<polyline points={line} fill="none" stroke={color} stroke-width="2" stroke-linejoin="round" stroke-linecap="round" />{/if}
    {#each markers as marker, i (i)}
      <circle cx={px(marker.x)} cy={py(yAt(marker.x))} r="4" class="marker {marker.kind}"><title>{marker.label}</title></circle>
    {/each}
    {#if hovered && hovered.y != null}
      <line x1={px(hovered.x)} x2={px(hovered.x)} y1={TOP} y2={TOP + plotH} class="cross" />
      <circle cx={px(hovered.x)} cy={py(hovered.y)} r="4" fill={color} class="dot" />
    {/if}
  </svg>
  {#if hovered && hovered.y != null && hover != null}
    {@const extra = detail?.(hover)}
    <div class="tip" style:left="{tipLeft}px">
      <span class="tip-value tabular">{format(hovered.y)}</span>
      <span class="tip-detail">{formatDuration(hovered.x)} in{extra ? ` · ${extra}` : ""}</span>
    </div>
  {/if}
</div>

<style>
  .chart {
    position: relative;
    width: 100%;
  }

  svg {
    display: block;
    overflow: visible;
    touch-action: none;
  }

  .band {
    fill: rgba(236, 230, 216, 0.035);
  }

  .divider {
    stroke: rgba(236, 230, 216, 0.3);
    stroke-width: 1;
  }

  .band-label {
    font-size: var(--text-xs);
    fill: var(--muted);
  }

  .axis {
    stroke: var(--border-strong);
    stroke-width: 1;
  }

  .grid {
    stroke: rgba(236, 230, 216, 0.07);
    stroke-width: 1;
  }

  .tick {
    font-size: 10px;
    fill: var(--faint);
    font-variant-numeric: tabular-nums;
  }

  .marker {
    stroke: var(--bg);
    stroke-width: 2;
  }

  .levelUp {
    fill: var(--good);
  }

  .death {
    fill: var(--bad);
  }

  .cross {
    stroke: rgba(236, 230, 216, 0.45);
    stroke-width: 1;
  }

  .dot {
    stroke: var(--bg);
    stroke-width: 2;
  }

  .tip {
    position: absolute;
    top: 22px;
    display: flex;
    flex-direction: column;
    max-width: 150px;
    padding: 5px 8px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface-3);
    font-size: var(--text-xs);
    pointer-events: none;
  }

  .tip-value {
    font-weight: 600;
    color: var(--text);
  }

  .tip-detail {
    color: var(--muted);
  }
</style>
