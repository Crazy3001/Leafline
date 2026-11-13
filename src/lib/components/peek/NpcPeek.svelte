<script lang="ts">
  import NpcCard from "../lookup/NpcCard.svelte";
  import Resolve from "./Resolve.svelte";
  import { backend } from "$lib/api";
  import type { PeekTarget } from "$lib/peek.svelte";
  import { live } from "$lib/readings.svelte";

  let { target }: { target: PeekTarget } = $props();

  const find = (): Promise<number | null> =>
    target.id != null ? Promise.resolve(target.id) : target.name ? backend.findNpc(target.name) : Promise.resolve(null);
</script>

<Resolve {find} what="NPC" name={target.name}>
  {#snippet children(id: number)}
    <NpcCard {id} map={live.analysis?.map ?? null} plain compact />
  {/snippet}
</Resolve>
