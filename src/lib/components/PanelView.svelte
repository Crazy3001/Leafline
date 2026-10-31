<script lang="ts">
  // What one panel shows, the same as a tab of the overlay or pinned into its own window. A
  // pinned window names the panel itself, so single cards drop their frame there.
  import CharacterCard from "./CharacterCard.svelte";
  import HistoryCard from "./HistoryCard.svelte";
  import ItemCard from "./ItemCard.svelte";
  import MapCard from "./MapCard.svelte";
  import QuestCard from "./QuestCard.svelte";
  import SessionCard from "./SessionCard.svelte";
  import TimersCard from "./TimersCard.svelte";
  import TrainCard from "./TrainCard.svelte";
  import type { Analysis, Panel, SessionSummary } from "$lib/types";

  let {
    panel,
    analysis,
    session,
    pinned = false,
  }: { panel: Panel; analysis: Analysis | null; session: SessionSummary | null; pinned?: boolean } = $props();
</script>

{#snippet empty(text: string)}
  <p class="empty">{text}</p>
{/snippet}

{#if panel === "stats"}
  <CharacterCard hud={analysis?.hud ?? null} mesos={analysis?.mesos ?? null} {session} />
  {#if session}<SessionCard {session} compact />{/if}
  <HistoryCard />
  <TimersCard />
{:else if panel === "map"}
  {#if analysis?.map}
    <MapCard map={analysis.map} {session} variant="compact" plain={pinned} />
  {:else}
    {@render empty("The minimap isn't in view. Its map's details show here when it is.")}
  {/if}
{:else if panel === "train"}
  <TrainCard level={analysis?.hud?.level ?? session?.level ?? null} map={analysis?.map ?? null} {session} plain={pinned} />
{:else if panel === "quests"}
  {#if analysis?.quest}
    <QuestCard quest={analysis.quest} map={analysis.map} {session} plain={pinned} />
  {:else}
    {@render empty("Open the Quest Helper in game to track quest counters here.")}
  {/if}
{:else if panel === "item"}
  {#each analysis?.tooltips ?? [] as item, i}
    <ItemCard {item} title={i === 0 ? "Item" : "Compared item"} plain={pinned && i === 0} />
  {:else}
    {@render empty("Hover a piece of equipment in game and press Ctrl+Alt+I to see its stats and whether it has been scrolled.")}
  {/each}
{/if}

<style>
  .empty {
    margin: 0;
    padding: 18px 10px;
    color: var(--faint);
    font-size: 12px;
    text-align: center;
  }
</style>
