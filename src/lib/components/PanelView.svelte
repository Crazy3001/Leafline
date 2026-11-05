<script lang="ts">
  // What one panel shows, the same as a tab of the overlay or pinned into its own window. A
  // pinned window names the panel itself, so single cards drop their frame there.
  import BuildSection from "./BuildSection.svelte";
  import CharacterCard from "./CharacterCard.svelte";
  import LookupPanel from "./LookupPanel.svelte";
  import MapCard from "./MapCard.svelte";
  import QuestPanel from "./QuestPanel.svelte";
  import TrainPanel from "./TrainPanel.svelte";
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

{#if panel === "character"}
  <!-- One card for who the character is and where the build plan has them, so the level shows once. -->
  <CharacterCard hud={analysis?.hud ?? null} mesos={analysis?.mesos ?? null} {session} remember plain={pinned} linkTrain={!pinned}>
    <BuildSection level={analysis?.hud?.level ?? session?.level ?? null} character={analysis?.hud?.name ?? null} />
  </CharacterCard>
{:else if panel === "map"}
  {#if analysis?.map}
    <MapCard map={analysis.map} {session} variant="compact" plain={pinned} />
  {:else}
    {@render empty("The minimap isn't in view. Its map's details show here when it is.")}
  {/if}
{:else if panel === "train"}
  <TrainPanel level={analysis?.hud?.level ?? session?.level ?? null} map={analysis?.map ?? null} {session} />
{:else if panel === "quests"}
  <QuestPanel
    quest={analysis?.quest ?? null}
    map={analysis?.map ?? null}
    {session}
    level={analysis?.hud?.level ?? session?.level ?? null}
    {pinned}
  />
{:else if panel === "item"}
  <LookupPanel
    tooltips={analysis?.tooltips ?? []}
    level={analysis?.hud?.level ?? session?.level ?? null}
    character={analysis?.hud?.name ?? null}
    job={analysis?.hud?.job ?? null}
    map={analysis?.map ?? null}
    quest={analysis?.quest ?? null}
    {session}
    {pinned}
  />
{/if}

<style>
  .empty {
    margin: 0;
    padding: 18px 10px;
    color: var(--faint);
    font-size: var(--text-sm);
    text-align: center;
  }
</style>
