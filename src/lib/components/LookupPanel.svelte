<script lang="ts">
  // The Lookup tab: items and monsters, each searched from the map data. Neither needs anything
  // from the game, so both work with live reading off.
  import ItemPanel from "./ItemPanel.svelte";
  import MonsterPanel from "./MonsterPanel.svelte";
  import { lookup } from "$lib/items.svelte";
  import { watchMonsters } from "$lib/monsters.svelte";
  import type { ItemTooltip, MapReading, QuestReading, SessionSummary } from "$lib/types";

  let {
    tooltips,
    level = null,
    character = null,
    map = null,
    quest = null,
    session = null,
    pinned = false,
  }: {
    tooltips: ItemTooltip[];
    level?: number | null;
    character?: string | null;
    map?: MapReading | null;
    quest?: QuestReading | null;
    session?: SessionSummary | null;
    pinned?: boolean;
  } = $props();

  const MODES = [
    { id: "items", label: "Items" },
    { id: "monsters", label: "Monsters" },
  ] as const;

  watchMonsters();
</script>

<div class="modes" class:pinned role="radiogroup" aria-label="What to look up">
  {#each MODES as mode (mode.id)}
    <button role="radio" aria-checked={lookup.mode === mode.id} onclick={() => (lookup.mode = mode.id)}>{mode.label}</button>
  {/each}
</div>

{#if lookup.mode === "monsters"}
  <MonsterPanel {map} {level} {quest} {session} {pinned} />
{:else}
  <ItemPanel {tooltips} {level} {character} {map} {pinned} />
{/if}

<style>
  .modes {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0;
    margin: 10px 12px 0;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
  }

  .modes.pinned {
    margin: 0 0 8px;
  }

  .modes button {
    height: 26px;
    padding: 0 8px;
    border: 0;
    background: transparent;
    color: var(--muted);
    font-size: 12px;
    cursor: pointer;
  }

  .modes button + button {
    border-left: 1px solid var(--border-strong);
  }

  .modes button:hover,
  .modes button:focus-visible {
    color: var(--text);
    outline: none;
  }

  .modes [aria-checked="true"] {
    background: var(--accent-soft);
    color: var(--accent-strong);
    font-weight: 600;
  }
</style>
