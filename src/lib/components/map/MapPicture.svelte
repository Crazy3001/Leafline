<script lang="ts">
  // A map's picture, cut to what is drawn and dimmed, with its portals, NPCs and spawn points
  // marked where they stand, at the same size however wide the panel. Each mark opens its card;
  // a teleport within the map points out where it comes out instead. The picture is downloaded
  // the first time it is shown.
  import { LoaderCircle } from "@lucide/svelte";

  import { backend } from "$lib/api";
  import { openPeek } from "$lib/peek.svelte";
  import { pictureUrl } from "$lib/pictures";
  import { formatNumber } from "$lib/format";
  import { tip } from "$lib/tip.svelte";
  import type { MapPicture, MobRow, ViewPoint } from "$lib/types";
  import Pic from "$lib/ui/Pic.svelte";

  let {
    mapId,
    highlight = null,
    mobs = [],
    npcs = [],
  }: {
    mapId: number;
    /** The map's monsters, for their levels in a spawn point's tooltip. */
    mobs?: Pick<MobRow, "id" | "name" | "level" | "hp" | "exp" | "boss">[];
    /** The map's NPCs, for how many quests each gives. */
    npcs?: { id: number; quests: unknown[] }[];
    /** A monster whose spawn points stand out, such as the one under the pointer in the list. */
    highlight?: string | null;
  } = $props();

  let picture = $state<MapPicture | null>(null);
  let done = $state(false);
  let failed = $state(false);
  let loaded = $state(false);
  /** The teleport whose exit is pointed out, by the clicked one's index. */
  let paired = $state<number | null>(null);

  $effect(() => {
    const id = mapId;
    picture = null;
    done = false;
    failed = false;
    loaded = false;
    paired = null;
    backend
      .mapPicture(id)
      .then((result) => {
        if (id === mapId) picture = result;
      })
      .catch(() => (picture = null))
      .finally(() => (done = true));
  });

  const src = $derived(picture ? pictureUrl("mapview", mapId) : null);
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
          class:teleport={point.teleport}
          class:hidden={point.hidden && !point.teleport}
          class:lit={point.kind === "mob" && highlight != null && point.name === highlight}
          class:dim={point.kind === "mob" && highlight != null && point.name !== highlight}
          class:paired={pairedEnds.has(i)}
          class:unknown={!point.known && !point.teleport}
          style:left="{point.x * 100}%"
          style:top="{point.y * 100}%"
          aria-label={label(point)}
          use:tip={markTip}
          onclick={(e) => open(i, e.currentTarget)}
        >
          <span class="shape"></span>
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
                  >{point.hidden ? "Hidden portal" : "Portal"}{point.known ? " · click for the map's card" : " · not in the map data"}</span
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
                <span><b>Lv</b> {mob.level}</span><span><b>HP</b> {formatNumber(mob.hp)}</span><span><b>EXP</b> {formatNumber(mob.exp)}</span>
              </span>
            {/if}
          {/if}
        {/snippet}
      {/each}
    {/if}
  </div>
  <p class="legend">
    {#if counts.portals}<span><i class="key portal"></i>Portals</span>{/if}
    {#if counts.hidden}<span><i class="key portal hidden"></i>Hidden</span>{/if}
    {#if counts.teleports}<span><i class="key teleport"></i>Teleports</span>{/if}
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
    container-type: inline-size;
  }

  /* The picture is dimmed on the window's own dark, so the marks stand out on it. */
  .stage {
    background: var(--bg);
  }

  img {
    display: block;
    width: 100%;
    height: 100%;
    opacity: 0;
    filter: saturate(0.8) brightness(0.72);
    transition: opacity 0.2s ease;
    user-select: none;
  }

  img.ready {
    opacity: 1;
  }

  /*
   * Each mark is a 24 px target centred on its point, the same size however wide the panel, with
   * its shape drawn inside. A dark outline keeps every shape clear of the picture under it.
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
