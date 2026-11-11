<script lang="ts">
  // One quest from the search: who it comes from and how it goes, step by step.
  import { Check, LoaderCircle, Plus } from "@lucide/svelte";
  import { untrack } from "svelte";

  import Card from "./Card.svelte";
  import QuestSteps from "./quest/QuestSteps.svelte";
  import { backend } from "$lib/api";
  import { DROPS_CREDIT, DROPS_URL, OSMS_CREDIT, OSMS_URL } from "$lib/mapdata.svelte";
  import { follow, isFollowed, unfollow } from "$lib/quests.svelte";
  import type { MapReading, QuestWhere } from "$lib/types";

  let { id, map = null, plain = false }: { id: number; map?: MapReading | null; plain?: boolean } = $props();

  let found = $state<QuestWhere | null>(null);
  let error = $state<string | null>(null);

  const key = $derived(JSON.stringify([id, map?.name, map?.street, map?.twin]));

  // Follows the key alone: every live reading brings new objects with the same content.
  $effect(() => {
    const asked = key;
    error = null;
    untrack(() => backend.questSheet(id, map?.name ?? null, map?.street ?? null, map?.twin ?? null))
      .then((result) => {
        if (asked === key) found = result;
      })
      .catch((e) => (error = String(e)));
  });

  const sheet = $derived(found?.sheet.row.id === id ? found.sheet : null);
  const meta = $derived(
    sheet
      ? [
          sheet.row.levelMin ? `Lv ${sheet.row.levelMin}` : null,
          sheet.row.region,
          sheet.row.repeat ? sheet.row.repeat[0].toUpperCase() + sheet.row.repeat.slice(1) : null,
          sheet.chain,
        ].filter(Boolean)
      : [],
  );
</script>

<Card title="Quest" {plain}>
  {#if sheet}
    <h3 class="name">{sheet.row.name}</h3>
    {#if meta.length}<p class="meta">{meta.join(" · ")}</p>{/if}
    {#if isFollowed(sheet.row.id)}
      <p class="following">
        <Check size={12} /><span>Following in the Quests tab</span>
        <button class="stop" onclick={() => unfollow(sheet.row.id)}>Stop</button>
      </p>
    {:else}
      <button class="follow" onclick={() => follow(sheet.row.id, sheet.row.name)}><Plus size={12} />Follow</button>
    {/if}

    <QuestSteps {sheet} from={found?.from ?? null} />

    <p class="credit">
      <button onclick={() => backend.openLink(OSMS_URL)}>{OSMS_CREDIT}</button>
      <button onclick={() => backend.openLink(DROPS_URL)}>{DROPS_CREDIT}</button>
    </p>
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

  .meta {
    margin: 2px 0 6px;
    color: var(--muted);
    font-size: var(--text-sm);
  }

  /* Keeps a quest in the Quests tab, with its hints, while the Quest Helper isn't showing it. */
  .follow {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 26px;
    margin: 0 0 8px;
    padding: 0 9px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: transparent;
    font-size: var(--text-sm);
    cursor: pointer;
  }

  .follow:hover,
  .follow:focus-visible {
    background: var(--surface-3);
    outline: none;
  }

  .following {
    display: flex;
    align-items: center;
    gap: 5px;
    min-height: 26px;
    margin: 0 0 8px;
    color: var(--good);
    font-size: var(--text-sm);
  }

  .stop {
    margin-left: 4px;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--muted);
    font-size: var(--text-sm);
    text-decoration: underline;
    text-decoration-color: var(--border-strong);
    cursor: pointer;
  }

  .stop:hover,
  .stop:focus-visible {
    color: var(--text);
    outline: none;
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
    display: grid;
    gap: 2px;
    margin: 12px 0 0;
  }

  .credit button {
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--faint);
    font-size: var(--text-xs);
    text-align: left;
    cursor: pointer;
  }

  .credit button:hover,
  .credit button:focus-visible {
    color: var(--muted);
    outline: none;
  }
</style>
