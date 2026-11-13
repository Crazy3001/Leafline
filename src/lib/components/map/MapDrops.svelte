<script lang="ts">
  // Every known drop of this map's monsters: by item, with who drops it, or by monster, each
  // opening to its own drops. The first open downloads Leafline's drop list if it isn't here yet;
  // lookups stay on this PC.
  import Ref from "$lib/components/Ref.svelte";
  import { ChevronRight, LoaderCircle, RefreshCw } from "@lucide/svelte";
  import { untrack } from "svelte";

  import { backend } from "$lib/api";
  import Segmented from "$lib/ui/Segmented.svelte";
  import { showItemNamed } from "$lib/items.svelte";
  import { DROPS_CREDIT, DROPS_URL } from "$lib/mapdata.svelte";
  import { showMonsterNamed } from "$lib/monsters.svelte";
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

  type View = "item" | "monster";
  const VIEW_KEY = "leafline.map.dropsView";
  const VIEWS: { id: View; label: string }[] = [
    { id: "item", label: "By item" },
    { id: "monster", label: "By monster" },
  ];

  function rememberedView(): View {
    try {
      return localStorage.getItem(VIEW_KEY) === "monster" ? "monster" : "item";
    } catch {
      return "item";
    }
  }

  let view = $state<View>(rememberedView());
  /** Monsters opened in the by-monster view. */
  let opened = $state<string[]>([]);

  function setView(next: View) {
    view = next;
    try {
      localStorage.setItem(VIEW_KEY, next);
    } catch {
      // A convenience only.
    }
  }

  function toggle(monster: string) {
    opened = opened.includes(monster) ? opened.filter((m) => m !== monster) : [...opened, monster];
  }

  let drops = $state<MonsterDrops[] | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);

  // A different map's monsters: forget the old list.
  $effect(() => {
    void monsters.join("|");
    drops = null;
    error = null;
    opened = [];
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

  /** Each monster with its drops, the best rate first, in the map's own order. */
  const byMonster = $derived(
    (drops ?? []).map((entry) => {
      const sorted = [...entry.drops].sort((a, b) => (b.rate ?? -1) - (a.rate ?? -1));
      const meso = sorted.find((d) => d.category === "meso");
      const items = sorted.filter((d) => d.category !== "meso").length;
      const summary = [
        items ? `${items} item${items === 1 ? "" : "s"}` : null,
        meso && meso.minQuantity != null ? `${quantity(meso).slice(2)} mesos` : null,
      ].filter(Boolean);
      return { ...entry, drops: sorted, summary: summary.join(" · ") };
    }),
  );

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
    if (value == null) return "";
    return value >= 10 ? `${value.toFixed(0)}%` : `${value.toFixed(1)}%`;
  }

  /** Below this many kills, drops rarer than about 1 in N are likely missing from the log. */
  const FEW_KILLS = 200;

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
    <div class="view"><Segmented label="Show drops" size="sm" options={VIEWS} value={view} onchange={setView} /></div>
  {/if}
  {#if items.length && view === "monster"}
    <ul class="monsters">
      {#each byMonster as entry (entry.monster)}
        {@const open = opened.includes(entry.monster)}
        <li>
          {#if entry.drops.length}
            <!-- The name opens the monster in Lookup; the rest of the row opens its drops. -->
            <div class="mob-head">
              <button class="chevron-button" aria-expanded={open} aria-label="Drops of {entry.monster}" onclick={() => toggle(entry.monster)}>
                <span class="chevron" class:open><ChevronRight size={12} /></span>
              </button>
              <Ref kind="mob" name={entry.monster} class="mob-name" />
              <button class="row-rest" tabindex="-1" onclick={() => toggle(entry.monster)}>
                <span class="summary tabular">{entry.summary}</span>
              </button>
            </div>
            {#if open}
              <ul class="mob-drops">
                {#each entry.drops as drop (drop.item)}
                  <li class:unmeasured={drop.rate == null} title={evidence(drop)}>
                    {#if drop.category === "meso"}
                      <span class="name mesos">{drop.item}</span>
                    {:else}
                      <Ref kind="item" name={drop.item} class="name" />
                    {/if}
                    {#if drop.quest}<span class="tag">quest</span>{/if}
                    <span class="rate tabular">{percent(drop.rate)}{quantity(drop)}</span>
                  </li>
                {/each}
                {#if entry.sampleKills != null && entry.sampleKills < FEW_KILLS}
                  <li class="card-link faint">Only {entry.sampleKills} kills logged</li>
                {/if}
              </ul>
            {/if}
          {:else}
            <div class="mob-head empty">
              <span class="chevron"></span>
              <span class="mob-name">{entry.monster}</span>
              <span class="summary">nothing logged yet</span>
            </div>
          {/if}
        </li>
      {/each}
    </ul>
  {:else if items.length}
    <ul class="items">
      {#each items as item (item.name)}
        <li>
          <div class="item">
            {#if item.category === "meso"}
              <span class="name mesos">{item.name}</span>
            {:else}
              <Ref kind="item" name={item.name} class="name" />
            {/if}
            {#if item.quest}<span class="tag">quest</span>{/if}
            {#if item.category}<span class="cat">{item.category}</span>{/if}
          </div>
          <div class="sources">
            {#each item.sources as source (source.monster)}
              <span class="source" class:unmeasured={source.rate == null} title={evidence(source)}>
                <Ref kind="mob" name={source.monster} class="monster" />
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
  {#if items.length && unlogged.length && view === "item"}
    <p class="note none">Nothing logged yet for {unlogged.join(", ")}.</p>
  {/if}
  {#if fewKills.length && view === "item"}
    <p class="few">
      Few kills logged for {fewKills.map((e) => `${e.monster} (${e.sampleKills})`).join(", ")}, so rarer drops, such as
      most equips and potions, may not show up yet.
    </p>
  {/if}
  <p class="credit">
    Known drops, without rates.
    <button class="link" onclick={() => backend.openLink(DROPS_URL)}>{DROPS_CREDIT}</button>
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
    margin: 0;
    padding: 0;
    list-style: none;
  }

  /* Each drop ruled off from the next, so its name and the monsters under it read as one entry. */
  .items > li {
    padding: 5px 0;
  }

  .items > li + li {
    border-top: 1px solid var(--border);
  }

  .item {
    display: flex;
    align-items: baseline;
    gap: 4px;
    font-size: var(--text-sm);
  }

  .name {
    min-width: 0;
    font-size: var(--text-sm);
    font-weight: 600;
  }

  .sources {
    display: flex;
    flex-wrap: wrap;
    gap: 0 10px;
    font-size: var(--text-xs);
  }

  .source {
    display: inline-flex;
    align-items: baseline;
    gap: 4px;
  }

  .mesos {
    color: var(--mesos);
  }

  .sources :global(.ref.monster) {
    font-size: var(--text-xs);
  }

  .rate {
    color: var(--text);
    font-weight: 600;
  }

  .view {
    margin-bottom: 6px;
  }

  .monsters,
  .mob-drops {
    display: grid;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .monsters > li + li {
    border-top: 1px solid var(--border);
  }

  .mob-head {
    display: flex;
    align-items: center;
    gap: 5px;
    width: 100%;
    min-height: 28px;
    padding: 4px 0;
    border: 0;
    background: transparent;
    text-align: left;
    cursor: pointer;
  }

  .mob-head.empty {
    cursor: default;
  }

  .chevron-button,
  .row-rest {
    display: flex;
    align-items: center;
    padding: 0;
    border: 0;
    background: transparent;
    cursor: pointer;
  }

  .row-rest {
    flex: 1;
    align-self: stretch;
  }

  .chevron {
    display: grid;
    width: 12px;
    color: var(--faint);
    transition: transform 0.15s ease;
  }

  .chevron.open {
    transform: rotate(90deg);
  }

  .mob-name {
    font-size: var(--text-sm);
    font-weight: 600;
  }

  .empty .mob-name,
  .empty .summary {
    color: var(--faint);
    font-weight: 400;
  }

  .summary {
    margin-left: auto;
    color: var(--muted);
    font-size: var(--text-xs);
  }

  .mob-drops {
    gap: 2px;
    padding: 0 0 7px 17px;
  }

  .mob-drops > li {
    display: flex;
    align-items: baseline;
    gap: 4px;
  }

  .mob-drops .rate {
    margin-left: auto;
    font-size: var(--text-xs);
  }

  .card-link {
    margin-top: 2px;
    font-size: var(--text-xs);
  }

  .faint {
    color: var(--faint);
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
    font-size: var(--text-xs);
  }

  .tag {
    margin-left: 4px;
    padding: 0 4px;
    border-radius: var(--radius-sm);
    background: var(--surface-3);
    color: var(--muted);
    font-size: var(--text-xs);
  }

  .cat {
    margin-left: auto;
    font-size: var(--text-xs);
    color: var(--faint);
  }

  .note {
    margin: 0;
    font-size: var(--text-sm);
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
    font-size: var(--text-sm);
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
    font-size: var(--text-xs);
    color: var(--faint);
  }

  .link {
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

  .error {
    margin: 0 0 6px;
    font-size: var(--text-sm);
    color: var(--bad);
  }
</style>
