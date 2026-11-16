<script lang="ts">
  // The loot one session picked up, as the game's Pickup Log showed it: each item and how many,
  // with what they are worth (the player's own price when they logged one, else the NPC price),
  // and the mesos picked up. A line Leafline couldn't read is left out, so the totals can come out
  // short but never over.
  import { ChevronRight } from "@lucide/svelte";

  import { backend } from "$lib/api";
  import Ref from "$lib/components/Ref.svelte";
  import { formatCompact, formatDuration, formatNumber } from "$lib/format";
  import { lootPerHour, lootValue, valued, wantPrices } from "$lib/loot.svelte";
  import { tip } from "$lib/tip.svelte";
  import type { LootSummary, SessionLoot } from "$lib/types";
  import Button from "$lib/ui/Button.svelte";

  let {
    loot,
    collapsible = true,
    reading = false,
  }: {
    loot: LootSummary | SessionLoot | null | undefined;
    /** Folds behind a one-line summary, as on the Train tab. */
    collapsible?: boolean;
    /** Live reading is on, so an unseen log is worth a word. */
    reading?: boolean;
  } = $props();

  const OPEN_KEY = "leafline.train.lootOpen";
  let opened = $state(stored() === "true");
  const open = $derived(!collapsible || opened);
  let exported = $state<string | null>(null);

  function stored() {
    try {
      return localStorage.getItem(OPEN_KEY);
    } catch {
      return null;
    }
  }

  function toggle() {
    opened = !opened;
    try {
      localStorage.setItem(OPEN_KEY, String(opened));
    } catch {
      // A convenience only.
    }
  }

  const seen = $derived(loot != null && ("watched" in loot ? loot.watched : true));
  const items = $derived(loot?.items ?? []);
  $effect(() => wantPrices(items));
  const rows = $derived(valued(items));
  const worth = $derived(lootValue(items));
  const perHour = $derived(loot ? lootPerHour(items, loot.seconds) : null);
  const priced = $derived(rows.filter((r) => r.total != null).length);
  const missed = $derived((loot?.unread ?? 0) + (loot?.unnamed ?? 0));

  async function exportCsv() {
    exported = null;
    try {
      const count = await backend.exportCsv("loot");
      if (count != null) exported = `${formatNumber(count)} row${count === 1 ? "" : "s"} saved.`;
    } catch (e) {
      exported = String(e);
    }
  }
</script>

<div class="loot">
  {#if collapsible}
    <button class="head" aria-expanded={open} onclick={toggle}>
      <span class="chevron" class:open><ChevronRight size={12} /></span>
      <span class="title">Loot</span>
      <span class="grow"></span>
      {#if seen && perHour != null}
        <span class="sum tabular">{formatCompact(perHour)} <span class="muted">mesos / hr in items</span></span>
      {:else if seen}
        <span class="muted">{items.length ? `${items.length} kind${items.length === 1 ? "" : "s"}` : "Nothing yet"}</span>
      {:else}
        <span class="muted">Pickup Log not seen</span>
      {/if}
    </button>
  {/if}

  {#if open}
    <div class="body">
      {#if !seen}
        <p class="muted">
          {#if reading}
            Leafline counts loot from the lines at the bottom right of the game that say what you picked up. It hasn't
            seen them yet: pick something up and they show.
          {:else}
            Loot is counted from the game's Pickup Log while live reading is on.
          {/if}
        </p>
      {:else}
        <div class="totals tabular">
          <span><span class="muted">Mesos picked up</span> <strong>{formatNumber(loot?.mesos ?? 0)}</strong></span>
          <span
            ><span class="muted">Items worth</span>
            <strong>{formatNumber(worth)}</strong>{#if priced < rows.length}<span class="muted">
                · {rows.length - priced} unpriced</span
              >{/if}</span
          >
        </div>
        {#if rows.length}
          <table>
            <thead><tr><th>Item</th><th>Count</th><th>Each</th><th>Worth</th></tr></thead>
            <tbody>
              {#each rows as row (row.id)}
                <tr>
                  <td><Ref kind="item" id={row.id} name={row.name} /></td>
                  <td class="tabular">{formatNumber(row.count)}</td>
                  <td class="tabular" class:own={row.source === "own"}>
                    {#if row.each != null}
                      <span use:tip={row.source === "own" ? "The middle of the prices you logged" : "What an NPC pays, going by the item data"}
                        >{formatNumber(row.each)}</span
                      >
                    {:else}
                      <span class="muted">—</span>
                    {/if}
                  </td>
                  <td class="tabular">{row.total != null ? formatNumber(row.total) : "—"}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        {:else}
          <p class="muted">No items picked up yet.</p>
        {/if}
        <p class="small muted">
          Over {formatDuration(loot?.seconds ?? 0)} of play.{#if missed > 0}
            {missed} line{missed === 1 ? "" : "s"} couldn't be read or named and {missed === 1 ? "isn't" : "aren't"} counted.{/if}
          Items are worth your own logged price, else the NPC price.
        </p>
        {#if !collapsible}
          <div class="actions">
            <Button variant="ghost" size="sm" onclick={exportCsv}>Export loot CSV</Button>
            {#if exported}<span class="muted small">{exported}</span>{/if}
          </div>
        {/if}
      {/if}
    </div>
  {/if}
</div>

<style>
  .loot {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .head {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    padding: 4px 0;
    border: 0;
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
    text-align: left;
    cursor: pointer;
  }

  .head:hover .title {
    color: var(--accent-strong);
  }

  .title {
    font-weight: 600;
  }

  .chevron {
    display: inline-grid;
    color: var(--faint);
    transition: transform 0.12s ease;
  }

  .chevron.open {
    transform: rotate(90deg);
  }

  .grow {
    flex: 1;
  }

  .body {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .muted {
    color: var(--muted);
  }

  p {
    margin: 0;
    font-size: var(--text-sm);
  }

  .small {
    font-size: var(--text-xs);
  }

  .totals {
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
    gap: 8px;
    font-size: var(--text-sm);
  }

  .totals strong {
    font-weight: 600;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--text-sm);
  }

  th {
    padding: 0 0 4px;
    font-size: var(--text-xs);
    font-weight: 400;
    color: var(--faint);
    text-align: right;
  }

  td {
    padding: 3px 0;
    border-top: 1px solid var(--border);
    text-align: right;
  }

  th:first-child,
  td:first-child {
    text-align: left;
  }

  td.own {
    color: var(--accent-strong);
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }
</style>
