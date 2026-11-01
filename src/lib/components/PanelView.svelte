<script lang="ts">
  // What one panel shows, the same as a tab of the overlay or pinned into its own window. A
  // pinned window names the panel itself, so single cards drop their frame there.
  import BuildCard from "./BuildCard.svelte";
  import CharacterCard from "./CharacterCard.svelte";
  import HistoryCard from "./HistoryCard.svelte";
  import ItemPanel from "./ItemPanel.svelte";
  import MapCard from "./MapCard.svelte";
  import QuestPanel from "./QuestPanel.svelte";
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
  <QuestPanel
    quest={analysis?.quest ?? null}
    map={analysis?.map ?? null}
    {session}
    level={analysis?.hud?.level ?? session?.level ?? null}
    {pinned}
  />
{:else if panel === "item"}
  <ItemPanel
    tooltips={analysis?.tooltips ?? []}
    level={analysis?.hud?.level ?? session?.level ?? null}
    character={analysis?.hud?.name ?? null}
    map={analysis?.map ?? null}
    {pinned}
  />
{:else if panel === "build"}
  <BuildCard level={analysis?.hud?.level ?? session?.level ?? null} character={analysis?.hud?.name ?? null} plain={pinned} />
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
