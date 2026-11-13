<script lang="ts">
  // A map's picture, cut to what is drawn, with its portals, NPCs and spawn points marked where
  // they stand. Each mark opens its card. The picture is downloaded the first time it is shown.
  import { EyeOff, LoaderCircle } from "@lucide/svelte";

  import { backend } from "$lib/api";
  import { openPeek } from "$lib/peek.svelte";
  import { pictureUrl } from "$lib/pictures";
  import type { MapPicture, ViewPoint } from "$lib/types";

  let {
    mapId,
    highlight = null,
  }: {
    mapId: number;
    /** A monster whose spawn points stand out, such as the one under the pointer in the list. */
    highlight?: string | null;
  } = $props();

  let picture = $state<MapPicture | null>(null);
  let done = $state(false);
  let failed = $state(false);
  let loaded = $state(false);

  $effect(() => {
    const id = mapId;
    picture = null;
    done = false;
    failed = false;
    loaded = false;
    backend
      .mapPicture(id)
      .then((result) => {
        if (id === mapId) picture = result;
      })
      .catch(() => (picture = null))
      .finally(() => (done = true));
  });

  const src = $derived(picture ? pictureUrl("mapview", mapId) : null);
  // Spawns first, so portals and NPCs sit on top of them.
  const marks = $derived(
    picture ? [...picture.points].sort((a, b) => Number(b.kind === "mob") - Number(a.kind === "mob")) : [],
  );
  const counts = $derived({
    portals: marks.filter((p) => p.kind === "portal").length,
    npcs: marks.filter((p) => p.kind === "npc").length,
    spawns: marks.filter((p) => p.kind === "mob").length,
  });

  function open(point: ViewPoint, target: Element) {
    if (!point.known) return;
    const kind = point.kind === "portal" ? "map" : point.kind;
    openPeek({ kind, id: point.id, name: point.name }, target);
  }

  function label(point: ViewPoint) {
    if (point.kind === "portal") return `${point.hidden ? "Hidden portal" : "Portal"} to ${point.name}`;
    return point.kind === "npc" ? point.name : `${point.name} spawn`;
  }
</script>

{#if picture && src && !failed}
  <div class="stage" style:aspect-ratio="{picture.width} / {picture.height}">
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
        <button
          class="mark {point.kind}"
          class:hidden={point.hidden}
          class:lit={point.kind === "mob" && highlight != null && point.name === highlight}
          class:dim={point.kind === "mob" && highlight != null && point.name !== highlight}
          class:unknown={!point.known}
          class:at-left={point.kind === "portal" && point.x < 0.12}
          class:at-right={point.kind === "portal" && point.x > 0.88}
          style:left="{point.x * 100}%"
          style:top="{point.y * 100}%"
          aria-label={label(point)}
          onclick={(e) => open(point, e.currentTarget)}
        >
          {#if point.kind === "portal"}{#if point.hidden}<EyeOff size={9} />{/if}<span class="to">{point.name}</span>{/if}
        </button>
      {/each}
    {/if}
  </div>
  <p class="legend">
    {#if counts.portals}<span><i class="key portal"></i>Portals</span>{/if}
    {#if marks.some((p) => p.hidden)}<span><i class="key portal hidden"></i>Hidden</span>{/if}
    {#if counts.npcs}<span><i class="key npc"></i>NPCs</span>{/if}
    {#if counts.spawns}<span><i class="key mob"></i>Spawns</span>{/if}
    <span class="hint">Click a mark for its card</span>
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
    background: #000;
    container-type: inline-size;
  }

  img {
    display: block;
    width: 100%;
    height: 100%;
    opacity: 0;
    transition: opacity 0.2s ease;
    user-select: none;
  }

  img.ready {
    opacity: 1;
  }

  .mark {
    position: absolute;
    z-index: 1;
    display: inline-flex;
    align-items: center;
    gap: 3px;
    padding: 0;
    border: 0;
    cursor: pointer;
    transform: translate(-50%, -50%);
    transition:
      transform 0.12s ease,
      opacity 0.12s ease,
      box-shadow 0.12s ease;
  }

  .mark.unknown {
    cursor: default;
  }

  .mark:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  /* Spawn points: small coral dots, brightened for the monster asked about, faded for the rest. */
  .mob {
    width: 7px;
    height: 7px;
    border: 1px solid rgba(0, 0, 0, 0.6);
    border-radius: 50%;
    background: var(--ref-mob);
    opacity: 0.85;
  }

  .mob:hover,
  .mob.lit {
    z-index: 3;
    transform: translate(-50%, -50%) scale(1.6);
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--ref-mob) 45%, transparent), 0 0 10px var(--ref-mob);
    opacity: 1;
  }

  .mob.dim {
    opacity: 0.25;
  }

  /* NPCs: a teal pin with a light ring. */
  .npc {
    z-index: 2;
    width: 11px;
    height: 11px;
    border: 2px solid #fff;
    border-radius: 50%;
    background: var(--ref-npc);
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.6);
  }

  .npc:hover {
    transform: translate(-50%, -50%) scale(1.25);
  }

  /* Portals: an amber tag naming where they lead; hidden ones dashed, with the eye. */
  .portal {
    z-index: 2;
    max-width: 30cqi;
    height: 16px;
    padding: 0 5px;
    border: 1px solid var(--map);
    border-radius: var(--radius-pill);
    background: rgba(20, 16, 8, 0.85);
    color: var(--map);
    font-size: 9px;
    font-weight: 600;
    line-height: 1;
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.6);
  }

  /* Near an edge a tag grows inward, so its name isn't cut off. */
  .portal.at-left {
    transform: translate(-6px, -50%);
  }

  .portal.at-right {
    transform: translate(calc(-100% + 6px), -50%);
  }

  .portal.hidden {
    border-style: dashed;
    color: var(--warn);
    border-color: var(--warn);
  }

  .portal:hover {
    z-index: 4;
    max-width: none;
    background: rgba(20, 16, 8, 0.95);
  }

  .to {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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
    background: var(--ref-mob);
  }

  .key.npc {
    border: 1.5px solid #fff;
    background: var(--ref-npc);
  }

  .key.portal {
    width: 12px;
    border: 1px solid var(--map);
    border-radius: var(--radius-pill);
  }

  .key.portal.hidden {
    border: 1px dashed var(--warn);
  }

  .note {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0;
    color: var(--faint);
    font-size: var(--text-sm);
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
