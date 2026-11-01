<script lang="ts">
  // One quest from the search: who it comes from and how it goes, step by step.
  import { LoaderCircle } from "@lucide/svelte";
  import { untrack } from "svelte";

  import Card from "./Card.svelte";
  import QuestSteps from "./quest/QuestSteps.svelte";
  import { backend } from "$lib/api";
  import { OSMS_CREDIT, OSMS_URL, WIKI_CREDIT, WIKI_URL } from "$lib/mapdata.svelte";
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

    <QuestSteps {sheet} from={found?.from ?? null} />

    <p class="credit">
      <button onclick={() => backend.openLink(OSMS_URL)}>{OSMS_CREDIT}</button>
      <button onclick={() => backend.openLink(WIKI_URL)}>{WIKI_CREDIT}</button>
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
    font-size: 16px;
    font-weight: 600;
    line-height: 1.25;
  }

  .meta {
    margin: 2px 0 6px;
    color: var(--muted);
    font-size: 12px;
  }

  .note {
    margin: 0;
    color: var(--faint);
    font-size: 11.5px;
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
    font-size: 10.5px;
    text-align: left;
    cursor: pointer;
  }

  .credit button:hover,
  .credit button:focus-visible {
    color: var(--muted);
    outline: none;
  }
</style>
