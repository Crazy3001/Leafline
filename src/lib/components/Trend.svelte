<script lang="ts">
  // A small line of the recent rate over time, so a change of pace shows at a glance.
  let { points }: { points: number[] } = $props();

  const WIDTH = 120;
  const HEIGHT = 22;

  const line = $derived.by(() => {
    if (points.length < 2) return "";
    const top = Math.max(...points);
    const bottom = Math.min(...points);
    const range = top - bottom || 1;
    return points
      .map((value, i) => {
        const x = (i / (points.length - 1)) * WIDTH;
        const y = HEIGHT - 2 - ((value - bottom) / range) * (HEIGHT - 4);
        return `${x.toFixed(1)},${y.toFixed(1)}`;
      })
      .join(" ");
  });
</script>

{#if line}
  <svg viewBox="0 0 {WIDTH} {HEIGHT}" preserveAspectRatio="none" aria-hidden="true">
    <polyline points={line} />
  </svg>
{/if}

<style>
  svg {
    display: block;
    width: 100%;
    height: 22px;
  }

  polyline {
    fill: none;
    stroke: var(--exp);
    stroke-width: 1.25;
    vector-effect: non-scaling-stroke;
  }
</style>
