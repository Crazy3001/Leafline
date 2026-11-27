<script lang="ts">
  // The compass's map: the map you're on, its picture in a box of fixed size, with every portal
  // out marked where it stands. The portal the trip takes next is a green ring with its name;
  // with no trip, the left-most and right-most ways out carry short names and the rest (doors,
  // buildings) are small dots named on hover. The whole map always shows, so every portal is in
  // view: a wide map fills the box, a tall one stands in its middle. Without the picture (game pictures off, or not
  // downloaded yet) the marks stand on a plain box in the same places; without the picture's
  // marks, each portal sits in the third of the map the data names, and the box says so.
  import { pictureUrl } from "$lib/pictures";
  import { display } from "$lib/settings.svelte";
  import { tip } from "$lib/tip.svelte";
  import type { MapExit, MapPicture, MapRef } from "$lib/types";

  let {
    map,
    picture,
    exits,
    next = null,
    height = 96,
    onportal,
  }: {
    map: MapRef;
    /** The picture's size and marks; null when the data has none. */
    picture: MapPicture | null;
    /** The map's ways out, for names, doors and hidden portals. */
    exits: MapExit[];
    /** The portal the trip takes next from this map. */
    next?: { portal: string | null; to: number } | null;
    /** The box's height in pixels; its width is the space it's given. */
    height?: number;
    /** A portal clicked, with its mark, so its map's card can open beside it. */
    onportal: (to: number, from: Element) => void;
  } = $props();

  type Mark = { x: number; y: number; to: number; name: string; portal: string | null; exit: MapExit | null };

  const exitTo = (to: number, portal: string | null) =>
    exits.find((e) => portal != null && e.portals.includes(portal)) ?? exits.find((e) => e.to.id === to) ?? null;

  const across = { left: 0.15, centre: 0.5, right: 0.85 };
  const down = { high: 0.2, middle: 0.5, low: 0.8 };
  /** From the picture's marks when it has them; otherwise roughly, from the data's thirds. */
  const placed = $derived(Boolean(picture?.points.some((p) => p.kind === "portal" && !p.teleport)));
  const marks = $derived.by<Mark[]>(() => {
    if (placed)
      return picture!.points
        .filter((p) => p.kind === "portal" && !p.teleport && p.id !== map.id)
        .map((p) => ({ x: p.x, y: p.y, to: p.id, name: p.name, portal: p.portal, exit: exitTo(p.id, p.portal) }));
    return exits.map((e) => ({
      x: across[e.side ?? (e.direction === "west" ? "left" : e.direction === "east" ? "right" : "centre")],
      y: down[e.height ?? "middle"],
      to: e.to.id,
      name: e.to.name,
      portal: e.portals[0] ?? null,
      exit: e,
    }));
  });

  /** The marks for the next portal: by its own name, or else any to the next map. */
  const nextMarks = $derived.by(() => {
    if (!next) return new Set<Mark>();
    const named = next.portal ? marks.filter((m) => m.portal === next.portal) : [];
    return new Set(named.length ? named : marks.filter((m) => m.to === next.to));
  });

  /** A way across rather than a door: the map's side portals, and anything the data doesn't call in or out. */
  const door = (m: Mark) => m.exit?.direction === "in" || m.exit?.direction === "out";
  /** With no trip, the furthest left and right ways across carry names. */
  const named = $derived.by(() => {
    if (next) return new Set(nextMarks);
    const across = marks.filter((m) => !door(m) && !m.exit?.hidden);
    if (!across.length) return new Set<Mark>();
    const byX = [...across].sort((a, b) => a.x - b.x);
    return new Set([byX[0], byX[byX.length - 1]]);
  });

  let width = $state(0);
  let failed = $state(false);
  let loaded = $state(false);
  const mapId = $derived(map.id);
  $effect(() => {
    void mapId;
    failed = false;
    loaded = false;
  });
  const src = $derived(picture && display.showPictures && !failed ? pictureUrl("mapview", mapId) : null);

  /** The whole map, as big as the box allows, in its middle. Without the picture's size, a wide box. */
  const frame = $derived.by(() => {
    if (!width) return null;
    const pw = placed ? picture!.width : width;
    const ph = placed ? picture!.height : height;
    const scale = Math.min(width / pw, height / ph);
    const w = pw * scale;
    const h = ph * scale;
    return { w, h, left: (width - w) / 2, top: (height - h) / 2 };
  });

  function title(m: Mark) {
    const what = m.exit?.hidden ? "Hidden portal" : door(m) ? "Door" : "Portal";
    return `${nextMarks.has(m) ? "Next · " : ""}${what} to ${m.name}`;
  }
</script>

<div class="box" class:rough={!placed} style:height="{height}px" bind:clientWidth={width}>
  {#if frame}
    <div class="layer" style:width="{frame.w}px" style:height="{frame.h}px" style:left="{frame.left}px" style:top="{frame.top}px">
      <!-- The frame hugs the map; the box's room around it stays the card's. -->
      <span class="frame"></span>
      {#if src}
        <img {src} alt="" class:ready={loaded} draggable="false" onload={() => (loaded = true)} onerror={() => (failed = true)} />
      {/if}
      {#each marks as m, i (i)}
        {@const lit = nextMarks.has(m)}
        {@const label = named.has(m)}
        <button
          class="mark"
          class:lit
          class:door={door(m)}
          class:hidden={m.exit?.hidden}
          class:right={m.x > 0.6}
          style:left="{m.x * 100}%"
          style:top="{m.y * 100}%"
          aria-label={title(m)}
          use:tip={label ? null : title(m)}
          onclick={(e) => onportal(m.to, e.currentTarget)}
        >
          <span class="dot"></span>
          {#if label}<span class="name">{m.name}</span>{/if}
        </button>
      {/each}
    </div>
  {/if}
  {#if !placed && marks.length}
    <span class="rough-note" use:tip={"The data has no picture of this map: each portal stands in the third of it the data names"}>rough</span>
  {/if}
</div>

<style>
  .box {
    position: relative;
    overflow: hidden;
  }

  .layer {
    position: absolute;
  }

  .frame {
    position: absolute;
    inset: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--bg) 70%, var(--surface));
  }

  img {
    position: absolute;
    inset: 0;
    display: block;
    width: 100%;
    height: 100%;
    border-radius: var(--radius-sm);
    opacity: 0;
    filter: saturate(0.75) brightness(0.62);
    transition: opacity 0.2s ease;
    user-select: none;
  }

  img.ready {
    opacity: 1;
  }

  .mark {
    position: absolute;
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0;
    border: 0;
    background: none;
    cursor: pointer;
    transform: translate(-6px, -50%);
  }

  /* A mark on the right half puts its name on its left, so it stays in the box. */
  .mark.right {
    flex-direction: row-reverse;
    transform: translate(calc(-100% + 6px), -50%);
  }

  .mark:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
    border-radius: 4px;
  }

  .dot {
    flex: none;
    width: 12px;
    height: 12px;
    border: 2px solid var(--marker-portal);
    border-radius: 50%;
    background: var(--bg);
    box-shadow: 0 0 0 1px color-mix(in srgb, var(--bg) 70%, transparent);
  }

  .door .dot {
    width: 9px;
    height: 9px;
    margin: 0 1.5px;
    border-width: 1.5px;
    border-color: color-mix(in srgb, var(--marker-portal) 65%, transparent);
  }

  .hidden .dot {
    border-style: dashed;
    opacity: 0.6;
  }

  .mark:hover .dot {
    background: var(--marker-portal);
  }

  .name {
    max-width: 150px;
    overflow: hidden;
    padding: 1px 5px;
    border-radius: 4px;
    background: color-mix(in srgb, var(--bg) 82%, transparent);
    color: var(--text);
    font-size: var(--text-xs);
    line-height: 1.3;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  /* The next portal: a Leafline green ring that breathes, with its name on it. */
  .lit {
    z-index: 1;
  }

  .lit .dot {
    width: 14px;
    height: 14px;
    border: 3px solid var(--accent-strong);
    background: radial-gradient(circle, var(--accent-strong) 0 2.5px, var(--bg) 3px);
    animation: breathe 2.4s ease-in-out infinite;
  }

  .lit .name {
    background: var(--accent-strong);
    color: var(--accent-ink);
    font-weight: var(--weight-strong);
  }

  @keyframes breathe {
    0%,
    100% {
      box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 40%, transparent);
    }
    50% {
      box-shadow: 0 0 0 6px color-mix(in srgb, var(--accent) 12%, transparent);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .lit .dot {
      animation: none;
      box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 35%, transparent);
    }
  }

  .rough-note {
    position: absolute;
    right: 6px;
    bottom: 4px;
    color: var(--faint);
    font-size: var(--text-2xs, 10px);
  }
</style>
