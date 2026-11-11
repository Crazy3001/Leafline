<script lang="ts">
  // What a monster is known to drop, from Leafline's drop list: presence only, so rates show only
  // where a drop has one. An empty list says so plainly.
  import { backend } from "$lib/api";
  import { showItemNamed } from "$lib/items.svelte";
  import { DROPS_CREDIT, DROPS_URL } from "$lib/mapdata.svelte";
  import type { MonsterDrops } from "$lib/types";

  type Drop = MonsterDrops["drops"][number];

  let { drops, name }: { drops: MonsterDrops | null; name: string } = $props();

  /** Below this many kills, drops rarer than about 1 in N are likely missing from the log. */
  const FEW_KILLS = 200;

  const list = $derived(drops?.drops ?? []);
  const few = $derived(drops?.sampleKills != null && drops.sampleKills < FEW_KILLS ? drops.sampleKills : null);

  function percent(value: number | null) {
    if (value == null) return "";
    return value >= 10 ? `${value.toFixed(0)}%` : `${value.toFixed(1)}%`;
  }

  function quantity(drop: Drop) {
    if (drop.minQuantity == null || drop.maxQuantity == null) return "";
    return ` ×${drop.minQuantity === drop.maxQuantity ? drop.minQuantity : `${drop.minQuantity}–${drop.maxQuantity}`}`;
  }

  /** How solid a rate is, for the tooltip. */
  function evidence(drop: Drop) {
    return [
      drop.rateLow != null && drop.rateHigh != null ? `Likely between ${drop.rateLow.toFixed(1)}% and ${drop.rateHigh.toFixed(1)}%` : null,
      drop.sampleKills ? `${drop.sampleKills.toLocaleString()} kills logged` : null,
      drop.gameVersion && `v${drop.gameVersion}`,
    ]
      .filter(Boolean)
      .join(" · ");
  }
</script>

{#if list.length}
  <ul class="drops">
    {#each list as drop (drop.item)}
      <li class:unmeasured={drop.rate == null} title={evidence(drop)}>
        {#if drop.category === "meso"}
          <span class="mesos">{drop.item}</span>
        {:else}
          <button class="ref ref-item" onclick={() => showItemNamed(drop.item)}>{drop.item}</button>
        {/if}
        {#if drop.quest}<span class="tag">quest</span>{/if}
        <span class="rate tabular">{percent(drop.rate)}{quantity(drop)}</span>
      </li>
    {/each}
  </ul>
  {#if few != null}
    <p class="few">Only {few} kills logged, so rarer drops, such as most equips and potions, may not show up yet.</p>
  {/if}
  <p class="credit">
    Known drops, without rates.
    <button onclick={() => backend.openLink(DROPS_URL)}>{DROPS_CREDIT}</button>
  </p>
{:else if drops}
  <p class="note">No known drops for {name} yet.</p>
{:else}
  <p class="note">The drop table isn't downloaded yet.</p>
{/if}

<style>
  .drops {
    display: grid;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li {
    display: flex;
    align-items: baseline;
    gap: 5px;
    padding: 2px 0;
    font-size: var(--text-sm);
  }

  li + li {
    border-top: 1px solid var(--border);
  }

  li .ref {
    min-width: 0;
    font-weight: 500;
  }

  .mesos {
    color: var(--mesos);
  }

  .rate {
    margin-left: auto;
    color: var(--text);
    font-weight: 600;
    font-size: var(--text-sm);
    font-weight: 600;
    white-space: nowrap;
  }

  .unmeasured .rate {
    color: var(--faint);
  }

  .tag {
    padding: 0 4px;
    border-radius: var(--radius-sm);
    background: var(--surface-3);
    color: var(--muted);
    font-size: var(--text-xs);
  }

  .few {
    margin: 8px 0 0;
    padding: 4px 7px;
    border-radius: var(--radius-sm);
    background: var(--warn-soft);
    color: var(--muted);
    font-size: var(--text-xs);
  }

  .note {
    margin: 0;
    color: var(--faint);
    font-size: var(--text-sm);
  }

  .credit {
    margin: 8px 0 0;
    color: var(--faint);
    font-size: var(--text-xs);
  }

  .credit button {
    display: block;
    margin-top: 3px;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--muted);
    font-size: var(--text-xs);
    text-align: left;
    text-decoration: underline;
    text-decoration-color: var(--border-strong);
    cursor: pointer;
  }
</style>
