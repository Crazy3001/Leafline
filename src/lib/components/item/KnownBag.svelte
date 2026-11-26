<script lang="ts">
  // The Me view's bag: each inventory tab Leafline has read for the character looked at, as it was
  // then, with its own age. This run's check of a tab, when it is theirs, stands in for the kept
  // one. For looking only; Sell or keep has the prices.
  import Card from "../Card.svelte";
  import Ref from "../Ref.svelte";
  import { bag, watchBag } from "$lib/bag.svelte";
  import { characters } from "$lib/characters.svelte";
  import { formatNumber } from "$lib/format";
  import { ago } from "$lib/profile.svelte";
  import Pic from "$lib/ui/Pic.svelte";
  import type { BagCheck, BagTab } from "$lib/types";

  watchBag();

  const TABS: Record<BagTab, string> = { equip: "Equip", use: "Use", setup: "Set-up", etc: "Etc", cash: "Cash", deco: "Deco" };
  const ORDER: BagTab[] = ["equip", "use", "setup", "etc", "cash", "deco"];

  const same = (a: string | null | undefined, b: string | null | undefined) =>
    a != null && b != null && a.trim().toLowerCase() === b.trim().toLowerCase();

  /** One check per tab, in the game's tab order: this run's when it is the character's, else the kept one. */
  const tabs = $derived.by(() => {
    const out = new Map<BagTab, { check: BagCheck; now: boolean }>();
    for (const k of bag.known) if (k.tab && !out.has(k.tab)) out.set(k.tab, { check: k, now: false });
    const run = bag.check;
    if (run?.tab && !run.busy && !run.problem && same(run.character, characters.active)) out.set(run.tab, { check: run, now: true });
    return ORDER.filter((t) => out.has(t)).map((t) => ({ tab: t, ...out.get(t)! }));
  });

  let picked = $state<BagTab | null>(null);
  const shown = $derived(tabs.find((t) => t.tab === picked) ?? tabs[0] ?? null);
</script>

{#if shown}
  <Card title="Bag">
    {#snippet aside()}
      <span class:stale={!shown.now}>{shown.now ? "read just now" : `as of ${ago(shown.check.at)}`}</span>
    {/snippet}
    <div class="tabs" role="group" aria-label="Inventory tabs read">
      {#each tabs as t (t.tab)}
        <button class:on={t === shown} aria-pressed={t === shown} onclick={() => (picked = t.tab)}
          >{TABS[t.tab]} <span class="muted">{t.now ? "now" : ago(t.check.at)}</span></button
        >
      {/each}
    </div>
    {#if shown.check.items.length}
      <ul class:stale={!shown.now}>
        {#each shown.check.items as item (item.slot)}
          <li>
            <span class="pic"><Pic kind="item" id={item.ids[0]} size={18} /></span>
            <span class="name"><Ref kind="item" id={item.ids[0]} name={item.name} icon={false} /></span>
            <span class="muted tabular">×{formatNumber(item.count)}</span>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="quiet">That tab was empty.</p>
    {/if}
    {#if shown.check.unknown.length}
      <p class="quiet">
        {shown.check.unknown.length} slot{shown.check.unknown.length === 1 ? "" : "s"} not recognised.
      </p>
    {/if}
    {#if !shown.now}<p class="quiet">Each tab as Leafline last read it. Open the inventory in game on a tab to read it again.</p>{/if}
  </Card>
{/if}

<style>
  /* Remembered from an earlier read, not read now. */
  .stale {
    opacity: 0.6;
  }

  .tabs {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1);
    margin-bottom: var(--space-2);
  }

  .tabs button {
    padding: var(--space-0-5) var(--space-2);
    border: var(--line-w) solid var(--line);
    border-radius: var(--radius-pill);
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: var(--text-xs);
    cursor: pointer;
  }

  .tabs button.on {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 14%, transparent);
  }

  ul {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(190px, 1fr));
    gap: var(--space-0-5) var(--space-3);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li {
    display: flex;
    align-items: center;
    gap: var(--space-1-5);
    min-width: 0;
    font-size: var(--text-md);
  }

  .pic {
    display: inline-grid;
    flex: none;
    place-items: center;
    width: 20px;
  }

  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .muted {
    color: var(--muted);
  }

  .quiet {
    margin: var(--space-2) 0 0;
    color: var(--muted);
    font-size: var(--text-sm);
  }

  .tabular {
    font-variant-numeric: tabular-nums;
  }
</style>
