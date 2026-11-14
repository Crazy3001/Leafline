<script lang="ts">
  // A map in the middle with its neighbours around it, or the same portals as a plain list.
  // Portals on the left and right of the map stack in those columns, and those in the middle go
  // above or below it. Their slot is the coarse side and height from the map data (the same words
  // the route bar uses); a portal without one is placed by its name. When the map's picture has
  // the portal's real spot, a small locator shows it and the portals in a slot follow their real
  // order. The pin only ever marks the map the player is on; the trip's next portal is flagged.
  // Pointing at a portal lights it on the map picture, and the other way round.
  import { tip } from "$lib/tip.svelte";
  import { ArrowDown, ArrowLeft, ArrowRight, ArrowUp, ArrowUpRight, EyeOff, MapPin } from "@lucide/svelte";

  import type { MapExit, MapRef, Side, ViewPoint } from "$lib/types";

  let {
    exits,
    map,
    current = null,
    points = null,
    next = null,
    lit = null,
    layout = "compass",
    onhover,
    onnavigate,
  }: {
    exits: MapExit[];
    /** The map being shown. */
    map: MapRef;
    /** The map the player is on, when known. */
    current?: MapRef | null;
    /** The map picture's marks, for where each portal really stands; null without a picture. */
    points?: ViewPoint[] | null;
    /** The portal the trip being followed takes next from this map. */
    next?: { portal: string | null; to: number } | null;
    /** A map whose portal is pointed at on the picture. */
    lit?: number | null;
    layout?: "compass" | "list";
    /** Pointing at a portal, or leaving it. */
    onhover?: (to: number | null) => void;
    onnavigate: (id: number) => void;
  } = $props();

  const here = $derived(current?.id === map.id);

  type Slot = "left" | "right" | "above" | "below" | "other";

  const sideByName: Record<MapExit["direction"], Side | null> = {
    west: "left",
    east: "right",
    in: "centre",
    out: "centre",
    other: null,
  };

  function slotOf(exit: MapExit): Slot {
    const side = exit.side ?? sideByName[exit.direction];
    if (side === null) return "other";
    if (side !== "centre") return side;
    const high = exit.height ? exit.height === "high" : exit.direction === "in";
    return high ? "above" : "below";
  }

  /** Where each exit's portals stand on the picture, 0 to 1 across and down, by the map they lead to. */
  const spots = $derived.by(() => {
    const out = new Map<MapExit, { x: number; y: number }>();
    if (!points) return out;
    const portals = points.filter((p) => p.kind === "portal" && !p.teleport);
    for (const exit of exits) {
      const named = portals.filter((p) => p.portal && exit.portals.includes(p.portal));
      const found = named.length ? named : portals.filter((p) => p.id === exit.to.id);
      if (!found.length) continue;
      out.set(exit, {
        x: found.reduce((s, p) => s + p.x, 0) / found.length,
        y: found.reduce((s, p) => s + p.y, 0) / found.length,
      });
    }
    return out;
  });

  const thirds = { high: 0.17, middle: 0.5, low: 0.83 };
  const across = { left: 0.17, centre: 0.5, right: 0.83 };
  /** The real spot when known, else the middle of the third the data names. */
  function spotOf(exit: MapExit) {
    return (
      spots.get(exit) ?? {
        x: across[exit.side ?? (sideByName[exit.direction] || "centre")],
        y: thirds[exit.height ?? "middle"],
      }
    );
  }

  /** Columns go top to bottom; the rows above and below the map go left to right. */
  const by = (slot: Slot) => {
    const list = exits.filter((e) => slotOf(e) === slot);
    const key = slot === "above" || slot === "below" ? (e: MapExit) => spotOf(e).x : (e: MapExit) => spotOf(e).y;
    return list.sort((a, b) => key(a) - key(b));
  };
  const left = $derived(by("left"));
  const right = $derived(by("right"));
  const up = $derived(by("above"));
  const down = $derived(by("below"));
  const other = $derived(by("other"));
  /** Every portal for the list: left to right across the map, top to bottom within each third. */
  const listed = $derived(
    [...exits].sort((a, b) => {
      const pa = spotOf(a);
      const pb = spotOf(b);
      const ta = Math.min(Math.floor(pa.x * 3), 2);
      const tb = Math.min(Math.floor(pb.x * 3), 2);
      return ta - tb || pa.y - pb.y || pa.x - pb.x;
    }),
  );

  const isNext = (exit: MapExit) => next != null && exit.to.id === next.to;

  /** More neighbours than this in one slot fold behind a "+N more" chip; the next portal never folds away. */
  const SHOWN = 3;
  let expanded = $state<Partial<Record<Slot, boolean>>>({});
  function shown(slot: Slot, list: MapExit[]) {
    if (expanded[slot] || list.length <= SHOWN) return list;
    const first = list.slice(0, SHOWN - 1);
    const kept = list.find((e, i) => i >= SHOWN - 1 && isNext(e));
    return kept ? [...first, kept] : first;
  }

  const icons = { left: ArrowLeft, right: ArrowRight, above: ArrowUp, below: ArrowDown, other: ArrowUpRight };
  const nameWords = { west: "West", east: "East", in: "In", out: "Out", other: "Portal" };
  const sideWords = { left: "Left", centre: "Middle", right: "Right" };

  function word(exit: MapExit) {
    if (!exit.side) return nameWords[exit.direction];
    const height = exit.height === "high" ? " · high" : exit.height === "low" ? " · low" : "";
    return sideWords[exit.side] + height;
  }

  /** Only what the card doesn't already say; nothing when that is all. */
  function title(exit: MapExit) {
    if (!exit.to.known) return "Leads to a map the data does not include";
    const lines = [
      exit.to.street ? `On ${exit.to.street}` : null,
      exit.portals.length > 1 ? `${exit.portals.length} portals lead there` : null,
      exit.hidden ? "The portal's name marks it as hidden: the game draws nothing there" : null,
      exit.to.id === current?.id ? "You are on this map" : null,
      isNext(exit) ? "Your next step on the trip" : null,
    ].filter(Boolean);
    return lines.length ? lines.join("\n") : undefined;
  }
</script>

{#snippet locator(exit: MapExit)}
  {@const spot = spots.get(exit)}
  {#if spot}
    <span class="locator" aria-hidden="true"><i style:left="{spot.x * 100}%" style:top="{spot.y * 100}%"></i></span>
  {/if}
{/snippet}

{#snippet group(slot: Slot, list: MapExit[])}
  {@const visible = shown(slot, list)}
  {#each visible as exit}{@render node(exit, slot)}{/each}
  {#if list.length > SHOWN}
    <button class="more" onclick={() => (expanded[slot] = !expanded[slot])}>
      {expanded[slot] ? "Show fewer" : `+${list.length - visible.length} more`}
    </button>
  {/if}
{/snippet}

{#snippet node(exit: MapExit, slot: Slot)}
  {@const yours = exit.to.id === current?.id}
  {@const Icon = yours ? MapPin : icons[slot]}
  <button
    class="node {exit.direction}"
    class:yours
    class:next={isNext(exit)}
    class:lit={lit === exit.to.id}
    class:unknown={!exit.to.known}
    disabled={!exit.to.known}
    use:tip={title(exit)}
    onclick={() => onnavigate(exit.to.id)}
    onmouseenter={() => onhover?.(exit.to.id)}
    onmouseleave={() => onhover?.(null)}
    onfocus={() => onhover?.(exit.to.id)}
    onblur={() => onhover?.(null)}
  >
    <span class="dir">
      {#if slot === "left"}{@render locator(exit)}{/if}
      <Icon size={10} />{word(exit)}
      {#if slot !== "left"}{@render locator(exit)}{/if}
    </span>
    <span class="label">{exit.to.name}</span>
    {#if isNext(exit)}<span class="flag">Next</span>{:else if exit.hidden}<span class="hidden" aria-label="Hidden portal"
        ><EyeOff size={11} /></span
      >{/if}
  </button>
{/snippet}

{#if !exits.length}
  <p class="none">No portals out of this map are listed.</p>
{:else if layout === "list"}
  <ul class="list">
    {#each listed as exit}
      {@const yours = exit.to.id === current?.id}
      {@const Icon = yours ? MapPin : icons[slotOf(exit)]}
      <li>
        <button
          class="row"
          class:yours
          class:next={isNext(exit)}
          class:lit={lit === exit.to.id}
          class:unknown={!exit.to.known}
          disabled={!exit.to.known}
          use:tip={exit.to.known ? undefined : "Not in the map data"}
          onclick={() => onnavigate(exit.to.id)}
          onmouseenter={() => onhover?.(exit.to.id)}
          onmouseleave={() => onhover?.(null)}
          onfocus={() => onhover?.(exit.to.id)}
          onblur={() => onhover?.(null)}
        >
          <span class="locator-cell">{@render locator(exit)}</span>
          <span class="row-dir"><Icon size={11} />{word(exit)}</span>
          <span class="row-name">{exit.to.name}</span>
          {#if exit.portals.length > 1}<span class="faint tabular" use:tip={`${exit.portals.length} portals lead there`}
              >×{exit.portals.length}</span
            >{/if}
          {#if exit.hidden}<span class="hidden-inline" use:tip={"The portal's name marks it as hidden"}><EyeOff size={11} /></span>{/if}
          {#if isNext(exit)}<span class="flag inline">Next</span>{/if}
        </button>
      </li>
    {/each}
  </ul>
{:else}
  <div class="compass" class:has-up={up.length} class:has-down={down.length}>
    {#if up.length}
      <div class="slot up">{@render group("above", up)}</div>
      <span class="wire v top"></span>
    {/if}

    <div class="slot west">{@render group("left", left)}</div>
    {#if left.length}<span class="wire h left"></span>{/if}
    <div class="here" class:away={!here}>
      {#if here}<MapPin size={13} />{/if}
      <span>{map.name}</span>
    </div>
    {#if right.length}<span class="wire h right"></span>{/if}
    <div class="slot east">{@render group("right", right)}</div>

    {#if down.length}
      <span class="wire v bottom"></span>
      <div class="slot down">{@render group("below", down)}</div>
    {/if}
  </div>

  {#if other.length}
    <div class="others">
      <span class="others-label">Other portals</span>
      <div class="other-list">{@render group("other", other)}</div>
    </div>
  {/if}
{/if}

<style>
  .compass {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 12px auto 12px minmax(0, 1fr);
    grid-template-rows: auto;
    grid-template-areas: "west wl here wr east";
    align-items: center;
    padding: 4px 0 2px;
  }

  .compass.has-up {
    grid-template-rows: auto 12px auto;
    grid-template-areas:
      "up up up up up"
      ". . vt . ."
      "west wl here wr east";
  }

  .compass.has-down {
    grid-template-rows: auto 12px auto;
    grid-template-areas:
      "west wl here wr east"
      ". . vb . ."
      "down down down down down";
  }

  .compass.has-up.has-down {
    grid-template-rows: auto 12px auto 12px auto;
    grid-template-areas:
      "up up up up up"
      ". . vt . ."
      "west wl here wr east"
      ". . vb . ."
      "down down down down down";
  }

  .slot {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }

  .slot.west {
    grid-area: west;
    align-items: flex-end;
  }

  .slot.east {
    grid-area: east;
    align-items: flex-start;
  }

  .slot.up,
  .slot.down {
    flex-flow: row wrap;
    justify-content: center;
  }

  .slot.up {
    grid-area: up;
  }

  .slot.down {
    grid-area: down;
  }

  .wire {
    background: var(--border-strong);
  }

  .wire.h {
    height: 1px;
    align-self: center;
  }

  .wire.left {
    grid-area: wl;
  }

  .wire.right {
    grid-area: wr;
  }

  .wire.v {
    width: 1px;
    height: 100%;
    justify-self: center;
  }

  .wire.top {
    grid-area: vt;
  }

  .wire.bottom {
    grid-area: vb;
  }

  .here {
    grid-area: here;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-self: center;
    gap: 2px;
    max-width: 84px;
    padding: 5px 6px;
    border-radius: var(--radius-sm);
    border: 1px solid color-mix(in srgb, var(--map) 45%, transparent);
    background: color-mix(in srgb, var(--map) 16%, transparent);
    color: var(--map);
    font-size: var(--text-xs);
    font-weight: 600;
    line-height: 1.25;
    text-align: center;
    overflow-wrap: anywhere;
  }

  .here.away {
    border-color: var(--border-strong);
    background: transparent;
    color: var(--text);
  }

  .node {
    position: relative;
    display: grid;
    gap: 1px;
    max-width: 100%;
    min-width: 0;
    padding: 4px 6px 5px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface-3);
    text-align: left;
    cursor: pointer;
    transition:
      border-color 0.12s ease,
      background 0.12s ease,
      box-shadow 0.12s ease;
  }

  .slot.west .node {
    text-align: right;
  }

  .slot.west .dir {
    justify-content: flex-end;
  }

  .slot.up .node,
  .slot.down .node {
    max-width: min(100%, 220px);
    text-align: center;
  }

  .slot.up .dir,
  .slot.down .dir {
    justify-content: center;
  }

  .node:hover:not(:disabled),
  .node.lit {
    border-color: color-mix(in srgb, var(--map) 55%, transparent);
    background: color-mix(in srgb, var(--map) 10%, var(--surface-3));
  }

  .node.lit {
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--marker-portal) 45%, transparent);
  }

  .node:focus-visible {
    outline: 2px solid var(--map);
    outline-offset: 1px;
  }

  .dir {
    display: flex;
    align-items: center;
    gap: 3px;
    font-size: var(--text-xs);
    font-weight: 600;
    white-space: nowrap;
    color: var(--faint);
  }

  .label {
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    font-size: var(--text-sm);
    font-weight: 500;
    line-height: 1.25;
    overflow-wrap: anywhere;
  }

  /* Where the portal really stands: a tiny frame of the map with a dot in it. */
  .locator {
    position: relative;
    flex: none;
    width: 20px;
    height: 11px;
    border: 1px solid var(--border-strong);
    border-radius: 2px;
    background: color-mix(in srgb, var(--bg) 60%, transparent);
  }

  .locator i {
    position: absolute;
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--marker-portal);
    transform: translate(-50%, -50%);
  }

  .next .locator i {
    background: var(--accent-strong);
  }

  .hidden {
    position: absolute;
    top: 3px;
    right: 4px;
    color: var(--warn);
  }

  .slot.west .hidden {
    right: auto;
    left: 4px;
  }

  /* The trip's next portal: the accent edge and a small flag on the corner. */
  .node.next,
  .row.next {
    border-color: color-mix(in srgb, var(--accent) 70%, transparent);
    background: color-mix(in srgb, var(--accent) 12%, var(--surface-3));
  }

  .node.next .dir {
    color: var(--accent-strong);
  }

  .flag {
    position: absolute;
    top: -7px;
    right: 6px;
    padding: 0 4px;
    border-radius: var(--radius-sm);
    background: var(--accent-strong);
    color: var(--accent-ink);
    font-size: 10px;
    font-weight: 700;
    line-height: 13px;
  }

  .slot.west .flag {
    right: auto;
    left: 6px;
  }

  .flag.inline {
    position: static;
    margin-left: auto;
  }

  .more {
    align-self: center;
    padding: 3px 8px;
    border: 1px dashed var(--border-strong);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--muted);
    font-size: var(--text-xs);
    cursor: pointer;
  }

  .more:hover {
    color: var(--text);
    border-color: color-mix(in srgb, var(--map) 55%, transparent);
  }

  .node.yours,
  .row.yours {
    border-color: color-mix(in srgb, var(--map) 45%, transparent);
    background: color-mix(in srgb, var(--map) 16%, transparent);
  }

  .node.yours .dir {
    color: var(--map);
  }

  .node.unknown,
  .row.unknown {
    border-style: dashed;
    background: transparent;
    color: var(--faint);
    cursor: default;
  }

  .others {
    display: grid;
    gap: 5px;
    margin-top: 10px;
  }

  .others-label {
    font-size: var(--text-xs);
    color: var(--faint);
  }

  .other-list {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .other-list .node {
    max-width: 48%;
  }

  /* The list: one row per portal, left to right across the map. */
  .list {
    display: grid;
    gap: 3px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    min-height: 28px;
    padding: 3px 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface-3);
    font-size: var(--text-sm);
    text-align: left;
    cursor: pointer;
    transition:
      border-color 0.12s ease,
      background 0.12s ease;
  }

  .row:hover:not(:disabled),
  .row.lit {
    border-color: color-mix(in srgb, var(--map) 55%, transparent);
    background: color-mix(in srgb, var(--map) 10%, var(--surface-3));
  }

  .row:focus-visible {
    outline: 2px solid var(--map);
    outline-offset: 1px;
  }

  .locator-cell {
    display: inline-grid;
    flex: none;
    width: 20px;
  }

  .row-dir {
    display: inline-flex;
    flex: none;
    align-items: center;
    gap: 3px;
    width: 92px;
    color: var(--faint);
    font-size: var(--text-xs);
    font-weight: 600;
    white-space: nowrap;
  }

  .row.next .row-dir {
    color: var(--accent-strong);
  }

  .row.yours .row-dir {
    color: var(--map);
  }

  .row-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    font-weight: 500;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .hidden-inline {
    display: inline-grid;
    color: var(--warn);
  }

  .faint {
    color: var(--faint);
    font-size: var(--text-xs);
  }

  .none {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--faint);
  }
</style>
