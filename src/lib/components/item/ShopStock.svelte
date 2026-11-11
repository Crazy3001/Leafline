<script lang="ts">
  // What one NPC shop sells, with prices, from Leafline's shop file. Items open their own page.
  import { backend } from "$lib/api";
  import { formatNumber } from "$lib/format";
  import { seenLine, shortDate } from "$lib/shops";
  import type { Shop } from "$lib/types";

  let { shop }: { shop: Shop } = $props();

  const seen = $derived(seenLine(shop.items.map((i) => i.seen)));
</script>

<ul class="stock">
  {#each shop.items as item, i (i)}
    <li>
      {#if item.itemId != null}
        <button class="ref ref-item" onclick={() => backend.showItem(item.itemId, null)}>{item.item}</button>
      {:else}
        <span>{item.item}</span>
      {/if}
      {#if item.needs}<span class="needs">{item.needs}</span>{/if}
      <span class="price tabular" title="Seen {shortDate(item.seen)}">{formatNumber(item.price)}</span>
    </li>
  {/each}
</ul>
{#if seen}<p class="seen">{seen}. They can change; the game's shop window has the last word.</p>{/if}

<style>
  .stock {
    margin: 0;
    padding: 0;
    list-style: none;
    font-size: var(--text-sm);
  }

  .stock li {
    display: flex;
    align-items: baseline;
    gap: 6px;
    padding: 3px 0;
    border-top: 1px solid var(--border);
  }

  .stock li:first-child {
    border-top: 0;
  }

  .needs {
    font-size: var(--text-xs);
    color: var(--warn);
  }

  .price {
    margin-left: auto;
    flex: none;
  }

  .seen {
    margin: 4px 0 0;
    font-size: var(--text-xs);
    color: var(--faint);
  }
</style>
