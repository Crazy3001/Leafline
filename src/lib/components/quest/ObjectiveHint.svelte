<script lang="ts">
  // Where to go for one quest objective: a one-line hint that opens into the monsters to hunt,
  // their best maps, and the route to the chosen map.
  import { ChevronRight, Crosshair } from "@lucide/svelte";

  import { backend } from "$lib/api";
  import RouteSteps from "$lib/components/map/RouteSteps.svelte";
  import type { HuntingGround, MapRef, MapRoute, ObjectiveGuide } from "$lib/types";

  let { guide, from }: { guide: ObjectiveGuide; from: MapRef | null } = $props();

  let open = $state(false);
  /** The map whose route is shown: the best one until another is picked. */
  let picked = $state<number | null>(null);
  let route = $state<MapRoute | null>(null);
  let routing = $state(false);

  const hunts = $derived(guide.hunts.filter((h) => h.monster.known && h.maps.length));
  const top = $derived(hunts[0] ?? null);
  const best = $derived(top?.maps[0] ?? null);
  const target = $derived(picked ?? best?.map.id ?? null);

  $effect(() => {
    const to = target;
    const start = from?.id;
    route = null;
    if (!open || to == null || start == null) return;
    routing = true;
    backend
      .mapRoute(start, to)
      .then((r) => {
        if (target === to && from?.id === start) route = r;
      })
      .finally(() => (routing = false));
  });

  function rate(value: number | null) {
    if (value == null) return "rate not logged";
    return `${value >= 10 ? value.toFixed(0) : value.toFixed(1)}%`;
  }

  function rateTitle(drop: NonNullable<ObjectiveGuide["hunts"][number]["drop"]>) {
    const parts = ["Player-observed drop rate from mapleclassic.wiki"];
    if (drop.sampleKills) parts.push(`over ${drop.sampleKills.toLocaleString()} kills`);
    if (drop.gameVersion) parts.push(`(v${drop.gameVersion})`);
    if (drop.rateLow != null && drop.rateHigh != null) parts.push(`· likely ${drop.rateLow.toFixed(1)}–${drop.rateHigh.toFixed(1)}%`);
    return parts.join(" ");
  }

  function distance(ground: HuntingGround) {
    if (ground.hops == null) return from ? "no portal route" : "";
    if (ground.hops === 0) return "you're here";
    return `${ground.hops} portal${ground.hops === 1 ? "" : "s"}`;
  }
</script>

{#if top && best}
  <button class="hint" aria-expanded={open} onclick={() => (open = !open)}>
    <span class="icon"><Crosshair size={11} /></span>
    <span class="line">
      {#if guide.kind === "item"}
        <span class="who">{top.monster.name}</span>
        {#if top.drop}<span class="muted" title={rateTitle(top.drop)}>{rate(top.drop.rate)}</span>{/if}
        <span class="sep">·</span>
      {/if}
      <span class="where">{best.map.name}</span>
    </span>
    {#if best.hops != null}<span class="far tabular">{distance(best)}</span>{/if}
    <span class="chevron" class:open><ChevronRight size={11} /></span>
  </button>

  {#if open}
    <div class="detail">
      {#each hunts as hunt (hunt.monster.name)}
        <div class="hunt">
          <div class="mob">
            <span class="name">{hunt.monster.name}</span>
            <span class="muted">Lv {hunt.monster.level}{hunt.monster.boss ? " · boss" : ""}</span>
            {#if hunt.drop}
              <span class="rate" title={rateTitle(hunt.drop)}>{rate(hunt.drop.rate)} drop</span>
            {/if}
          </div>
          <ul class="maps">
            {#each hunt.maps as ground (ground.map.id)}
              <li>
                <button
                  class="ground"
                  class:selected={ground.map.id === target}
                  aria-pressed={ground.map.id === target}
                  onclick={() => (picked = ground.map.id)}
                >
                  <span class="map">{ground.map.name}</span>
                  <span class="count tabular" title="How many spawn on the map">×{ground.count}</span>
                  <span class="dist tabular">{distance(ground)}</span>
                </button>
              </li>
            {/each}
          </ul>
        </div>
      {/each}
      {#each guide.hunts.filter((h) => !h.monster.known) as hunt (hunt.monster.name)}
        <p class="note">
          {hunt.monster.name}{#if hunt.drop}<span title={rateTitle(hunt.drop)}> ({rate(hunt.drop.rate)})</span>{/if} also drops
          it, but isn't in the map data.
        </p>
      {/each}

      {#if !from}
        <p class="note">Routes appear when the minimap is in view.</p>
      {:else if route}
        <div class="route"><RouteSteps {route} /></div>
      {:else if !routing}
        <p class="note">No route through portals is known from {from.name}.</p>
      {/if}
    </div>
  {/if}
{:else if guide.kind === "item" && guide.lookedUp && guide.hunts.length}
  <p class="hint quiet">
    Dropped by {guide.hunts.map((h) => h.monster.name).join(", ")}, not found in the map data.
  </p>
{:else if guide.kind === "item" && guide.lookedUp}
  <p class="hint quiet">No monster drops logged on mapleclassic.wiki.</p>
{:else if guide.kind === "monster"}
  <p class="hint quiet">Not found spawning on any map in the data.</p>
{/if}

<style>
  .hint {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    margin: 5px 0 0;
    padding: 3px 4px 3px 2px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--muted);
    font-size: 11.5px;
    text-align: left;
    cursor: pointer;
  }

  button.hint:hover {
    background: rgba(255, 255, 255, 0.04);
    color: var(--text);
  }

  button.hint:focus-visible,
  .ground:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -1px;
  }

  .hint.quiet {
    cursor: default;
    color: var(--faint);
  }

  .icon {
    display: inline-grid;
    flex: none;
    color: var(--map);
  }

  .line {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .who,
  .where {
    color: var(--text);
  }

  .muted,
  .sep {
    color: var(--faint);
  }

  .far {
    flex: none;
    color: var(--faint);
    font-size: 11px;
  }

  .chevron {
    display: inline-grid;
    flex: none;
    color: var(--faint);
    transition: transform 0.12s ease;
  }

  .chevron.open {
    transform: rotate(90deg);
  }

  .detail {
    margin: 4px 0 2px 19px;
    padding-left: 8px;
    border-left: 1px solid var(--border);
  }

  .hunt + .hunt {
    margin-top: 8px;
  }

  .mob {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 6px;
    font-size: 12px;
  }

  .mob .name {
    font-weight: 600;
  }

  .rate {
    margin-left: auto;
    color: var(--accent-strong);
    font-size: 11px;
  }

  .maps {
    display: grid;
    gap: 1px;
    margin: 3px 0 0;
    padding: 0;
    list-style: none;
  }

  .ground {
    display: flex;
    align-items: baseline;
    gap: 8px;
    width: 100%;
    padding: 3px 6px;
    border: 0;
    border-radius: 5px;
    background: transparent;
    color: var(--muted);
    font-size: 11.5px;
    text-align: left;
    cursor: pointer;
  }

  .ground:hover {
    background: rgba(255, 255, 255, 0.04);
    color: var(--text);
  }

  .ground.selected {
    background: color-mix(in srgb, var(--map) 12%, transparent);
    color: var(--text);
  }

  .ground .map {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .count,
  .dist {
    flex: none;
    color: var(--faint);
  }

  .dist {
    min-width: 56px;
    text-align: right;
  }

  .route {
    margin-top: 2px;
  }

  .note {
    margin: 6px 0 0;
    font-size: 11px;
    color: var(--faint);
  }
</style>
