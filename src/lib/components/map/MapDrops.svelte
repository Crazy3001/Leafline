<script lang="ts">
  // Every known drop of this map's monsters, one line each: by item, grouped by kind, with the
  // monsters that drop it as small pictures; or by monster, each with its items. The first open
  // downloads Leafline's drop list if it isn't here yet; lookups stay on this PC.
  import IconRef from "$lib/components/IconRef.svelte";
  import Ref from "$lib/components/Ref.svelte";
  import { LoaderCircle, RefreshCw } from "@lucide/svelte";

  import { tip } from "$lib/tip.svelte";
  import Pic from "$lib/ui/Pic.svelte";
  import { untrack } from "svelte";

  import { backend } from "$lib/api";
  import Segmented from "$lib/ui/Segmented.svelte";
  import type { MonsterDrops } from "$lib/types";

  type Drop = MonsterDrops["drops"][number];

  /** One item on this map, and each monster seen dropping it. */
  interface MapItem {
    name: string;
    id: number | null;
    type: string | null;
    level: number | null;
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

  function setView(next: View) {
    view = next;
    try {
      localStorage.setItem(VIEW_KEY, next);
    } catch {
      // A convenience only.
    }
  }

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
          item = { name: drop.item, id: drop.itemId, type: drop.itemType, level: drop.reqLevel, category: drop.category, quest: false, sources: [] };
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

  /** The kinds items are grouped under, in this order; anything else goes last as Other. */
  const KINDS: { id: string; label: string }[] = [
    { id: "equip", label: "Equip" },
    { id: "use", label: "Use" },
    { id: "scroll", label: "Scrolls" },
    { id: "setup", label: "Set-up" },
    { id: "etc", label: "Etc" },
  ];

  const isMeso = (item: { name?: string; item?: string; category: string | null }) => item.category === "meso";

  /** The items under quiet headings, mesos apart; a heading with nothing under it is left out. */
  const groups = $derived.by(() => {
    const known = new Set(KINDS.map((k) => k.id));
    const list = items.filter((i) => !isMeso(i));
    const out = KINDS.map((k) => ({ label: k.label, items: list.filter((i) => i.category === k.id) }));
    out.push({ label: "Other", items: list.filter((i) => !i.category || !known.has(i.category)) });
    return out.filter((g) => g.items.length);
  });
  const mesos = $derived(items.find(isMeso) ?? null);

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

  /** A dropper's picture says nothing alone: its name, and for mesos how many. */
  function dropperTip(source: MapItem["sources"][number], meso = false) {
    const amount = meso ? quantity(source).replace(" ×", "") : "";
    return meso && amount ? `${source.monster}\n${amount} mesos` : source.monster;
  }

  function percent(value: number | null) {
    if (value == null) return "";
    return value >= 10 ? `${value.toFixed(0)}%` : `${value.toFixed(1)}%`;
  }


  function quantity(drop: Drop) {
    if (drop.minQuantity == null || drop.maxQuantity == null) return "";
    return ` ×${drop.minQuantity === drop.maxQuantity ? drop.minQuantity : `${drop.minQuantity}–${drop.maxQuantity}`}`;
  }
</script>

{#if drops}
  {#if items.length}
    <div class="view"><Segmented label="Show drops" size="sm" options={VIEWS} value={view} onchange={setView} /></div>
  {/if}
  {#if items.length && view === "monster"}
    <ul class="monsters">
      {#each byMonster as entry (entry.monster)}
        <li>
          <div class="mob-head" class:empty={!entry.drops.length}>
            <Ref kind="mob" name={entry.monster} class="mob-name" />
            <span class="summary tabular">{entry.drops.length ? entry.summary : "nothing logged yet"}</span>
          </div>
          {#if entry.drops.some((d) => !isMeso(d))}
            <ul class="wrap">
              {#each entry.drops.filter((d) => !isMeso(d)) as drop (drop.item)}
                <li>
                  <Ref kind="item" name={drop.item} />{#if drop.quest}<span class="tag">quest</span>{/if}
                </li>
              {/each}
            </ul>
          {/if}
        </li>
      {/each}
    </ul>
  {:else if items.length}
    {#if mesos}
      <p class="meso-line">
        <span class="mesos">Mesos</span>
        <span class="droppers">
          {#each mesos.sources as source (source.monster)}<IconRef kind="mob" name={source.monster} title={dropperTip(source, true)} />{/each}
        </span>
      </p>
    {/if}
    {#each groups as group (group.label)}
      <h4 class="kind">{group.label} <span class="tabular">{group.items.length}</span></h4>
      <ul class="items">
        {#each group.items as item (item.name)}
          <li>
            <span class="item" use:tip={itemTip}
              ><Ref kind="item" id={item.id} name={item.name} />{#if item.quest}<span class="tag">quest</span>{/if}</span
            >
            <span class="droppers">
              {#each item.sources as source (source.monster)}<IconRef kind="mob" name={source.monster} title={dropperTip(source)} />{/each}
            </span>
          </li>
          {#snippet itemTip()}
            <span class="tip-head">
              <Pic kind="item" id={item.id} name={item.name} size={32} />
              <span>
                <span class="tip-title">{item.name}</span>
                <span class="tip-sub">{[item.type ?? group.label, item.level ? `REQ LEV ${item.level}` : null, item.quest ? "quest only" : null].filter(Boolean).join(" · ")}</span>
              </span>
            </span>
            <ul class="tip-list">
              {#each item.sources as source (source.monster)}
                <li><Pic kind="mob" name={source.monster} size={16} />{source.monster}</li>
              {/each}
            </ul>
          {/snippet}
        {/each}
      </ul>
    {/each}
  {:else}
    <p class="note">No drops logged for this map's monsters yet.</p>
  {/if}
  {#if items.length && unlogged.length && view === "item"}
    <p class="note none">Nothing logged yet for {unlogged.join(", ")}.</p>
  {/if}

{:else if error}
  <p class="error">{error}</p>
  <button class="load" onclick={load} disabled={loading}><RefreshCw size={13} />Try again</button>
{:else}
  <p class="note"><span class="spin"><LoaderCircle size={13} /></span> Loading drops…</p>
{/if}

<style>
  /* A quiet heading over each kind of item, so no row has to say what it is. */
  .kind {
    display: flex;
    align-items: baseline;
    gap: 6px;
    margin: 10px 0 2px;
    color: var(--faint);
    font-family: var(--font-display);
    font-size: var(--text-xs);
    font-weight: 600;
    letter-spacing: 0.05em;
    text-transform: uppercase;
  }

  .kind span {
    font-weight: 400;
    letter-spacing: 0;
  }

  .items {
    display: grid;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  /* One line an item: its picture and name, then who drops it as small pictures. */
  .items > li,
  .meso-line {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 30px;
    padding: 2px 0;
  }

  .items > li + li {
    border-top: 1px solid var(--border);
  }

  .item {
    display: flex;
    flex: 1;
    align-items: center;
    gap: 4px;
    min-width: 0;
    overflow: hidden;
    font-size: var(--text-sm);
    white-space: nowrap;
  }

  .item :global(.ref) {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .droppers {
    display: flex;
    flex: none;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 2px;
    max-width: 55%;
  }

  .meso-line {
    margin: 0;
  }

  .meso-line .mesos {
    flex: 1;
    font-size: var(--text-sm);
    font-weight: 600;
  }

  .mesos {
    color: var(--mesos);
  }

  .view {
    margin-bottom: 6px;
  }

  .monsters {
    display: grid;
    gap: 8px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .monsters > li + li {
    padding-top: 8px;
    border-top: 1px solid var(--border);
  }

  .mob-head {
    display: flex;
    align-items: center;
    gap: 6px;
    min-height: 26px;
  }

  .mob-head :global(.ref.mob-name) {
    font-size: var(--text-sm);
  }

  .empty .summary {
    color: var(--faint);
  }

  .summary {
    margin-left: auto;
    color: var(--muted);
    font-size: var(--text-xs);
  }

  /* A monster's items as a compact wrap of picture and name. */
  .wrap {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 12px;
    margin: 2px 0 0;
    padding: 0 0 0 4px;
    list-style: none;
    font-size: var(--text-sm);
  }

  .wrap li {
    display: inline-flex;
    align-items: center;
    min-width: 0;
  }

  .tag {
    margin-left: 4px;
    padding: 0 4px;
    border-radius: var(--radius-sm);
    background: var(--surface-3);
    color: var(--muted);
    font-size: var(--text-xs);
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

  .error {
    margin: 0 0 6px;
    font-size: var(--text-sm);
    color: var(--bad);
  }
</style>
