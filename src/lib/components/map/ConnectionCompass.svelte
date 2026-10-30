<script lang="ts">
  // The current map in the middle with its neighbours around it, placed by portal name:
  // west and east to the sides, "in" above and "out" below. No coordinates or map images.
  import { ArrowDown, ArrowLeft, ArrowRight, ArrowUp, ArrowUpRight, EyeOff, MapPin } from "@lucide/svelte";

  import type { Direction, MapExit } from "$lib/types";

  let { exits, onnavigate }: { exits: MapExit[]; onnavigate: (id: number) => void } = $props();

  const by = (direction: Direction) => exits.filter((e) => e.direction === direction);
  const west = $derived(by("west"));
  const east = $derived(by("east"));
  const up = $derived(by("in"));
  const down = $derived(by("out"));
  const other = $derived(by("other"));

  /** More neighbours than this in one direction fold behind a "+N more" chip. */
  const SHOWN = 3;
  let expanded = $state<Partial<Record<Direction, boolean>>>({});

  const icons = { west: ArrowLeft, east: ArrowRight, in: ArrowUp, out: ArrowDown, other: ArrowUpRight };
  const words = { west: "West", east: "East", in: "In", out: "Out", other: "Portal" };

  function title(exit: MapExit) {
    const portals = exit.portals.length ? ` (portal ${exit.portals.join(", ")})` : "";
    const street = exit.to.street ? `${exit.to.street} : ` : "";
    const hidden = exit.hidden ? "\nThe portal's name marks it as hidden." : "";
    return exit.to.known
      ? `${words[exit.direction]} to ${street}${exit.to.name}${portals}${hidden}`
      : `Leads to map ${exit.to.id}, which the data does not include${portals}`;
  }
</script>

{#snippet group(direction: Direction, list: MapExit[])}
  {#each expanded[direction] || list.length <= SHOWN ? list : list.slice(0, SHOWN - 1) as exit}{@render node(exit)}{/each}
  {#if list.length > SHOWN}
    <button class="more" onclick={() => (expanded[direction] = !expanded[direction])}>
      {expanded[direction] ? "Show fewer" : `+${list.length - SHOWN + 1} more`}
    </button>
  {/if}
{/snippet}

{#snippet node(exit: MapExit)}
  {@const Icon = icons[exit.direction]}
  <button
    class="node {exit.direction}"
    class:unknown={!exit.to.known}
    disabled={!exit.to.known}
    title={title(exit)}
    onclick={() => onnavigate(exit.to.id)}
  >
    <span class="dir"><Icon size={11} />{words[exit.direction]}</span>
    <span class="label">{exit.to.name}</span>
    {#if exit.hidden}<span class="hidden" aria-label="Hidden portal"><EyeOff size={11} /></span>{/if}
  </button>
{/snippet}

{#if exits.length}
  <div class="compass" class:has-up={up.length} class:has-down={down.length}>
    {#if up.length}
      <div class="slot up">{@render group("in", up)}</div>
      <span class="wire v top"></span>
    {/if}

    <div class="slot west">{@render group("west", west)}</div>
    {#if west.length}<span class="wire h left"></span>{/if}
    <div class="here" title="You are here"><MapPin size={13} /><span>Here</span></div>
    {#if east.length}<span class="wire h right"></span>{/if}
    <div class="slot east">{@render group("east", east)}</div>

    {#if down.length}
      <span class="wire v bottom"></span>
      <div class="slot down">{@render group("out", down)}</div>
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
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 5px 9px;
    border-radius: 999px;
    border: 1px solid color-mix(in srgb, var(--map) 45%, transparent);
    background: color-mix(in srgb, var(--map) 16%, transparent);
    color: var(--map);
    font-size: 11px;
    font-weight: 600;
    white-space: nowrap;
  }

  .node {
    position: relative;
    display: grid;
    gap: 1px;
    max-width: 100%;
    min-width: 0;
    padding: 4px 8px 5px;
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
    gap: 3px;
    font-size: 9.5px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--faint);
  }

  .label {
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    font-size: 11.5px;
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
    border-radius: 999px;
    background: transparent;
    color: var(--muted);
    font-size: 10.5px;
    cursor: pointer;
  }

  .more:hover {
    color: var(--text);
    border-color: color-mix(in srgb, var(--map) 55%, transparent);
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
    font-size: 10.5px;
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
    font-size: 12px;
    color: var(--faint);
  }
</style>
