<script lang="ts">
  // What one panel shows, as a tab of the overlay or pinned into its own window. A pinned window
  // sits over the game and is glanced at mid-fight, so it shows the part that follows the game by
  // itself and leaves searching and browsing to the tab (Lookup, a tool more than a readout, stays
  // whole). It names the panel itself, so single cards drop their frame there.
  import { can } from "$lib/platform";
  import BuildSection from "./BuildSection.svelte";
  import CharacterCard from "./CharacterCard.svelte";
  import KnownBag from "./item/KnownBag.svelte";
  import LookupPanel from "./LookupPanel.svelte";
  import MapCard from "./MapCard.svelte";
  import QuestPanel from "./QuestPanel.svelte";
  import TrainPanel from "./TrainPanel.svelte";
  import NowPin from "$lib/shell/NowPin.svelte";
  import NowScreen from "$lib/shell/NowScreen.svelte";
  import { tabInfo } from "$lib/shell/tabs";
  import { openPanel } from "$lib/nav.svelte";
  import { ago, profile } from "$lib/profile.svelte";
  import { live } from "$lib/readings.svelte";
  import { splitMapLabel } from "$lib/format";
  import type { Analysis, Panel, SessionSummary } from "$lib/types";

  let {
    panel,
    analysis,
    session,
    pinned = false,
    roomy = false,
    shelled = false,
    wide = false,
    topSearch = false,
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
    /** The desk's top row searches this page's list, so it has no box of its own. */
    topSearch?: boolean;
  } = $props();

  /** The map the character was last read on, while the minimap isn't in view. */
  const lastMap = $derived(profile.known?.map ? splitMapLabel(profile.known.map).name : null);
  const lastMapAt = $derived(profile.known?.seenAt ?? null);
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
    <BuildSection level={analysis?.hud?.level ?? session?.level ?? null} character={profile.live ? (analysis?.hud?.name ?? profile.name) : profile.name} glance={pinned} whole={wide && !pinned} />
  </CharacterCard>
  <!-- Each bag tab as last read, for the character looked at. -->
  {#if !pinned}<KnownBag />{/if}
{:else if panel === "map"}
  {#if analysis?.map}
    <MapCard map={analysis.map} {session} variant={pinned ? "glance" : "compact"} plain={pinned} />
  {:else}
    {@render empty(
      (can.liveReading
        ? live.liveStatus.state === "reading"
          ? "Open your minimap in game: Leafline reads the map's name from its title, and the map's portals, monsters and drops show here."
          : "Leafline isn't reading the game. Once it is, open your minimap and the map you're on shows here."
        : "Search for a map to see its portals, monsters and drops, or share the game on Now and open your minimap.") +
        (lastMap ? ` Last seen on ${lastMap}, ${ago(lastMapAt!)}.` : ""),
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
    {topSearch}
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
