<script lang="ts">
  // One NPC from the search: where they stand with a route there, and their quests.
  import GoButton from "$lib/components/GoButton.svelte";
  import { npcPlace } from "$lib/destination.svelte";
  import { LoaderCircle } from "@lucide/svelte";
  import { untrack } from "svelte";

  import Card from "../Card.svelte";
  import ShopStock from "../item/ShopStock.svelte";
  import PlaceList from "../map/PlaceList.svelte";
  import Section from "../map/Section.svelte";
  import Ref from "../Ref.svelte";
  import PicFrame from "$lib/ui/PicFrame.svelte";
  import { backend } from "$lib/api";
  import { formatCompact } from "$lib/format";
  import { OSMS_CREDIT, OSMS_URL } from "$lib/mapdata.svelte";
  import { travelKey } from "$lib/settings.svelte";
  import type { MapReading, NpcWhere, QuestRow } from "$lib/types";

  let {
    id,
    map = null,
    plain = false,
    compact = false,
  }: {
    id: number;
    map?: MapReading | null;
    plain?: boolean;
    /** A link card's short form: the quests handed in here stay folded. */
    compact?: boolean;
  } = $props();

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

  /** What they do, for the line under their name. */
  const roles = $derived(
    sheet
      ? [
          sheet.shop ? "Shop" : null,
          sheet.quests.length ? `Gives ${sheet.quests.length} ${sheet.quests.length === 1 ? "quest" : "quests"}` : null,
          sheet.takes.length ? `Takes ${sheet.takes.length} in` : null,
        ].filter((r): r is string => !!r)
      : [],
  );

  function line(quest: QuestRow) {
    const repeat = quest.repeat ? quest.repeat[0].toUpperCase() + quest.repeat.slice(1) : null;
    return [quest.levelMin ? `Lv ${quest.levelMin}` : null, repeat, quest.exp ? `${formatCompact(quest.exp)} EXP` : null].filter(Boolean).join(" · ");
  }
</script>

<Card title="NPC" {plain}>
  {#if sheet}
    <div class="title-row">
      <PicFrame kind="npc" id={sheet.id} size={compact ? 52 : 60} />
      <div class="title-text">
        <h3 class="name">{sheet.name} <GoButton place={npcPlace(sheet.name, sheet.maps.map((m) => m.map))} /></h3>
        {#if roles.length}<p class="roles">{roles.join(" · ")}</p>{/if}
      </div>
    </div>

    <Section title="Where to find them" count={sheet.maps.length}>
      {#if sheet.maps.length}
        <PlaceList places={sheet.maps} from={found?.from ?? null} />
      {:else}
        <p class="note">Not on any map in the data. They may only appear during a quest or an event.</p>
      {/if}
    </Section>

    {#if sheet.shop}
      <Section title="Sells" count={sheet.shop.items.length}>
        <ShopStock shop={sheet.shop} />
      </Section>
    {/if}

    {#if sheet.quests.length}
      <Section title="Quests they give" count={sheet.quests.length}>
        <ul class="quests">
          {#each sheet.quests as quest (quest.id)}
            <li>
              <Ref kind="quest" id={quest.id} name={quest.name} />
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
              <Ref kind="quest" id={quest.id} name={quest.name} />
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
  .title-row {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 10px;
  }

  .title-text {
    min-width: 0;
  }

  .roles {
    margin: 2px 0 0;
    color: var(--muted);
    font-size: var(--text-sm);
  }

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

  .quests :global(.ref) {
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
