<script lang="ts">
  // A small line with no axes: how a rate has moved, at a glance. Drawn only with two points or
  // more; with fewer it keeps its height so the tile doesn't jump when the line arrives.
  let {
    values,
    color = "var(--accent)",
    height = 28,
    label,
  }: {
    /** Oldest first. Nulls are gaps and are skipped. */
    values: (number | null)[];
    color?: string;
    height?: number;
    label?: string;
  } = $props();

  const W = 150;
  const PAD = 3;

  const path = $derived.by(() => {
    const points = values.filter((v): v is number => v != null && Number.isFinite(v));
    if (points.length < 2) return "";
    const low = Math.min(...points);
    const high = Math.max(...points);
    const span = high - low || 1;
    const step = W / (points.length - 1);
    return points
      .map((v, i) => `${i ? "L" : "M"}${(i * step).toFixed(1)} ${(height - PAD - ((v - low) / span) * (height - 2 * PAD)).toFixed(1)}`)
      .join(" ");
  });
</script>

<svg width="100%" {height} viewBox="0 0 {W} {height}" preserveAspectRatio="none" role={label ? "img" : undefined} aria-label={label} aria-hidden={label ? undefined : "true"}>
  {#if path}
    <path d={path} fill="none" stroke={color} stroke-width="2.2" stroke-linejoin="round" stroke-linecap="round" vector-effect="non-scaling-stroke" />
  {/if}
</svg>

<style>
  svg {
    display: block;
  }
</style>
