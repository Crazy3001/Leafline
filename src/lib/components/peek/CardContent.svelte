<script lang="ts">
  // What a link card shows for its target, the same in the card beside a link, a pinned card and
  // a card popped out into a window of its own.
  import ItemPeek from "./ItemPeek.svelte";
  import MapPeek from "./MapPeek.svelte";
  import MobPeek from "./MobPeek.svelte";
  import NpcPeek from "./NpcPeek.svelte";
  import QuestPeek from "./QuestPeek.svelte";
  import SkillPeek from "./SkillPeek.svelte";
  import type { PeekTarget } from "$lib/peek.svelte";

  let { target }: { target: PeekTarget } = $props();
</script>

{#if target.kind === "item"}
  <ItemPeek {target} />
{:else if target.kind === "mob"}
  <MobPeek {target} />
{:else if target.kind === "npc"}
  <NpcPeek {target} />
{:else if target.kind === "map"}
  <MapPeek {target} />
{:else if target.kind === "skill"}
  <SkillPeek {target} />
{:else}
  <QuestPeek {target} />
{/if}
