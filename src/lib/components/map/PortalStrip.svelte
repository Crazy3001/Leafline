<script lang="ts">
  // The map the player is on, as a strip under the route bar's next step, with the portal to take
  // lit up where it stands. Wide maps show whole; tall ones are cut to a band around the portal.
  // Teleports and NPCs are left off: the one question it answers is "which portal".
  import { backend } from "$lib/api";
  import { openPeek } from "$lib/peek.svelte";
  import { pictureUrl } from "$lib/pictures";
  import { tip } from "$lib/tip.svelte";
  import type { MapPicture, MapRef, ViewPoint } from "$lib/types";

  let {
    from,
    portal,
    to,
    height = 76,
  }: {
    /** The map the step starts on, whose picture is shown. */
    from: MapRef;
    /** The portal's own name, as the route step gives it. */
    portal: string | null;
    /** The map the step leads to, for when the portal's name isn't on the picture. */
    to: number;
    /** The tallest the strip grows, in pixels. */
    height?: number;
  } = $props();

  /** Below this the picture is too thin to make out, so a wide map is cut at the sides instead. */
  const MIN_HEIGHT = 44;

  let picture = $state<MapPicture | null>(null);
  let loaded = $state(false);
  let failed = $state(false);
  let width = $state(0);

  $effect(() => {
    const id = from.id;
    picture = null;
    loaded = false;
    failed = false;
    backend
      .mapPicture(id)
      .then((result) => {
        if (id === from.id) picture = result;
      })
      .catch(() => (picture = null));
  });

  const src = $derived(picture ? pictureUrl("mapview", from.id) : null);
  const portals = $derived(picture?.points.filter((p) => p.kind === "portal" && !p.teleport) ?? []);
  /** The portal to take: by its own name, or else any portal to the next map. */
  const targets = $derived.by(() => {
    const named = portal ? portals.filter((p) => p.portal === portal) : [];
    return named.length ? named : portals.filter((p) => p.id === to);
  });
  const focus = $derived<Pick<ViewPoint, "x" | "y">>(targets[0] ?? { x: 0.5, y: 0.5 });

  /** The picture's size on screen and where it sits in the strip, kept on the portal. */
  const frame = $derived.by(() => {
    if (!picture || !width) return null;
    const scale = Math.max(width / picture.width, MIN_HEIGHT / picture.height);
    const w = picture.width * scale;
    const h = picture.height * scale;
    const viewH = Math.min(h, height);
    const clamp = (v: number, max: number) => Math.min(Math.max(v, 0), Math.max(max, 0));
    return {
      w,
      h,
      viewH,
      left: -clamp(focus.x * w - width / 2, w - width),
      top: -clamp(focus.y * h - viewH / 2, h - viewH),
    };
  });
</script>

{#if src && !failed}
  <button
    class="strip"
    class:ready={loaded && frame}
    bind:clientWidth={width}
    style:height="{frame?.viewH ?? MIN_HEIGHT}px"
    use:tip={targets.length ? `Where the portal stands on ${from.name}` : `${from.name}: the portal isn't on the picture`}
    aria-label="{from.name}, with the portal to take marked"
    onclick={(e) => openPeek({ kind: "map", id: from.id, name: from.name }, e.currentTarget)}
  >
    {#if frame}
      <span class="layer" style:width="{frame.w}px" style:height="{frame.h}px" style:left="{frame.left}px" style:top="{frame.top}px">
        <img {src} alt="" draggable="false" onload={() => (loaded = true)} onerror={() => (failed = true)} />
        {#each portals as point, i (i)}
          {@const lit = targets.includes(point)}
          <span class="mark" class:lit style:left="{point.x * 100}%" style:top="{point.y * 100}%"></span>
        {/each}
      </span>
    {/if}
  </button>
{/if}

<style>
  .strip {
    position: relative;
    display: block;
    width: 100%;
    margin-top: 6px;
    padding: 0;
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg);
    cursor: pointer;
  }

  .strip:hover {
    border-color: color-mix(in srgb, var(--map) 45%, var(--border));
  }

  .strip:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  .layer {
    position: absolute;
    opacity: 0;
    transition: opacity 0.2s ease;
  }

  .ready .layer {
    opacity: 1;
  }

  img {
    display: block;
    width: 100%;
    height: 100%;
    filter: saturate(0.7) brightness(0.6);
    user-select: none;
  }

  /* Other portals stay small and quiet; the one to take is a bright ring that pulses. */
  .mark {
    position: absolute;
    width: 7px;
    height: 7px;
    border: 2px solid color-mix(in srgb, var(--marker-portal) 70%, transparent);
    border-radius: 50%;
    background: var(--bg);
    transform: translate(-50%, -50%);
  }

  .mark.lit {
    z-index: 1;
    width: 12px;
    height: 12px;
    border: 3px solid var(--accent-strong);
    background: radial-gradient(circle, var(--accent-strong) 0 3px, var(--bg) 3.5px);
    box-shadow:
      0 0 0 2px var(--bg),
      0 0 12px color-mix(in srgb, var(--accent) 70%, transparent);
  }

  .mark.lit::after {
    content: "";
    position: absolute;
    inset: -8px;
    border: 2px solid var(--accent);
    border-radius: 50%;
    animation: ping 1.6s ease-out infinite;
  }

  @keyframes ping {
    from {
      opacity: 0.9;
      transform: scale(0.5);
    }
    to {
      opacity: 0;
      transform: scale(1.5);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .mark.lit::after {
      animation: none;
      opacity: 0.5;
    }
  }
</style>
