<script lang="ts">
  // Where to go for one quest objective: a one-line hint that opens into the monsters to hunt,
  // their best maps, and the route to the chosen map. An item an NPC sells says so, with the
  // cheapest shop, so it can be bought instead of farmed.
  import { tip } from "$lib/tip.svelte";
  import Ref from "$lib/components/Ref.svelte";
  import { ChevronRight, Crosshair, ShoppingBag } from "@lucide/svelte";

  import { backend } from "$lib/api";
  import { formatMesos, groundDistance } from "$lib/format";
  import { huntOrder, orderGrounds, setHuntOrder } from "$lib/quests.svelte";
  import { travelKey } from "$lib/settings.svelte";
  import RouteSteps from "$lib/components/map/RouteSteps.svelte";
  import GoButton from "$lib/components/GoButton.svelte";
  import { mapPlace } from "$lib/destination.svelte";
  import type { HuntingGround, MapRef, MapRoute, ObjectiveGuide } from "$lib/types";

  let {
    guide,
    from,
    left = null,
  }: {
    guide: ObjectiveGuide;
    from: MapRef | null;
    /** How many are still wanted, to price buying them all. */
    left?: number | null;
  } = $props();

  let open = $state(false);
  /** The map whose route is shown: the best one until another is picked. */
  let picked = $state<number | null>(null);
  let route = $state<MapRoute | null>(null);
  let routing = $state(false);

  const hunts = $derived(
    guide.hunts
      .filter((h) => h.monster.known && h.maps.length)
      .map((h) => ({ ...h, maps: orderGrounds(h.maps, huntOrder.value) })),
  );
  /** Whether the choice says anything: some monster has a map that is closer but not fuller. */
  const choosing = $derived(
    hunts.some((h) => h.maps.length > 1 && orderGrounds(h.maps, "most")[0].map.id !== h.maps[0].map.id) ||
      (huntOrder.value === "most" && hunts.some((h) => h.maps.length > 1)),
  );
  const shop = $derived(guide.shops?.[0] ?? null);
  const top = $derived(hunts[0] ?? null);
  const best = $derived(top?.maps[0] ?? null);
  const target = $derived(picked ?? best?.map.id ?? null);
  // Each guide brings a new `from`: the route follows the map, not the object.
  const fromId = $derived(from?.id);

  $effect(() => {
    const to = target;
    const start = fromId;
    void travelKey();
    route = null;
    if (!open || to == null || start == null) return;
    routing = true;
    backend
      .mapRoute(start, to)
      .then((r) => {
        if (target === to && fromId === start) route = r;
      })
      .finally(() => (routing = false));
  });

  const distance = (ground: HuntingGround) => groundDistance(ground, from != null);
</script>

{#snippet buy()}
  {#if shop}
    <div class="hint-row">
      <p class="hint quiet buy">
        <span class="icon shop"><ShoppingBag size={11} /></span>
        <span class="line">
          Or buy it from <Ref kind="npc" name={shop.npc} />
          <span class="sep">·</span>
          <span class="tabular mesos">{formatMesos(shop.price)} mesos</span>{#if left && left > 1}<span class="muted tabular"
              >&nbsp;each, {formatMesos(shop.price * left)} for {left}</span
            >{/if}
          {#if shop.needs}<span class="muted"> · {shop.needs}</span>{/if}
        </span>
        {#if shop.map && shop.hops != null}<span class="far tabular">{shop.hops === 0 ? "here" : `${shop.hops} portal${shop.hops === 1 ? "" : "s"}`}</span>{/if}
      </p>
      {#if shop.map && shop.hops !== 0}<GoButton place={mapPlace(shop.map)} />{/if}
    </div>
  {/if}
{/snippet}

{#if top && best}
  <div class="hint-row">
    <button class="hint" aria-expanded={open} onclick={() => (open = !open)}>
      <span class="icon"><Crosshair size={11} /></span>
      <span class="line">
        {#if guide.kind === "item"}
          <span class="who">{top.monster.name}</span>

          <span class="sep">·</span>
        {/if}
        <span class="where">{best.map.name}</span>
      </span>
      {#if best.travel}
        <span class="far tabular shortcut" use:tip={`${distance(best)}, via ${best.travel.via.name}`}>{best.travel.hops} with travel</span>
      {:else if best.hops != null}
        <span class="far tabular">{distance(best)}</span>
      {/if}
      <span class="chevron" class:open><ChevronRight size={11} /></span>
    </button>
    {#if best.hops !== 0}<GoButton place={mapPlace(best.map)} />{/if}
  </div>

  {@render buy()}

  {#if open}
    <div class="detail">
      {#if choosing}
        <div class="order" role="group" aria-label="Which map first">
          <button aria-pressed={huntOrder.value === "closest"} onclick={() => { picked = null; setHuntOrder("closest"); }} use:tip={"The nearest map with a fair number of them"}>Closest</button>
          <button aria-pressed={huntOrder.value === "most"} onclick={() => { picked = null; setHuntOrder("most"); }} use:tip={"The map with the most of them, however far"}>Most</button>
        </div>
      {/if}
      {#each hunts as hunt (hunt.monster.name)}
        <div class="hunt">
          <div class="mob">
            <Ref kind="mob" name={hunt.monster.name} />
            <span class="muted">Lv {hunt.monster.level}{hunt.monster.boss ? " · boss" : ""}</span>
          </div>
          <ul class="maps">
            {#each hunt.maps as ground (ground.map.id)}
              <li class="ground-row">
                <button
                  class="ground"
                  class:selected={ground.map.id === target}
                  aria-pressed={ground.map.id === target}
                  onclick={() => (picked = ground.map.id)}
                >
                  <span class="map">{ground.map.name}</span>
                  <span class="count tabular">×{ground.count} spawn</span>
                  <span
                    class="dist tabular"
                    class:shortcut={ground.travel}
                    use:tip={ground.travel ? `With the travel you turned on in Settings, via ${ground.travel.via.name}` : undefined}
                    >{distance(ground)}</span
                  >
                </button>
                {#if ground.hops !== 0}<GoButton place={mapPlace(ground.map)} />{/if}
              </li>
            {/each}
          </ul>
        </div>
      {/each}
      {#each guide.hunts.filter((h) => !h.monster.known) as hunt (hunt.monster.name)}
        <p class="note">
          {hunt.monster.name} also drops
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
  {@render buy()}
{:else if guide.kind === "item" && shop}
  {@render buy()}
{:else if guide.kind === "item" && guide.lookedUp}
  <p class="hint quiet">No known monster drops for this item.</p>
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
    font-size: var(--text-sm);
    text-align: left;
    cursor: pointer;
  }

  .hint-row {
    display: flex;
    align-items: center;
    gap: 4px;
    margin: 5px 0 0;
  }

  .hint-row .hint {
    flex: 1;
    min-width: 0;
    margin: 0;
  }

  .ground-row {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .ground-row .ground {
    flex: 1;
    min-width: 0;
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
    font-size: var(--text-xs);
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
    font-size: var(--text-sm);
  }

  .mob :global(.ref) {
    font-weight: 600;
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
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--muted);
    font-size: var(--text-sm);
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

  .far.shortcut,
  .dist.shortcut {
    color: var(--travel);
  }

  .route {
    margin-top: 2px;
  }

  .buy {
    margin: 0;
  }

  .icon.shop {
    color: var(--mesos);
  }

  .mesos {
    color: var(--mesos);
  }

  .order {
    display: inline-flex;
    gap: 2px;
    margin: 0 0 6px;
    padding: 2px;
    border-radius: var(--radius-sm);
    background: rgba(255, 255, 255, 0.04);
  }

  .order button {
    padding: 2px 8px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--muted);
    font-size: var(--text-xs);
    cursor: pointer;
  }

  .order button[aria-pressed="true"] {
    background: color-mix(in srgb, var(--map) 18%, transparent);
    color: var(--text);
  }

  .order button:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -1px;
  }

  .note {
    margin: 6px 0 0;
    font-size: var(--text-xs);
    color: var(--faint);
  }
</style>
