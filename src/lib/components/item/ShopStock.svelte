<script lang="ts">
  // What one NPC shop sells, with prices, from Leafline's shop file. Items open their own page.
  import Ref from "$lib/components/Ref.svelte";
  import { formatNumber } from "$lib/format";
  import type { Shop } from "$lib/types";

  let { shop }: { shop: Shop } = $props();

</script>

<ul class="stock">
  {#each shop.items as item, i (i)}
    <li>
      {#if item.itemId != null}
        <Ref kind="item" id={item.itemId} name={item.item} />
      {:else}
        <span>{item.item}</span>
      {/if}
      {#if item.needs}<span class="needs">{item.needs}</span>{/if}
      <span class="price tabular">{formatNumber(item.price)}</span>
    </li>
  {/each}
</ul>


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

</style>
