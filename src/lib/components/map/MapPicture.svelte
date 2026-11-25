<script lang="ts">
  // A map's picture, cut to what is drawn and dimmed, with its portals, NPCs and spawn points
  // marked where they stand, at the same size however wide the panel. Each mark opens its card;
  // a teleport within the map points out where it comes out instead. The picture is downloaded
  // the first time it is shown.
  //
  // The wheel zooms in on the pointer and dragging moves around; the marks stay the same size,
  // so a crowded town spreads out. Zoomed all the way out, the wheel scrolls the panel as usual,
  // so the picture never traps it. The portal the trip takes next, and portals pointed at in the
  // Portals section, stand out. Spawn points show the monster's own picture, growing a little as
  // the zoom goes in; a dot stands in when pictures are off or missing.
  import { LoaderCircle, Minus, Plus, Scan } from "@lucide/svelte";

  import { openPeek } from "$lib/peek.svelte";
  import { pictureUrl } from "$lib/pictures";
  import { formatNumber } from "$lib/format";
  import { tip } from "$lib/tip.svelte";
  import type { MapPicture, MobRow, ViewPoint } from "$lib/types";
  import Pic from "$lib/ui/Pic.svelte";

  let {
    mapId,
    picture,
    done,
    highlight = null,
    mobs = [],
    npcs = [],
    next = null,
    lit = null,
    onhoverportal,
  }: {
    mapId: number;
    /** The picture's size and marks, fetched by the map's card; null while loading or missing. */
    picture: MapPicture | null;
    /** The fetch has finished, found or not. */
    done: boolean;
    /** The map's monsters, for their levels in a spawn point's tooltip. */
    mobs?: Pick<MobRow, "id" | "name" | "level" | "hp" | "exp" | "boss">[];
    /** The map's NPCs, for how many quests each gives. */
    npcs?: { id: number; quests: unknown[] }[];
    /** A monster whose spawn points stand out, such as the one under the pointer in the list. */
    highlight?: string | null;
    /** The portal the trip being followed takes next from this map. */
    next?: { portal: string | null; to: number } | null;
    /** A map whose portals stand out, as when its card in the Portals section is pointed at. */
    lit?: number | null;
    /** Pointing at a portal out, or leaving it. */
    onhoverportal?: (to: number | null) => void;
  } = $props();

  let failed = $state(false);
  let loaded = $state(false);
  /** The teleport whose exit is pointed out, by the clicked one's index. */
  let paired = $state<number | null>(null);

  // Zoom: how many times bigger, and the picture's point (0 to 1 across and down) at the middle.
  const MAX_ZOOM = 6;
  let zoom = $state(1);
  let cx = $state(0.5);
  let cy = $state(0.5);
  let width = $state(0);

  $effect(() => {
    void mapId;
    failed = false;
    loaded = false;
    paired = null;
    zoom = 1;
    cx = 0.5;
    cy = 0.5;
  });

  const src = $derived(picture ? pictureUrl("mapview", mapId) : null);
  const height = $derived(picture && width ? (width * picture.height) / picture.width : 0);
  /** Keeps the middle where the picture still fills the frame. */
  function settle() {
    const half = 1 / (2 * zoom);
    cx = Math.min(Math.max(cx, half), 1 - half);
    cy = Math.min(Math.max(cy, half), 1 - half);
  }
  const left = $derived(width / 2 - cx * width * zoom);
  const top = $derived(height / 2 - cy * height * zoom);

  /** Zooms to `to`, keeping the picture's point under (x, y) in the frame where it is. */
  function zoomTo(to: number, x = width / 2, y = height / 2) {
    const next = Math.min(Math.max(to, 1), MAX_ZOOM);
    if (!width || !height || next === zoom) return;
    const u = (x - left) / (width * zoom);
    const v = (y - top) / (height * zoom);
    cx = u - (x - width / 2) / (width * next);
    cy = v - (y - height / 2) / (height * next);
    zoom = next;
    settle();
  }

  function reset() {
    zoom = 1;
    cx = 0.5;
    cy = 0.5;
  }

  /** The wheel zooms; at either end it lets the panel scroll instead. Needs a listener that can cancel. */
  function wheel(node: HTMLElement) {
    const on = (e: WheelEvent) => {
      if (e.ctrlKey && e.deltaMode === 0 && Math.abs(e.deltaY) < 10) return; // a pinch: the window zooms
      const step = e.deltaMode === 1 ? 0.08 : 0.0025;
      const to = Math.min(Math.max(zoom * Math.exp(-e.deltaY * step), 1), MAX_ZOOM);
      if (to === zoom) return;
      e.preventDefault();
      const box = node.getBoundingClientRect();
      zoomTo(to, e.clientX - box.left, e.clientY - box.top);
    };
    node.addEventListener("wheel", on, { passive: false });
    return { destroy: () => node.removeEventListener("wheel", on) };
  }

  // Dragging moves around a zoomed picture. A press only becomes a drag once it moves a little,
  // so clicking a mark still opens its card; the click that ends a drag opens nothing.
  let press: { id: number; x: number; y: number; cx: number; cy: number } | null = null;
  let dragging = $state(false);
  let dragged = false;

  function down(e: PointerEvent) {
    if (e.button !== 0 || zoom === 1) return;
    press = { id: e.pointerId, x: e.clientX, y: e.clientY, cx, cy };
  }

  function move(e: PointerEvent) {
    if (!press || e.pointerId !== press.id) return;
    const dx = e.clientX - press.x;
    const dy = e.clientY - press.y;
    if (!dragging) {
      if (Math.hypot(dx, dy) < 4) return;
      dragging = true;
      (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    }
    cx = press.cx - dx / (width * zoom);
    cy = press.cy - dy / (height * zoom);
    settle();
  }

  function up(e: PointerEvent) {
    if (!press || e.pointerId !== press.id) return;
    dragged = dragging;
    press = null;
    dragging = false;
  }

  function swallowDragClick(e: MouseEvent) {
    if (!dragged) return;
    dragged = false;
    e.stopPropagation();
    e.preventDefault();
  }

  /** Double-click: zoom in on the spot, or back out when already in. Not on a mark. */
  function doubleClick(e: MouseEvent) {
    if ((e.target as Element).closest(".mark, .zoom")) return;
    if (zoom > 1) return reset();
    const box = (e.currentTarget as HTMLElement).getBoundingClientRect();
    zoomTo(2.5, e.clientX - box.left, e.clientY - box.top);
  }

  /** Drawn bottom to top: teleports, spawns, NPCs, then portals out on top. */
  const ORDER = (p: ViewPoint) => (p.teleport ? 0 : p.kind === "mob" ? 1 : p.kind === "npc" ? 2 : 3);
  const marks = $derived(picture ? [...picture.points].sort((a, b) => ORDER(a) - ORDER(b)) : []);
  /** How many spawn points each monster has here, by id. */
  const spawnsOf = $derived(
    marks.reduce((n, p) => (p.kind === "mob" ? n.set(p.id, (n.get(p.id) ?? 0) + 1) : n), new Map<number, number>()),
  );
  const mobOf = (id: number) => mobs.find((m) => m.id === id) ?? null;
  const questsOf = (id: number) => npcs.find((n) => n.id === id)?.quests.length ?? 0;
  const counts = $derived({
    portals: marks.filter((p) => p.kind === "portal" && !p.teleport && !p.hidden).length,
    hidden: marks.filter((p) => p.kind === "portal" && !p.teleport && p.hidden).length,
    teleports: marks.filter((p) => p.teleport).length,
    npcs: marks.filter((p) => p.kind === "npc").length,
    spawns: marks.filter((p) => p.kind === "mob").length,
  });

  /** The portals out that the trip takes next: by the portal's own name, or else any to that map. */
  const nextMarks = $derived.by(() => {
    if (!next) return new Set<ViewPoint>();
    const out = marks.filter((p) => p.kind === "portal" && !p.teleport);
    const named = next.portal ? out.filter((p) => p.portal === next.portal) : [];
    return new Set(named.length ? named : out.filter((p) => p.id === next.to));
  });

  /** The other end of a teleport: where it comes out, or the teleports that come out here. */
  function endsOf(index: number): Set<number> {
    const from = marks[index];
    const ends = new Set<number>([index]);
    marks.forEach((p, i) => {
      if (!p.teleport || i === index) return;
      if ((from.toPortal && p.portal === from.toPortal) || (from.portal && p.toPortal === from.portal)) ends.add(i);
    });
    return ends;
  }

  function open(index: number, target: Element) {
    const point = marks[index];
    if (point.teleport) {
      paired = paired === index ? null : index;
      return;
    }
    if (!point.known) return;
    const kind = point.kind === "portal" ? "map" : point.kind;
    openPeek({ kind, id: point.id, name: point.name }, target);
  }

  function label(point: ViewPoint) {
    if (point.teleport) return `Teleport ${point.portal ?? ""} within this map`;
    if (point.kind === "portal") return `${point.hidden ? "Hidden portal" : "Portal"} to ${point.name}`;
    return point.kind === "npc" ? point.name : `${point.name} spawn`;
  }

  const pairedEnds = $derived(paired == null ? new Set<number>() : endsOf(paired));

  /** A spawn point's monster picture: small enough that a crowded map stays readable, bigger zoomed in. */
  const mobSize = $derived(Math.round(Math.min(34, 18 + (zoom - 1) * 6)));
</script>

{#if picture && src && !failed}
  <div
    class="stage"
    class:zoomed={zoom > 1}
    class:dragging
    style:aspect-ratio="{picture.width} / {picture.height}"
    bind:clientWidth={width}
    use:wheel
    onpointerdown={down}
    onpointermove={move}
    onpointerup={up}
    onpointercancel={up}
    onclickcapture={swallowDragClick}
    ondblclick={doubleClick}
    role="group"
    aria-label="Map picture"
  >
    <div class="layer" style:width="{zoom * 100}%" style:height="{zoom * 100}%" style:left="{left}px" style:top="{top}px">
      <img
        {src}
        alt=""
        class:ready={loaded}
        draggable="false"
        onload={() => (loaded = true)}
        onerror={() => (failed = true)}
      />
      {#if loaded}
        {#each marks as point, i (i)}
          {@const out = point.kind === "portal" && !point.teleport}
          <button
            class="mark {point.kind}"
            class:teleport={point.teleport}
            class:hidden={point.hidden && !point.teleport}
            class:lit={(point.kind === "mob" && highlight != null && point.name === highlight) || (out && lit === point.id)}
            class:dim={point.kind === "mob" && highlight != null && point.name !== highlight}
            class:next={nextMarks.has(point)}
            class:paired={pairedEnds.has(i)}
            class:unknown={!point.known && !point.teleport}
            style:left="{point.x * 100}%"
            style:top="{point.y * 100}%"
            aria-label={label(point)}
            use:tip={markTip}
            onclick={(e) => open(i, e.currentTarget)}
            onmouseenter={out ? () => onhoverportal?.(point.id) : undefined}
            onmouseleave={out ? () => onhoverportal?.(null) : undefined}
          >
            {#if point.kind === "mob"}
              <Pic kind="mob" id={point.id} name={point.name} size={mobSize} class="mob-pic">
                {#snippet fallback()}<span class="shape"></span>{/snippet}
              </Pic>
            {:else}
              <span class="shape"></span>
            {/if}
            {#if nextMarks.has(point)}<span class="flag">Next</span>{/if}
          </button>
          {#snippet markTip()}
            {#if point.teleport}
              <span class="tip-head">
                <span class="tip-key teleport" aria-hidden="true"></span>
                <span>
                  <span class="tip-title">Teleport within this map</span>
                  <span class="tip-sub">{point.toPortal ? "Click to see where it comes out" : "Where a teleport comes out"}</span>
                </span>
              </span>
            {:else if point.kind === "portal"}
              <span class="tip-head">
                <Pic kind="mark" id={point.id} name={point.name} size={26}>
                  {#snippet fallback()}<span class="tip-key portal" aria-hidden="true"></span>{/snippet}
                </Pic>
                <span>
                  <span class="tip-title">{point.name}</span>
                  <span class="tip-sub"
                    >{nextMarks.has(point) ? "Your next step · " : ""}{point.hidden ? "Hidden portal" : "Portal"}{point.known
                      ? " · click for the map's card"
                      : " · not in the map data"}</span
                  >
                </span>
              </span>
            {:else if point.kind === "npc"}
              {@const quests = questsOf(point.id)}
              <span class="tip-head">
                <Pic kind="npc" id={point.id} name={point.name} size={34} />
                <span>
                  <span class="tip-title">{point.name}</span>
                  <span class="tip-sub">NPC{quests ? ` · ${quests} quest${quests === 1 ? "" : "s"}` : ""}</span>
                </span>
              </span>
            {:else}
              {@const mob = mobOf(point.id)}
              {@const spots = spawnsOf.get(point.id) ?? 1}
              <span class="tip-head">
                <Pic kind="mob" id={point.id} name={point.name} size={34} />
                <span>
                  <span class="tip-title">{point.name}{mob?.boss ? " · boss" : ""}</span>
                  <span class="tip-sub tabular">{spots} spawn point{spots === 1 ? "" : "s"} on this map</span>
                </span>
              </span>
              {#if mob}
                <span class="tip-stats tabular">
                  <span><b>Lv</b> {mob.level}</span><span><b>HP</b> {formatNumber(mob.hp)}</span><span
                    ><b>EXP</b> {formatNumber(mob.exp)}</span
                  >
                </span>
              {/if}
            {/if}
          {/snippet}
        {/each}
      {/if}
    </div>

    {#if loaded}
      <div class="zoom" role="toolbar" aria-label="Zoom">
        <button use:tip={"Zoom out"} aria-label="Zoom out" disabled={zoom <= 1} onclick={() => zoomTo(zoom / 1.6)}
          ><Minus size={12} /></button
        >
        <span class="level tabular" aria-live="polite">{zoom < 1.05 ? "1" : zoom.toFixed(1)}×</span>
        <button use:tip={"Zoom in"} aria-label="Zoom in" disabled={zoom >= MAX_ZOOM} onclick={() => zoomTo(zoom * 1.6)}
          ><Plus size={12} /></button
        >
        {#if zoom > 1}
          <button use:tip={"Fit the whole map"} aria-label="Fit the whole map" onclick={reset}><Scan size={12} /></button>
        {/if}
      </div>
    {/if}
  </div>
  <p class="legend">
    {#if counts.portals}<span><i class="key portal"></i>Portals</span>{/if}
    {#if counts.hidden}<span><i class="key portal hidden"></i>Hidden</span>{/if}
    {#if counts.teleports}<span><i class="key teleport"></i>Teleports</span>{/if}
    {#if counts.npcs}<span><i class="key npc"></i>NPCs</span>{/if}
    {#if counts.spawns}<span><i class="key mob"></i>Spawns</span>{/if}
    <span class="hint">{zoom > 1 ? "Drag to move · double-click to fit" : "Scroll to zoom · click a mark for its card"}</span>
  </p>
{:else if !done}
  <p class="note"><span class="spin"><LoaderCircle size={13} /></span> Getting the map picture…</p>
{:else}
  <p class="note">No picture of this map. Game pictures may be off in Settings → Data.</p>
{/if}

<style>
  .stage {
    position: relative;
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    container-type: inline-size;
  }

  /* The picture is dimmed on the window's own dark, so the marks stand out on it. */
  .stage {
    background: var(--bg);
  }

  .stage.zoomed {
    cursor: grab;
    touch-action: none;
  }

  .stage.dragging {
    cursor: grabbing;
  }

  .stage.dragging .mark {
    pointer-events: none;
  }

  .layer {
    position: absolute;
  }

  img {
    display: block;
    width: 100%;
    height: 100%;
    opacity: 0;
    filter: saturate(0.8) brightness(0.72);
    transition: opacity 0.2s ease;
    user-select: none;
    pointer-events: none;
  }

  img.ready {
    opacity: 1;
  }

  /* Zoom buttons in the top corner: quiet until the pointer is on the picture or it is zoomed. */
  .zoom {
    position: absolute;
    top: 6px;
    right: 6px;
    z-index: 6;
    display: flex;
    align-items: center;
    gap: 1px;
    padding: 2px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--bg) 82%, transparent);
    opacity: 0;
    transition: opacity 0.14s ease;
    cursor: default;
  }

  .stage:hover .zoom,
  .stage.zoomed .zoom,
  .zoom:focus-within {
    opacity: 1;
  }

  .zoom button {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    padding: 0;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--muted);
    cursor: pointer;
  }

  .zoom button:hover:not(:disabled),
  .zoom button:focus-visible {
    background: var(--hover);
    color: var(--text);
    outline: none;
  }

  .zoom button:disabled {
    opacity: 0.35;
    cursor: default;
  }

  .level {
    min-width: 28px;
    color: var(--muted);
    font-size: var(--text-xs);
    text-align: center;
  }

  /*
   * Each mark is a 24 px target centred on its point, the same size however wide the panel or
   * far in the zoom, with its shape drawn inside. A dark outline keeps every shape clear of the
   * picture under it.
   */
  .mark {
    position: absolute;
    z-index: 1;
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    padding: 0;
    border: 0;
    border-radius: 50%;
    background: transparent;
    cursor: pointer;
    transform: translate(-50%, -50%);
  }

  .mark.unknown {
    cursor: default;
  }

  .mark:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  .shape {
    display: block;
    box-sizing: content-box;
    transition:
      transform 0.14s ease,
      opacity 0.14s ease,
      box-shadow 0.14s ease;
  }

  /* Spawns: red-orange dots. Pointing at a monster in the list lights its own and dims the rest. */
  .mob .shape {
    width: 12px;
    height: 12px;
    border: 2px solid var(--bg);
    border-radius: 50%;
    background: var(--marker-mob);
  }

  .mob.dim {
    z-index: 0;
  }

  .mob.dim .shape {
    opacity: 0.65;
    transform: scale(0.67);
  }

  .mob.lit {
    z-index: 3;
  }

  .mob.lit .shape,
  .mob:hover .shape {
    box-shadow: 0 0 0 6px color-mix(in srgb, var(--marker-mob) 35%, transparent);
  }

  /* A monster's own picture on its spawn point, outlined so it stands off the map behind it. */
  .mob :global(.mob-pic) {
    max-width: none;
    filter: drop-shadow(0 0 1px rgba(0, 0, 0, 0.9)) drop-shadow(0 1px 2px rgba(0, 0, 0, 0.6));
    transition:
      transform 0.14s ease,
      opacity 0.14s ease,
      filter 0.14s ease;
    pointer-events: none;
  }

  .mob.dim :global(.mob-pic) {
    opacity: 0.45;
    transform: scale(0.8);
  }

  .mob.lit :global(.mob-pic),
  .mob:hover :global(.mob-pic) {
    transform: scale(1.25);
    filter: drop-shadow(0 0 1px rgba(0, 0, 0, 0.9)) drop-shadow(0 0 6px var(--marker-mob));
  }

  /* NPCs: green rounded squares. */
  .npc {
    z-index: 2;
  }

  .npc .shape {
    width: 14px;
    height: 14px;
    border: 2px solid var(--bg);
    border-radius: 4px;
    background: var(--marker-npc);
  }

  .npc:hover .shape {
    transform: scale(1.15);
  }

  /* Portals out: a blue ring with a blue dot inside, on a dark disc; hidden ones have a dashed ring. */
  .portal {
    z-index: 3;
  }

  .portal .shape {
    position: relative;
    width: 14px;
    height: 14px;
    border: 3px solid var(--marker-portal);
    border-radius: 50%;
    background: radial-gradient(circle, var(--marker-portal) 0 4px, var(--bg) 4.5px);
    box-shadow: 0 0 0 2px var(--bg);
  }

  .portal.hidden .shape {
    border-style: dashed;
  }

  .portal:hover .shape {
    transform: scale(1.15);
  }

  /* A portal pointed at in the Portals section. */
  .portal.lit {
    z-index: 5;
  }

  .portal.lit .shape {
    transform: scale(1.25);
    box-shadow:
      0 0 0 2px var(--bg),
      0 0 0 7px color-mix(in srgb, var(--marker-portal) 40%, transparent);
  }

  /* The trip's next portal: amber, with a small "Next" flag above it. */
  .portal.next {
    z-index: 4;
  }

  .portal.next .shape {
    border-color: var(--accent-strong);
    background: radial-gradient(circle, var(--accent-strong) 0 4px, var(--bg) 4.5px);
    box-shadow:
      0 0 0 2px var(--bg),
      0 0 12px color-mix(in srgb, var(--accent) 70%, transparent);
  }

  .flag {
    position: absolute;
    bottom: calc(100% - 1px);
    left: 50%;
    padding: 0 4px;
    border-radius: var(--radius-sm);
    background: var(--accent-strong);
    color: var(--accent-ink);
    font-size: var(--text-2xs);
    font-weight: var(--weight-strong);
    line-height: 14px;
    white-space: nowrap;
    transform: translateX(-50%);
    pointer-events: none;
  }

  /* Teleports within the map: small and quiet, a dashed ring and no dot. */
  .portal.teleport {
    z-index: 0;
  }

  .portal.teleport .shape {
    width: 8px;
    height: 8px;
    border-width: 2px;
    border-style: dashed;
    background: color-mix(in srgb, var(--bg) 60%, transparent);
    box-shadow: none;
    opacity: 0.75;
  }

  .portal.teleport.paired {
    z-index: 4;
  }

  .portal.teleport.paired .shape {
    opacity: 1;
    border-style: solid;
    box-shadow: 0 0 0 5px color-mix(in srgb, var(--marker-portal) 35%, transparent);
    transform: scale(1.3);
  }

  .legend {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px 12px;
    margin: 6px 0 0;
    color: var(--faint);
    font-size: var(--text-xs);
  }

  .legend span {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }

  .hint {
    margin-left: auto;
  }

  .key {
    display: inline-block;
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }

  .key.mob {
    background: var(--marker-mob);
  }

  .key.npc {
    border-radius: 2px;
    background: var(--marker-npc);
  }

  .key.portal {
    border: 2px solid var(--marker-portal);
    background: radial-gradient(circle, var(--marker-portal) 0 1.5px, transparent 2px);
  }

  .key.portal.hidden {
    border-style: dashed;
  }

  .key.teleport {
    width: 6px;
    height: 6px;
    border: 1.5px dashed var(--marker-portal);
  }

  .note {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0;
    color: var(--faint);
    font-size: var(--text-md);
  }

  .spin {
    display: inline-grid;
    animation: spin 0.9s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
