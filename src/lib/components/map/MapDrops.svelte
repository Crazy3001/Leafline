<script lang="ts">
  // Drops players logged on mapleclassic.wiki, fetched only when asked for.
  import { Download, LoaderCircle } from "@lucide/svelte";

  import { backend } from "$lib/api";
  import { WIKI_CREDIT, WIKI_URL } from "$lib/mapdata.svelte";
  import type { MonsterDrops } from "$lib/types";

  let { monsters }: { monsters: string[] } = $props();

  let drops = $state<MonsterDrops[] | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);

  // A different map's monsters: forget the old list.
  $effect(() => {
    void monsters.join("|");
    drops = null;
    error = null;
  });

  async function load() {
    loading = true;
    error = null;
    const asked = monsters.join("|");
    try {
      const result = await backend.monsterDrops(monsters);
      if (asked === monsters.join("|")) drops = result;
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  function percent(value: number | null) {
    if (value == null) return "?";
    return value >= 10 ? `${value.toFixed(0)}%` : `${value.toFixed(1)}%`;
  }

  function sample(entry: MonsterDrops) {
    return [entry.sampleKills ? `${entry.sampleKills.toLocaleString()} kills` : null, entry.gameVersion && `v${entry.gameVersion}`]
      .filter(Boolean)
      .join(" · ");
  }

  /** Below this many kills, drops rarer than about 1 in N are likely missing from the log. */
  const FEW_KILLS = 200;

  function wikiPage(monster: string) {
    return `${WIKI_URL}/index.php?search=${encodeURIComponent(monster)}&go=Go`;
  }

  function range(low: number | null, high: number | null) {
    return low != null && high != null ? `Likely between ${low}% and ${high}%` : "";
  }
</script>

{#if drops}
  {#each drops as entry (entry.monster)}
    <div class="monster">
      <div class="head">
        <button class="name" title="Open {entry.monster} on mapleclassic.wiki" onclick={() => backend.openLink(wikiPage(entry.monster))}
          >{entry.monster}</button
        >
        <span class="sample">{sample(entry)}</span>
      </div>
      {#if entry.drops.length}
        <ul>
          {#each entry.drops as drop}
            <li class:unmeasured={drop.rate == null}>
              <span class="rate tabular" title={range(drop.rateLow, drop.rateHigh)}>{percent(drop.rate)}</span>
              <span class="item">
                {drop.item}
                {#if drop.minQuantity != null && drop.maxQuantity != null}
                  <span class="qty">
                    ×{drop.minQuantity === drop.maxQuantity ? drop.minQuantity : `${drop.minQuantity}–${drop.maxQuantity}`}
                  </span>
                {/if}
                {#if drop.quest}<span class="tag">quest</span>{/if}
              </span>
              {#if drop.category}<span class="cat">{drop.category}</span>{/if}
            </li>
          {/each}
        </ul>
      {:else}
        <p class="note">No drops logged yet.</p>
      {/if}
      {#if entry.sampleKills != null && entry.sampleKills < FEW_KILLS}
        <p class="few">
          Only {entry.sampleKills} kills logged, so drops rarer than about 1 in {entry.sampleKills}, such as most equips
          and potions, may not show up yet.
        </p>
      {/if}
    </div>
  {/each}
  <p class="credit">
    Rates are what players saw, not the game's tables; small samples can be far off. "?" means listed but not
    measured.
    <button class="link" onclick={() => backend.openLink(WIKI_URL)}>{WIKI_CREDIT}</button>
  </p>
{:else}
  <p class="note">Drops aren't in the game data. Players log them on mapleclassic.wiki, with how many kills each rate is based on.</p>
  <button class="load" onclick={load} disabled={loading}>
    {#if loading}<span class="spin"><LoaderCircle size={13} /></span>Asking the wiki…{:else}<Download size={13} />Load
      player-observed drops{/if}
  </button>
  {#if error}<p class="error">{error}</p>{/if}
{/if}

<style>
  .monster + .monster {
    margin-top: 10px;
  }

  .head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 3px;
  }

  .name {
    padding: 0;
    border: 0;
    background: transparent;
    font-size: 12px;
    font-weight: 600;
    text-align: left;
    cursor: pointer;
  }

  .name:hover {
    color: var(--map);
    text-decoration: underline;
  }

  .few {
    margin: 4px 0 0;
    padding: 4px 7px;
    border-radius: var(--radius-sm);
    background: var(--warn-soft);
    color: var(--muted);
    font-size: 10.5px;
  }

  .sample {
    font-size: 10.5px;
    color: var(--faint);
    white-space: nowrap;
  }

  ul {
    display: grid;
    gap: 1px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li {
    display: grid;
    grid-template-columns: 40px 1fr auto;
    align-items: baseline;
    gap: 8px;
    font-size: 11.5px;
  }

  .rate {
    text-align: right;
    color: var(--accent);
    font-weight: 600;
  }

  .unmeasured .rate {
    color: var(--faint);
  }

  .item {
    min-width: 0;
  }

  .qty {
    color: var(--muted);
  }

  .tag {
    margin-left: 4px;
    padding: 0 4px;
    border-radius: 3px;
    background: var(--accent-soft);
    color: var(--accent);
    font-size: 9.5px;
  }

  .cat {
    font-size: 10px;
    color: var(--faint);
  }

  .note {
    margin: 0 0 8px;
    font-size: 11.5px;
    color: var(--faint);
  }

  .load {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 10px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: transparent;
    font-size: 12px;
    cursor: pointer;
  }

  .load:hover:not(:disabled) {
    background: var(--surface-3);
  }

  .load:disabled {
    color: var(--muted);
    cursor: default;
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
    margin: 10px 0 0;
    font-size: 10.5px;
    color: var(--faint);
  }

  .link {
    display: block;
    margin-top: 3px;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--muted);
    font-size: 10.5px;
    text-align: left;
    text-decoration: underline;
    text-decoration-color: var(--border-strong);
    cursor: pointer;
  }

  .error {
    margin: 6px 0 0;
    font-size: 11.5px;
    color: var(--bad);
  }
</style>
