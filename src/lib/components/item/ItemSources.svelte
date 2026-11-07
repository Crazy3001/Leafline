<script lang="ts">
  // Where an item comes from, by how you get it: the monsters that drop it, each opening to the
  // maps it lives on, and the quests that give it.
  import { ChevronRight } from "@lucide/svelte";

  import GoButton from "$lib/components/GoButton.svelte";
  import { backend } from "$lib/api";
  import { mapPlace } from "$lib/destination.svelte";
  import { groundDistance } from "$lib/format";
  import { showMonsterNamed } from "$lib/monsters.svelte";
  import { showNpcNamed } from "$lib/places.svelte";
  import type { Hunt, ItemWhere } from "$lib/types";

  let { found }: { found: ItemWhere } = $props();

  const drops = $derived(found.origin.drops);
  const quests = $derived(found.origin.quests);
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

{#if known.length || unknown.length}
  <div class="group">
    {@render head("Dropped by", drops.hunts.length, dropsOpen, () => (dropsOpen = !dropsOpen))}
    {#if dropsOpen}
      <ul class="monsters">
        {#each known as hunt (hunt.monster.name)}
          {@const open = isOpen(hunt)}
          <li>
            <button class="mob-head" aria-expanded={open} onclick={() => toggle(hunt)}>
              <span class="chevron" class:open><ChevronRight size={11} /></span>
              <span class="mob-name">{hunt.monster.name}</span>
              <span class="muted tabular">Lv {hunt.monster.level}{hunt.monster.boss ? " · boss" : ""}</span>
              <span class="maps-count faint tabular">{hunt.maps.length} map{hunt.maps.length === 1 ? "" : "s"}</span>
              {#if rate(hunt)}<span class="rate tabular" title={evidence(hunt)}>{rate(hunt)}</span>{/if}
            </button>
            {#if open}
              <ul class="maps">
                {#each hunt.maps as ground (ground.map.id)}
                  <li>
                    <button class="ref ref-map" onclick={() => backend.showPlace("maps", ground.map.id)}>{ground.map.name}</button>
                    <span class="faint tabular">×{ground.count}</span>
                    <span class="dist tabular">{groundDistance(ground, found.from != null)}</span>
                    {#if ground.hops !== 0}<GoButton place={mapPlace(ground.map)} />{/if}
                  </li>
                {:else}
                  <li class="faint">No map in the data spawns it.</li>
                {/each}
                <li class="card-link">
                  <button class="ref ref-mob" onclick={() => showMonsterNamed(hunt.monster.name)}>{hunt.monster.name}'s card</button>
                </li>
              </ul>
            {/if}
          </li>
        {/each}
        {#each unknown as hunt (hunt.monster.name)}
          <li class="mob-head missing">
            <span class="chevron"></span>
            <span class="mob-name">{hunt.monster.name}</span>
            <span class="faint">not in the map data</span>
            {#if rate(hunt)}<span class="rate tabular" title={evidence(hunt)}>{rate(hunt)}</span>{/if}
          </li>
        {/each}
      </ul>
    {/if}
  </div>
{:else if drops.lookedUp}
  <p class="note">No monster drops logged on mapleclassic.wiki.</p>
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
            <button class="quest" onclick={() => backend.showQuest(quest.quest.id)}>{quest.quest.name}</button>
            <span class="who">
              {#if quest.quest.levelMin}Lv {quest.quest.levelMin}{/if}{#if quest.giver}{quest.quest.levelMin ? " · " : ""}<button
                  class="ref ref-npc"
                  onclick={() => showNpcNamed(quest.giver!)}>{quest.giver}</button
                >{/if}{#if giverMap}{quest.quest.levelMin || quest.giver ? " · " : ""}{#if giverMap.known}<button
                    class="ref ref-map"
                    onclick={() => backend.showPlace("maps", giverMap.id)}>{giverMap.name}</button
                  >{:else}{giverMap.name}{/if}{/if}
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

  .card-link .ref {
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

  .quest {
    padding: 0;
    border: 0;
    background: transparent;
    font-size: var(--text-sm);
    font-weight: 600;
    text-align: left;
    cursor: pointer;
  }

  .quest:hover,
  .quest:focus-visible {
    color: var(--accent-strong);
    outline: none;
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
