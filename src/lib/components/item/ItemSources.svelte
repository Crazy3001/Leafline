<script lang="ts">
  // Where an item comes from, by how you get it: the NPC shops that sell it, the monsters that
  // drop it, each opening to the maps it lives on, and the quests that give it.
  import Ref from "$lib/components/Ref.svelte";
  import { ChevronRight } from "@lucide/svelte";

  import GoButton from "$lib/components/GoButton.svelte";
  import { mapPlace } from "$lib/destination.svelte";
  import { formatNumber, groundDistance } from "$lib/format";
  import type { Hunt, ItemWhere } from "$lib/types";

  let { found }: { found: ItemWhere } = $props();

  const drops = $derived(found.origin.drops);
  const quests = $derived(found.origin.quests);
  const shops = $derived(found.origin.shops ?? []);
  // Every group starts folded, with its count, for each item opened.
  let shopsOpen = $state(false);
  const known = $derived(drops.hunts.filter((h) => h.monster.known));
  const unknown = $derived(drops.hunts.filter((h) => !h.monster.known));

  let dropsOpen = $state(false);
  let questsOpen = $state(false);
  /** Monsters opened to their maps. */
  let opened = $state<string[]>([]);

  $effect(() => {
    void found;
    shopsOpen = false;
    dropsOpen = false;
    questsOpen = false;
    opened = [];
  });
  const isOpen = (hunt: Hunt) => opened.includes(hunt.monster.name);

  function toggle(hunt: Hunt) {
    const name = hunt.monster.name;
    opened = opened.includes(name) ? opened.filter((m) => m !== name) : [...opened, name];
  }

</script>

{#snippet head(title: string, count: number, open: boolean, onclick: () => void)}
  <button class="group-head" aria-expanded={open} {onclick}>
    <span class="chevron" class:open><ChevronRight size={12} /></span>
    <span class="group-title">{title}</span>
    <span class="count tabular">{count}</span>
  </button>
{/snippet}

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

          </li>
        {/each}
      </ul>
    {/if}
  </div>
{:else}
  <div class="group">
    {@render head("Dropped by", 0, dropsOpen, () => (dropsOpen = !dropsOpen))}
    {#if dropsOpen}
      <p class="note">{drops.lookedUp ? "No monster is known to drop it." : "The drop table isn't downloaded yet."}</p>
    {/if}
  </div>
{/if}

{#if shops.length}
  <div class="group">
    {@render head("Sold by", shops.length, shopsOpen, () => (shopsOpen = !shopsOpen))}
    {#if shopsOpen}
      <ul class="quests">
        {#each shops as shop, i (i)}
          <li>
            <span class="shop-line">
              {#if shop.npcId != null}<Ref kind="npc" id={shop.npcId} name={shop.npc} />{:else}{shop.npc}{/if}
              <span class="price tabular">{formatNumber(shop.price)} mesos</span>
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
    {/if}
  </div>
{/if}

{#if quests.length}
  <div class="group">
    {@render head("From quests", quests.length, questsOpen, () => (questsOpen = !questsOpen))}
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
    margin-top: var(--space-1-5);
  }

  .shop-line {
    display: flex;
    align-items: baseline;
    gap: var(--space-1-5);
  }

  .shop-line .price {
    margin-left: auto;
  }

  .needs {
    color: var(--warn);
  }

  .group-head,
  .mob-head {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    width: 100%;
    padding: var(--space-1) 0;
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
    font-weight: var(--weight-strong);
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
    gap: var(--space-1);
    align-self: stretch;
  }

  .maps-count {
    margin-left: auto;
  }

  .mob-name {
    font-size: var(--text-md);
    font-weight: var(--weight-strong);
  }

  .missing .mob-name {
    color: var(--faint);
    font-weight: var(--weight-body);
  }

  .muted {
    color: var(--muted);
    font-size: var(--text-xs);
  }

  .faint {
    color: var(--faint);
    font-size: var(--text-xs);
  }

  .maps {
    gap: var(--space-0-5);
    padding: 0 0 var(--space-1-5) 17px;
  }

  .maps > li {
    display: flex;
    align-items: center;
    gap: var(--space-1-5);
    font-size: var(--text-md);
  }

  .dist {
    margin-left: auto;
    color: var(--muted);
    font-size: var(--text-xs);
  }

  .quests {
    gap: var(--space-1-5);
    padding: var(--space-1-5) 0 var(--space-0-5);
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
    font-size: var(--text-md);
  }
</style>
