<script lang="ts">
  import QuestSheetCard from "../QuestSheetCard.svelte";
  import Resolve from "./Resolve.svelte";
  import type { PeekTarget } from "$lib/peek.svelte";
  import { live } from "$lib/readings.svelte";

  let { target }: { target: PeekTarget } = $props();

  const find = (): Promise<number | null> => Promise.resolve(target.id);
</script>

<Resolve {find} what="quest" name={target.name}>
  {#snippet children(id: number)}
    <QuestSheetCard {id} map={live.analysis?.map ?? null} quest={live.analysis?.quest ?? null} plain compact />
  {/snippet}
</Resolve>
