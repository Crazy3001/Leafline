<script lang="ts">
  // One NPC from the search: where they stand with a route there, and their quests.
  import GoButton from "$lib/components/GoButton.svelte";
  import { npcPlace } from "$lib/destination.svelte";
  import { LoaderCircle } from "@lucide/svelte";
  import { untrack } from "svelte";

  import Card from "../Card.svelte";
  import PlaceList from "../map/PlaceList.svelte";
  import Section from "../map/Section.svelte";
  import { backend } from "$lib/api";
  import { formatCompact } from "$lib/format";
  import { OSMS_CREDIT, OSMS_URL } from "$lib/mapdata.svelte";
  import { travelKey } from "$lib/settings.svelte";
  import type { MapReading, NpcWhere, QuestRow } from "$lib/types";

  let { id, map = null, plain = false }: { id: number; map?: MapReading | null; plain?: boolean } = $props();

  let found = $state<NpcWhere | null>(null);
  let error = $state<string | null>(null);

  const key = $derived(JSON.stringify([id, map?.name, map?.street, map?.twin, travelKey()]));

  // Follows the key alone: every live reading brings new objects with the same content.
  $effect(() => {
    const asked = key;
    error = null;
    untrack(() => backend.npcSheet(id, map?.name ?? null, map?.street ?? null, map?.twin ?? null))
      .then((result) => {
        if (asked === key) found = result;
      })
      .catch((e) => (error = String(e)));
  });

  const sheet = $derived(found?.sheet.id === id ? found.sheet : null);

  function line(quest: QuestRow) {
    const repeat = quest.repeat ? quest.repeat[0].toUpperCase() + quest.repeat.slice(1) : null;
    return [quest.levelMin ? `Lv ${quest.levelMin}` : null, repeat, quest.exp ? `${formatCompact(quest.exp)} EXP` : null].filter(Boolean).join(" · ");
  }
</script>

<Card title="NPC" {plain}>
  {#if sheet}
    <h3 class="name">{sheet.name} <GoButton place={npcPlace(sheet.name, sheet.maps.map((m) => m.map))} /></h3>

    <Section title="Where to find them" count={sheet.maps.length}>
      {#if sheet.maps.length}
        <PlaceList places={sheet.maps} from={found?.from ?? null} />
      {:else}
        <p class="note">Not on any map in the data. They may only appear during a quest or an event.</p>
      {/if}
    </Section>

    {#if sheet.quests.length}
      <Section title="Quests they give" count={sheet.quests.length}>
        <ul class="quests">
          {#each sheet.quests as quest (quest.id)}
            <li>
              <button class="ref ref-quest" onclick={() => backend.showQuest(quest.id)}>{quest.name}</button>
              <span class="who">{line(quest)}</span>
            </li>
          {/each}
        </ul>
      </Section>
    {/if}

    {#if sheet.takes.length}
      <Section title="Quests handed in here" count={sheet.takes.length} open={false}>
        <ul class="quests">
          {#each sheet.takes as quest (quest.id)}
            <li>
              <button class="ref ref-quest" onclick={() => backend.showQuest(quest.id)}>{quest.name}</button>
              {#if quest.levelMin}<span class="who">Lv {quest.levelMin}</span>{/if}
            </li>
          {/each}
        </ul>
      </Section>
    {/if}

    <button class="credit" onclick={() => backend.openLink(OSMS_URL)}>{OSMS_CREDIT}</button>
  {:else if error}
    <p class="note">{error}</p>
  {:else}
    <p class="note"><span class="spin"><LoaderCircle size={13} /></span> Looking it up…</p>
  {/if}
</Card>

<style>
  .name {
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--text-lg);
    font-weight: 600;
    line-height: 1.25;
  }

  .quests {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .quests li {
    padding: 3px 0;
  }

  .quests li + li {
    border-top: 1px solid var(--border);
  }

  .quests .ref {
    display: block;
    font-size: var(--text-sm);
    font-weight: 600;
  }

  .who {
    color: var(--faint);
    font-size: var(--text-xs);
  }

  .note {
    margin: 0;
    color: var(--faint);
    font-size: var(--text-sm);
  }

  .spin {
    display: inline-grid;
    animation: spin 0.9s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .credit {
    display: block;
    margin-top: 12px;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--faint);
    font-size: var(--text-xs);
    text-align: left;
    cursor: pointer;
  }

  .credit:hover,
  .credit:focus-visible {
    color: var(--muted);
    outline: none;
  }
</style>
