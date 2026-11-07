<script lang="ts">
  // A map in the middle with its neighbours around it. Portals on the left and right of the map
  // stack in those columns, highest first, and those in the middle go above or below it. Their
  // place is the coarse side and height from the map data; a portal without one is placed by its
  // name. Never drawn at real positions, and no map images. The pin only ever marks the map the
  // player is on.
  import { ArrowDown, ArrowLeft, ArrowRight, ArrowUp, ArrowUpRight, EyeOff, MapPin } from "@lucide/svelte";

  import type { MapExit, MapRef, Side } from "$lib/types";

  let {
    exits,
    map,
    current = null,
    onnavigate,
  }: {
    exits: MapExit[];
    /** The map being shown. */
    map: MapRef;
    /** The map the player is on, when known. */
    current?: MapRef | null;
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

  const top = { high: 0, middle: 1, low: 2 };
  const by = (slot: Slot) =>
    exits
      .filter((e) => slotOf(e) === slot)
      .sort((a, b) => top[a.height ?? "middle"] - top[b.height ?? "middle"]);
  const left = $derived(by("left"));
  const right = $derived(by("right"));
  const up = $derived(by("above"));
  const down = $derived(by("below"));
  const other = $derived(by("other"));

  /** More neighbours than this in one slot fold behind a "+N more" chip. */
  const SHOWN = 3;
  let expanded = $state<Partial<Record<Slot, boolean>>>({});

  const icons = { left: ArrowLeft, right: ArrowRight, above: ArrowUp, below: ArrowDown, other: ArrowUpRight };
  const nameWords = { west: "West", east: "East", in: "In", out: "Out", other: "Portal" };
  const sideWords = { left: "Left", centre: "Middle", right: "Right" };

  function word(exit: MapExit) {
    if (!exit.side) return nameWords[exit.direction];
    const height = exit.height === "high" ? " · high" : exit.height === "low" ? " · low" : "";
    return sideWords[exit.side] + height;
  }

  function title(exit: MapExit) {
    const street = exit.to.street ? `${exit.to.street} : ` : "";
    const hidden = exit.hidden ? "\nThe portal's name marks it as hidden." : "";
    const yours = exit.to.id === current?.id ? "\nYou are on this map." : "";
    return exit.to.known
      ? `${word(exit)} to ${street}${exit.to.name}${hidden}${yours}`
      : "Leads to a map the data does not include";
  }
</script>

{#snippet group(slot: Slot, list: MapExit[])}
  {#each expanded[slot] || list.length <= SHOWN ? list : list.slice(0, SHOWN - 1) as exit}{@render node(exit)}{/each}
  {#if list.length > SHOWN}
    <button class="more" onclick={() => (expanded[slot] = !expanded[slot])}>
      {expanded[slot] ? "Show fewer" : `+${list.length - SHOWN + 1} more`}
    </button>
  {/if}
{/snippet}

{#snippet node(exit: MapExit)}
  {@const yours = exit.to.id === current?.id}
  {@const Icon = yours ? MapPin : icons[slotOf(exit)]}
  <button
    class="node {exit.direction}"
    class:yours
    class:unknown={!exit.to.known}
    disabled={!exit.to.known}
    title={exit.to.known ? undefined : "Not in the map data"}
    onclick={() => onnavigate(exit.to.id)}
  >
    <span class="dir"><Icon size={10} />{word(exit)}</span>
    <span class="label">{exit.to.name}</span>
    {#if exit.hidden}<span class="hidden" aria-label="Hidden portal"><EyeOff size={11} /></span>{/if}
  </button>
{/snippet}

{#if exits.length}
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
{:else}
  <p class="none">No portals out of this map are listed.</p>
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
      background 0.12s ease;
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

  .node:hover:not(:disabled) {
    border-color: color-mix(in srgb, var(--map) 55%, transparent);
    background: color-mix(in srgb, var(--map) 10%, var(--surface-3));
  }

  .node:focus-visible {
    outline: 2px solid var(--map);
    outline-offset: 1px;
  }

  .dir {
    display: flex;
    align-items: center;
    gap: 2px;
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

  .node.yours {
    border-color: color-mix(in srgb, var(--map) 45%, transparent);
    background: color-mix(in srgb, var(--map) 16%, transparent);
  }

  .node.yours .dir {
    color: var(--map);
  }

  .node.unknown {
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

  .none {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--faint);
  }
</style>
