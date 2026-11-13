<script lang="ts">
  import MonsterCard from "../MonsterCard.svelte";
  import Resolve from "./Resolve.svelte";
  import { backend } from "$lib/api";
  import type { PeekTarget } from "$lib/peek.svelte";
  import { live } from "$lib/readings.svelte";

  let { target }: { target: PeekTarget } = $props();

  const find = (): Promise<number | null> =>
    target.id != null ? Promise.resolve(target.id) : target.name ? backend.findMonster(target.name) : Promise.resolve(null);
</script>

<Resolve {find} what="monster" name={target.name}>
  {#snippet children(id: number)}
    <MonsterCard {id} map={live.analysis?.map ?? null} level={live.analysis?.hud?.level ?? null} session={live.session} plain compact />
  {/snippet}
</Resolve>
