<script lang="ts">
  // What one panel shows, as a tab of the overlay or pinned into its own window. A pinned window
  // sits over the game and is glanced at mid-fight, so it shows the part that follows the game by
  // itself and leaves searching and browsing to the tab (Lookup, a tool more than a readout, stays
  // whole). It names the panel itself, so single cards drop their frame there.
  import { can } from "$lib/platform";
  import BuildSection from "./BuildSection.svelte";
  import CharacterCard from "./CharacterCard.svelte";
  import LookupPanel from "./LookupPanel.svelte";
  import MapCard from "./MapCard.svelte";
  import QuestPanel from "./QuestPanel.svelte";
  import TrainPanel from "./TrainPanel.svelte";
  import NowPin from "$lib/shell/NowPin.svelte";
  import NowScreen from "$lib/shell/NowScreen.svelte";
  import { tabInfo } from "$lib/shell/tabs";
  import { openPanel } from "$lib/nav.svelte";
  import type { Analysis, Panel, SessionSummary } from "$lib/types";

  let {
    panel,
    analysis,
    session,
    pinned = false,
    roomy = false,
    shelled = false,
    wide = false,
  }: {
    panel: Panel;
    analysis: Analysis | null;
    session: SessionSummary | null;
    pinned?: boolean;
    /** A tall column with room, as in the desk view: panels that split into views show them all. */
    roomy?: boolean;
    /** Under the main window's header, which shows the level, name, job and EXP: panels leave those out. */
    shelled?: boolean;
    /** A whole wide window, as a desk view: parts that take turns elsewhere sit side by side. */
    wide?: boolean;
  } = $props();
</script>

{#snippet empty(text: string)}
  <p class="empty">{text}</p>
{/snippet}

{#if panel === "now"}
  {#if pinned}
    <NowPin {analysis} {session} />
  {:else}
    <NowScreen {analysis} {session} ongo={(t) => openPanel(tabInfo(t).panel)} />
  {/if}
{:else if panel === "character"}
  <!-- One card for who the character is and where the build plan has them, so the level shows once. -->
  <CharacterCard hud={analysis?.hud ?? null} mesos={analysis?.mesos ?? null} {session} remember plain={pinned} linkTrain={!pinned} trim={shelled} bare={wide && !pinned}>
    <BuildSection level={analysis?.hud?.level ?? session?.level ?? null} character={analysis?.hud?.name ?? null} glance={pinned} whole={wide && !pinned} />
  </CharacterCard>
{:else if panel === "map"}
  {#if analysis?.map}
    <MapCard map={analysis.map} {session} variant={pinned ? "glance" : "compact"} plain={pinned} />
  {:else}
    {@render empty(
      can.liveReading
        ? "The minimap isn't in view. Its map's details show here when it is."
        : "Search for a map to see its portals, monsters and drops.",
    )}
  {/if}
{:else if panel === "train"}
  <TrainPanel level={analysis?.hud?.level ?? session?.level ?? null} map={analysis?.map ?? null} {session} glance={pinned} whole={roomy} />
{:else if panel === "quests"}
  <QuestPanel
    quest={analysis?.quest ?? null}
    map={analysis?.map ?? null}
    {session}
    level={analysis?.hud?.level ?? session?.level ?? null}
    character={session?.name ?? analysis?.hud?.name ?? null}
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
    padding: var(--space-4) var(--space-3);
    color: var(--faint);
    font-size: var(--text-md);
    text-align: center;
  }
</style>
