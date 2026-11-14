<script lang="ts">
  import ItemCard from "../ItemCard.svelte";
  import Resolve from "./Resolve.svelte";
  import { backend } from "$lib/api";
  import type { PeekTarget } from "$lib/peek.svelte";
  import type { ItemRecord } from "$lib/types";

  let { target }: { target: PeekTarget } = $props();

  const find = (): Promise<ItemRecord | null> =>
    target.id != null ? backend.itemRecord(target.id) : target.name ? backend.findItem(target.name, null) : Promise.resolve(null);
</script>

<Resolve {find} what="item" name={target.name}>
  {#snippet children(record: ItemRecord)}
    <ItemCard item={null} {record} plain />
  {/snippet}
</Resolve>
