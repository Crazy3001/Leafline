<script lang="ts">
  // Every drop players logged on mapleclassic.wiki for this map's monsters, by item, with who
  // drops it. The first open downloads the wiki's whole table; lookups stay on this PC.
  import { LoaderCircle, RefreshCw } from "@lucide/svelte";
  import { untrack } from "svelte";

  import { backend } from "$lib/api";
  import { showItemNamed } from "$lib/items.svelte";
  import { WIKI_CREDIT, WIKI_URL } from "$lib/mapdata.svelte";
  import type { MonsterDrops } from "$lib/types";

  type Drop = MonsterDrops["drops"][number];

  /** One item on this map, and each monster seen dropping it. */
  interface MapItem {
    name: string;
    category: string | null;
    quest: boolean;
    sources: (Drop & { monster: string })[];
  }

  let { monsters }: { monsters: string[] } = $props();

  let drops = $state<MonsterDrops[] | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);

  // A different map's monsters: forget the old list.
  $effect(() => {
    void monsters.join("|");
    drops = null;
    error = null;
    untrack(load);
  });

  /** The best rate first; items only listed, with no rate, last. */
  const items = $derived.by(() => {
    const byName = new Map<string, MapItem>();
    for (const entry of drops ?? []) {
      for (const drop of entry.drops) {
        let item = byName.get(drop.item);
        if (!item) {
          item = { name: drop.item, category: drop.category, quest: false, sources: [] };
          byName.set(drop.item, item);
        }
        item.quest ||= drop.quest;
        item.sources.push({ ...drop, monster: entry.monster });
      }
    }
    const best = (item: MapItem) => item.sources[0].rate ?? -1;
    const list = [...byName.values()];
    for (const item of list) item.sources.sort((a, b) => (b.rate ?? -1) - (a.rate ?? -1));
    return list.sort((a, b) => best(b) - best(a));
  });

  const unlogged = $derived((drops ?? []).filter((e) => !e.drops.length).map((e) => e.monster));
  const fewKills = $derived((drops ?? []).filter((e) => e.sampleKills != null && e.sampleKills < FEW_KILLS));

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

  /** Below this many kills, drops rarer than about 1 in N are likely missing from the log. */
  const FEW_KILLS = 200;

  function wikiPage(monster: string) {
    return `${WIKI_URL}/index.php?search=${encodeURIComponent(monster)}&go=Go`;
  }

  function quantity(drop: Drop) {
    if (drop.minQuantity == null || drop.maxQuantity == null) return "";
    return ` ×${drop.minQuantity === drop.maxQuantity ? drop.minQuantity : `${drop.minQuantity}–${drop.maxQuantity}`}`;
  }

  /** How solid a rate is, for the tooltip. */
  function evidence(drop: Drop) {
    return [
      drop.rateLow != null && drop.rateHigh != null ? `Likely between ${drop.rateLow}% and ${drop.rateHigh}%` : null,
      drop.sampleKills ? `${drop.sampleKills.toLocaleString()} kills logged` : null,
      drop.gameVersion && `v${drop.gameVersion}`,
    ]
      .filter(Boolean)
      .join(" · ");
  }
</script>

{#if drops}
  {#if items.length}
    <ul class="items">
      {#each items as item (item.name)}
        <li>
          <div class="item">
            <button class="name" title="Show it in the Item tab" onclick={() => showItemNamed(item.name)}>{item.name}</button>
            {#if item.quest}<span class="tag">quest</span>{/if}
            {#if item.category}<span class="cat">{item.category}</span>{/if}
          </div>
          <div class="sources">
            {#each item.sources as source (source.monster)}
              <span class="source" class:unmeasured={source.rate == null} title={evidence(source)}>
                <button
                  class="monster"
                  title="Open {source.monster} on mapleclassic.wiki"
                  onclick={() => backend.openLink(wikiPage(source.monster))}>{source.monster}</button
                >
                <span class="rate tabular">{percent(source.rate)}{quantity(source)}</span>
              </span>
            {/each}
          </div>
        </li>
      {/each}
    </ul>
  {:else}
    <p class="note">No drops logged for this map's monsters yet.</p>
  {/if}
  {#if items.length && unlogged.length}
    <p class="note none">Nothing logged yet for {unlogged.join(", ")}.</p>
  {/if}
  {#if fewKills.length}
    <p class="few">
      Few kills logged for {fewKills.map((e) => `${e.monster} (${e.sampleKills})`).join(", ")}, so rarer drops, such as
      most equips and potions, may not show up yet.
    </p>
  {/if}
  <p class="credit">
    Rates are what players saw, not the game's tables; small samples can be far off. "?" means listed but not
    measured.
    <button class="link" onclick={() => backend.openLink(WIKI_URL)}>{WIKI_CREDIT}</button>
  </p>
{:else if error}
  <p class="error">{error}</p>
  <button class="load" onclick={load} disabled={loading}><RefreshCw size={13} />Try again</button>
{:else}
  <p class="note"><span class="spin"><LoaderCircle size={13} /></span> Loading drops…</p>
{/if}

<style>
  .items {
    display: grid;
    gap: 7px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .item {
    display: flex;
    align-items: baseline;
    gap: 4px;
    font-size: 12px;
  }

  .name {
    min-width: 0;
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

  .sources {
    display: flex;
    flex-wrap: wrap;
    gap: 0 10px;
    font-size: 11px;
  }

  .source {
    display: inline-flex;
    align-items: baseline;
    gap: 4px;
  }

  .monster {
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--muted);
    font-size: 11px;
    text-align: left;
    cursor: pointer;
  }

  .monster:hover {
    color: var(--map);
    text-decoration: underline;
  }

  .rate {
    color: var(--accent);
    font-weight: 600;
  }

  .unmeasured .rate {
    color: var(--faint);
  }

  .few {
    margin: 8px 0 0;
    padding: 4px 7px;
    border-radius: var(--radius-sm);
    background: var(--warn-soft);
    color: var(--muted);
    font-size: 10.5px;
  }

  .tag {
    margin-left: 4px;
    padding: 0 4px;
    border-radius: var(--radius-sm);
    background: var(--accent-soft);
    color: var(--accent);
    font-size: 9.5px;
  }

  .cat {
    margin-left: auto;
    font-size: 10px;
    color: var(--faint);
  }

  .note {
    margin: 0;
    font-size: 11.5px;
    color: var(--faint);
  }

  .none {
    margin-top: 8px;
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
    margin: 0 0 6px;
    font-size: 11.5px;
    color: var(--bad);
  }
</style>
