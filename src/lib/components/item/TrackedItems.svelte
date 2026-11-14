<script lang="ts">
  // The items the player starred: each with the monster that drops it and the best map to hunt
  // it, opening to every dropper and map with a route, and a shop when an NPC sells it.
  import { tip } from "$lib/tip.svelte";
  import { Star, X } from "@lucide/svelte";
  import { untrack as quietly } from "svelte";

  import Card from "../Card.svelte";
  import Ref from "../Ref.svelte";
  import ObjectiveHint from "../quest/ObjectiveHint.svelte";
  import { backend } from "$lib/api";
  import { nearestAndRichest, trackedItems, untrack, watchTracked } from "$lib/tracked.svelte";
  import Pic from "$lib/ui/Pic.svelte";
  import type { MapReading, ObjectiveGuide, ItemWhere } from "$lib/types";

  let { map = null, plain = false }: { map?: MapReading | null; plain?: boolean } = $props();

  /** Droppers and maps shown per item before the item's own page. */
  const DROPPERS = 4;
  const MAPS = 4;

  watchTracked();

  const items = $derived(trackedItems());
  const key = $derived(JSON.stringify([items.map((i) => i.id), map?.name, map?.street, map?.twin]));
  let found = $state<Record<number, ItemWhere | null>>({});

  // Follows the key alone: every live reading brings new objects with the same content.
  $effect(() => {
    const asked = key;
    quietly(() => {
      Promise.all(
        items.map((i) => backend.itemOrigin(i.id, map?.name ?? null, map?.street ?? null, map?.twin ?? null).catch(() => null)),
      ).then((results) => {
        if (asked !== key) return;
        const next: Record<number, ItemWhere | null> = {};
        items.forEach((item, i) => (next[item.id] = results[i]));
        found = next;
      });
    });
  });

  /** An item's sources as a quest objective's guide, cut to the best few. */
  function guide(where: ItemWhere): ObjectiveGuide {
    const drops = where.origin.drops;
    return {
      ...drops,
      hunts: drops.hunts.slice(0, DROPPERS).map((h) => ({ ...h, maps: nearestAndRichest(h.maps, MAPS) })),
      shops: where.origin.shops ?? [],
    };
  }
</script>

{#if items.length}
  <Card title="Tracked items" {plain}>
    <ul class="tracked">
      {#each items as item (item.id)}
        {@const where = found[item.id]}
        <li>
          <div class="head">
            <span class="star"><Star size={11} /></span>
            <Pic kind="item" id={item.id} size={18} />
            <Ref kind="item" id={item.id} name={item.name} icon={false} />
            <button class="remove" use:tip={"Stop tracking"} aria-label="Stop tracking {item.name}" onclick={() => untrack(item.id)}
              ><X size={11} /></button
            >
          </div>
          {#if where}
            <ObjectiveHint guide={guide(where)} from={where.from} />
            {#if !where.origin.drops.hunts.length && !where.origin.shops?.length && where.origin.quests.length}
              <p class="note">
                From a quest: <Ref kind="quest" id={where.origin.quests[0].quest.id} name={where.origin.quests[0].quest.name} />
              </p>
            {:else if !where.origin.drops.hunts.length && !where.origin.shops?.length}
              <p class="note">No drop, shop or quest is known for it.</p>
            {/if}
          {:else if where === null}
            <p class="note">Not in the map data.</p>
          {/if}
        </li>
      {/each}
    </ul>
  </Card>
{/if}

<style>
  .tracked {
    display: grid;
    gap: 10px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .head {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: var(--text-sm);
    font-weight: 600;
  }

  .head :global(.ref) {
    flex: 1;
    min-width: 0;
  }

  .star {
    display: inline-grid;
    color: var(--mesos);
  }

  .remove {
    display: inline-grid;
    place-items: center;
    width: 20px;
    height: 20px;
    padding: 0;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--faint);
    cursor: pointer;
  }

  .remove:hover,
  .remove:focus-visible {
    background: rgba(255, 255, 255, 0.06);
    color: var(--text);
  }

  .note {
    margin: 4px 0 0 19px;
    color: var(--faint);
    font-size: var(--text-xs);
  }
</style>
