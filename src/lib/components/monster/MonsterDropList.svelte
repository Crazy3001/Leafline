<script lang="ts">
  // What a monster is known to drop, from Leafline's drop list: which items, and how many mesos.
  // An empty list says so plainly.
  import Ref from "$lib/components/Ref.svelte";
  import type { MonsterDrops } from "$lib/types";

  type Drop = MonsterDrops["drops"][number];

  let { drops, name }: { drops: MonsterDrops | null; name: string } = $props();

  const list = $derived(drops?.drops ?? []);

  function quantity(drop: Drop) {
    if (drop.minQuantity == null || drop.maxQuantity == null) return "";
    return ` ×${drop.minQuantity === drop.maxQuantity ? drop.minQuantity : `${drop.minQuantity}–${drop.maxQuantity}`}`;
  }
</script>

{#if list.length}
  <ul class="drops">
    {#each list as drop (drop.item)}
      <li>
        {#if drop.category === "meso"}
          <span class="mesos">{drop.item}</span>
        {:else}
          <Ref kind="item" name={drop.item} />
        {/if}
        {#if drop.quest}<span class="tag">quest</span>{/if}
        {#if quantity(drop)}<span class="rate tabular">{quantity(drop)}</span>{/if}
      </li>
    {/each}
  </ul>
{:else if drops}
  <p class="note">No known drops for {name} yet.</p>
{:else}
  <p class="note">The drop table isn't downloaded yet.</p>
{/if}

<style>
  .drops {
    display: grid;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li {
    display: flex;
    align-items: baseline;
    gap: 5px;
    padding: 2px 0;
    font-size: var(--text-sm);
  }

  li + li {
    border-top: 1px solid var(--border);
  }

  li :global(.ref) {
    min-width: 0;
    font-weight: 500;
  }

  .mesos {
    color: var(--mesos);
  }

  .rate {
    margin-left: auto;
    color: var(--text);
    font-weight: 600;
    font-size: var(--text-sm);
    font-weight: 600;
    white-space: nowrap;
  }

  .tag {
    padding: 0 4px;
    border-radius: var(--radius-sm);
    background: var(--surface-3);
    color: var(--muted);
    font-size: var(--text-xs);
  }

  .note {
    margin: 0;
    color: var(--faint);
    font-size: var(--text-sm);
  }

</style>
