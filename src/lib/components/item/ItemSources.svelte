<script lang="ts">
  // Where an item comes from, by how you get it: the NPC shops that sell it, the monsters that
  // drop it, each opening to the maps it lives on, and the quests that give it.
  import { tip } from "$lib/tip.svelte";
  import Ref from "$lib/components/Ref.svelte";
  import { ChevronRight } from "@lucide/svelte";

  import GoButton from "$lib/components/GoButton.svelte";
  import { backend } from "$lib/api";
  import { mapPlace } from "$lib/destination.svelte";
  import { formatNumber, groundDistance } from "$lib/format";
  import { seenLine, shortDate } from "$lib/shops";
  import { showMonsterNamed } from "$lib/monsters.svelte";
  import { showNpcNamed } from "$lib/places.svelte";
  import type { Hunt, ItemWhere } from "$lib/types";

  let { found }: { found: ItemWhere } = $props();

  const drops = $derived(found.origin.drops);
  const quests = $derived(found.origin.quests);
  const shops = $derived(found.origin.shops ?? []);
  const shopsSeen = $derived(seenLine(shops.map((s) => s.seen)));
  let shopsOpen = $state(true);
  const known = $derived(drops.hunts.filter((h) => h.monster.known));
  const unknown = $derived(drops.hunts.filter((h) => !h.monster.known));

  let dropsOpen = $state(true);
  let questsOpen = $state(true);
  /** Monsters opened to their maps. */
  let opened = $state<string[]>([]);
  const isOpen = (hunt: Hunt) => opened.includes(hunt.monster.name);

  function toggle(hunt: Hunt) {
    const name = hunt.monster.name;
    opened = opened.includes(name) ? opened.filter((m) => m !== name) : [...opened, name];
  }

  function rate(hunt: Hunt) {
    const value = hunt.drop?.rate;
    if (value == null) return null;
    return `${value >= 10 ? value.toFixed(0) : value.toFixed(1)}%`;
  }

  /** How solid a rate is. */
  function evidence(hunt: Hunt) {
    const drop = hunt.drop;
    if (!drop) return undefined;
    return [
      drop.rateLow != null && drop.rateHigh != null ? `Likely ${drop.rateLow.toFixed(1)}–${drop.rateHigh.toFixed(1)}%` : null,
      drop.sampleKills ? `${drop.sampleKills.toLocaleString()} kills logged` : null,
    ]
      .filter(Boolean)
      .join(" · ");
  }
</script>

{#snippet head(title: string, count: number, open: boolean, onclick: () => void)}
  <button class="group-head" aria-expanded={open} {onclick}>
    <span class="chevron" class:open><ChevronRight size={12} /></span>
    <span class="group-title">{title}</span>
    <span class="count tabular">{count}</span>
  </button>
{/snippet}

{#if shops.length}
  <div class="group">
    {@render head("Sold by", shops.length, shopsOpen, () => (shopsOpen = !shopsOpen))}
    {#if shopsOpen}
      <ul class="quests">
        {#each shops as shop, i (i)}
          <li>
            <span class="shop-line">
              {#if shop.npcId != null}<Ref kind="npc" id={shop.npcId} name={shop.npc} />{:else}{shop.npc}{/if}
              <span class="price tabular" use:tip={`Seen ${shortDate(shop.seen)}`}>{formatNumber(shop.price)} mesos</span>
            </span>
            <span class="who">
              {#if shop.map}<Ref kind="map" id={shop.map!.id}>{shop.map.name}</Ref
                >{:else}{shop.mapName}{/if}{#if shop.hops != null && shop.hops > 0}{" · "}{shop.hops} portal{shop.hops === 1
                  ? ""
                  : "s"}{/if}{#if shop.needs}{" · "}<span class="needs">{shop.needs}</span>{/if}
              {#if shop.map && shop.hops !== 0}<GoButton place={mapPlace(shop.map)} />{/if}
            </span>
          </li>
        {/each}
      </ul>
      {#if shopsSeen}<p class="seen">{shopsSeen}</p>{/if}
    {/if}
  </div>
{/if}

{#if known.length || unknown.length}
  <div class="group">
    {@render head("Dropped by", drops.hunts.length, dropsOpen, () => (dropsOpen = !dropsOpen))}
    {#if dropsOpen}
      <ul class="monsters">
        {#each known as hunt (hunt.monster.name)}
          {@const open = isOpen(hunt)}
          <li>
            <!-- The name opens the monster in Lookup; the rest of the row opens its maps. -->
            <div class="mob-head">
              <button class="chevron-button" aria-expanded={open} aria-label="Maps for {hunt.monster.name}" onclick={() => toggle(hunt)}>
                <span class="chevron" class:open><ChevronRight size={11} /></span>
              </button>
              <Ref kind="mob" name={hunt.monster.name} class="mob-name" />
              <button class="row-rest" tabindex="-1" onclick={() => toggle(hunt)}>
                <span class="muted tabular">Lv {hunt.monster.level}{hunt.monster.boss ? " · boss" : ""}</span>
                <span class="maps-count faint tabular">{hunt.maps.length} map{hunt.maps.length === 1 ? "" : "s"}</span>
                {#if rate(hunt)}<span class="rate tabular" use:tip={evidence(hunt)}>{rate(hunt)}</span>{/if}
              </button>
            </div>
            {#if open}
              <ul class="maps">
                {#each hunt.maps as ground (ground.map.id)}
                  <li>
                    <Ref kind="map" id={ground.map.id} name={ground.map.name} />
                    <span class="faint tabular">×{ground.count}</span>
                    <span class="dist tabular">{groundDistance(ground, found.from != null)}</span>
                    {#if ground.hops !== 0}<GoButton place={mapPlace(ground.map)} />{/if}
                  </li>
                {:else}
                  <li class="faint">No map in the data spawns it.</li>
                {/each}
              </ul>
            {/if}
          </li>
        {/each}
        {#each unknown as hunt (hunt.monster.name)}
          <li class="mob-head missing">
            <span class="chevron"></span>
            <span class="mob-name">{hunt.monster.name}</span>
            <span class="faint">not in the map data</span>
            {#if rate(hunt)}<span class="rate tabular" use:tip={evidence(hunt)}>{rate(hunt)}</span>{/if}
          </li>
        {/each}
      </ul>
    {/if}
  </div>
{:else if drops.lookedUp}
  <p class="note">No known monster drops for this item.</p>
{:else}
  <p class="note">The drop table isn't downloaded yet.</p>
{/if}

{#if quests.length}
  <div class="group">
    {@render head("Quest reward", quests.length, questsOpen, () => (questsOpen = !questsOpen))}
    {#if questsOpen}
      <ul class="quests">
        {#each quests as quest (quest.quest.id)}
          {@const giverMap = quest.giverMaps[0]}
          <li>
            <Ref kind="quest" id={quest.quest.id} name={quest.quest.name} class="quest" />
            <span class="who">
              {#if quest.quest.levelMin}Lv {quest.quest.levelMin}{/if}{#if quest.giver}{quest.quest.levelMin ? " · " : ""}<Ref kind="npc" name={quest.giver} />{/if}{#if giverMap}{quest.quest.levelMin || quest.giver ? " · " : ""}{#if giverMap.known}<Ref kind="map" id={giverMap.id} name={giverMap.name} />{:else}{giverMap.name}{/if}{/if}
            </span>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
{/if}

<style>
  .group + .group {
    margin-top: 6px;
  }

  .shop-line {
    display: flex;
    align-items: baseline;
    gap: 6px;
  }

  .shop-line .price {
    margin-left: auto;
  }

  .needs {
    color: var(--warn);
  }

  .seen {
    margin: 4px 0 0;
    font-size: var(--text-xs);
    color: var(--faint);
  }

  .group-head,
  .mob-head {
    display: flex;
    align-items: center;
    gap: 5px;
    width: 100%;
    padding: 4px 0;
    border: 0;
    background: transparent;
    text-align: left;
    cursor: pointer;
  }

  .group-head {
    border-bottom: 1px solid var(--border);
  }

  .group-title {
    color: var(--muted);
    font-size: var(--text-xs);
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .count {
    color: var(--faint);
    font-size: var(--text-xs);
  }

  .chevron {
    display: grid;
    flex: none;
    width: 12px;
    color: var(--faint);
    transition: transform 0.15s ease;
  }

  .chevron.open {
    transform: rotate(90deg);
  }

  .monsters,
  .maps,
  .quests {
    display: grid;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .monsters > li + li {
    border-top: 1px solid var(--border);
  }

  .mob-head.missing {
    cursor: default;
  }

  .chevron-button,
  .row-rest {
    display: flex;
    align-items: center;
    padding: 0;
    border: 0;
    background: transparent;
    cursor: pointer;
  }

  .row-rest {
    flex: 1;
    gap: 5px;
    align-self: stretch;
  }

  .maps-count {
    margin-left: auto;
  }

  .maps-count + .rate {
    margin-left: 8px;
  }

  .mob-name {
    font-size: var(--text-sm);
    font-weight: 600;
  }

  .missing .mob-name {
    color: var(--faint);
    font-weight: 400;
  }

  .muted {
    color: var(--muted);
    font-size: var(--text-xs);
  }

  .faint {
    color: var(--faint);
    font-size: var(--text-xs);
  }

  .rate {
    margin-left: auto;
    color: var(--text);
    font-size: var(--text-xs);
    font-weight: 600;
  }

  .maps {
    gap: 3px;
    padding: 0 0 7px 17px;
  }

  .maps > li {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: var(--text-sm);
  }

  .dist {
    margin-left: auto;
    color: var(--muted);
    font-size: var(--text-xs);
  }

  .quests {
    gap: 6px;
    padding: 6px 0 2px;
  }

  .quests li {
    display: grid;
    gap: 1px;
  }

  .who {
    color: var(--muted);
    font-size: var(--text-xs);
  }

  .note {
    margin: 0;
    color: var(--faint);
    font-size: var(--text-sm);
  }
</style>
