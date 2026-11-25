<script lang="ts">
  // Sell or keep: the inventory tab checked last, each item marked to keep (with why) or to sell
  // to an NPC, and what the bag fetches at NPC prices. Items are named by their pictures.
  import { tip } from "$lib/tip.svelte";
  import { LoaderCircle, X } from "@lucide/svelte";

  import Card from "../Card.svelte";
  import Ref from "../Ref.svelte";
  import { backend } from "$lib/api";
  import { bag, keepReasons, stackValue, watchBag } from "$lib/bag.svelte";
  import { formatAgo, formatNumber } from "$lib/format";
  import { display, hotkey } from "$lib/settings.svelte";
  import Button from "$lib/ui/Button.svelte";
  import Pic from "$lib/ui/Pic.svelte";
  import type { BagItem, BagTab } from "$lib/types";

  let { plain = false }: { plain?: boolean } = $props();

  watchBag();

  const TABS: Record<BagTab, string> = { equip: "Equip", use: "Use", setup: "Set-up", etc: "Etc", cash: "Cash", deco: "Deco" };

  const check = $derived(bag.check);
  const rows = $derived((check?.items ?? []).map((item) => ({ item, why: keepReasons(item), value: stackValue(item) })));
  const keep = $derived(rows.filter((r) => r.why.length));
  const sell = $derived(rows.filter((r) => !r.why.length));
  const total = (list: typeof rows) => list.reduce((a, r) => a + (r.value ?? 0), 0);
  const worth = $derived(total(rows));
  const sellWorth = $derived(total(sell));
  const roughly = $derived(rows.some((r) => r.item.priceRange != null));
  let asking = $state(false);

  function lookFurther() {
    asking = true;
    backend
      .bagLookFurther()
      .catch(() => {})
      .finally(() => (asking = false));
  }

  const lookAlikes = (item: BagItem) =>
    item.ids.length > 1
      ? `The game draws ${item.ids.length} items with this picture. Hover it in game to see which.`
      : undefined;
</script>

{#if check}
  <Card title="Sell or keep" {plain}>
    {#snippet aside()}
      <span class="aside">
        {#if check.tab}{TABS[check.tab]} tab ·{/if}
        {formatAgo(Date.now() / 1000 - check.at)}
        <button class="icon" aria-label="Close the bag check" use:tip={"Close"} onclick={() => (bag.check = null)}><X size={12} /></button>
      </span>
    {/snippet}

    {#if check.busy}
      <p class="quiet"><span class="spin"><LoaderCircle size={13} /></span> Checking your bag… The first check downloads item pictures.</p>
    {/if}
    {#if check.problem}
      <p class="warn">{check.problem}</p>
    {:else if rows.length || check.unknown.length}
      <div class="sum">
        <span><span class="muted">Bag at NPC prices</span> <strong class="tabular">{roughly ? "at least " : ""}{formatNumber(worth)}</strong></span>
        <span><span class="muted">Selling the rest fetches</span> <strong class="tabular">{formatNumber(sellWorth)}</strong></span>
      </div>

      {#if sell.length}
        <h4>Sell to an NPC</h4>
        <ul>
          {#each sell as { item, value } (item.slot)}
            <li>
              <span class="pic"><Pic kind="item" id={item.ids[0]} size={18} /></span>
              <span class="name" use:tip={lookAlikes(item)}
                ><Ref kind="item" id={item.ids[0]} name={item.name} icon={false} />{#if item.ids.length > 1}<span class="muted">
                    or look-alike</span
                  >{/if}</span
              >
              <span class="muted tabular">×{formatNumber(item.count)}</span>
              <span class="tabular value">{value != null ? formatNumber(value) : "—"}</span>
            </li>
          {/each}
        </ul>
      {/if}

      {#if keep.length}
        <h4>Keep</h4>
        <ul>
          {#each keep as { item, why, value } (item.slot)}
            <li>
              <span class="pic"><Pic kind="item" id={item.ids[0]} size={18} /></span>
              <span class="name" use:tip={lookAlikes(item)}
                ><Ref kind="item" id={item.ids[0]} name={item.name} icon={false} />{#if item.ids.length > 1}<span class="muted">
                    or look-alike</span
                  >{/if}<span class="why">{why.join(", ")}</span></span
              >
              <span class="muted tabular">×{formatNumber(item.count)}</span>
              <span class="tabular value muted">{value != null ? formatNumber(value) : "—"}</span>
            </li>
          {/each}
        </ul>
      {/if}

      {#if check.unknown.length}
        <p class="quiet">
          {check.unknown.length} slot{check.unknown.length === 1 ? "" : "s"} not recognised{check.wide ? "." : ": Leafline tried the items most likely at your level."}
        </p>
        {#if !check.wide && check.more > 0}
          <Button size="sm" variant="ghost" onclick={lookFurther} disabled={asking || check.busy}>
            Look further (downloads up to {formatNumber(check.more)} pictures)
          </Button>
        {/if}
      {/if}
      <p class="caption">
        Keep: what a quest you're doing or follow asks for, what crafting uses, gear your build will wear, tracked items and
        scrolls. NPC prices are from the item data and not yet checked in game. Leafline only shows this: selling is up to you.
      </p>
    {:else if !check.busy}
      <p class="quiet">That tab is empty.</p>
    {/if}
  </Card>
{:else}
  <p class="hint">
    {#if display.live.bagCheck}
      Open your Item Inventory on a tab and hold it still while Leafline reads the game{hotkey("CheckBag")
        ? `, or press ${hotkey("CheckBag")}`
        : ""}, to see what to keep and what to sell.
    {:else}
      Open your Item Inventory and press {hotkey("CheckBag") ? hotkey("CheckBag") : "the Sell or keep hotkey (set it in Settings)"} to see what
      to keep and what to sell.
    {/if}
  </p>
{/if}

<style>
  .aside {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .icon {
    display: inline-grid;
    padding: 2px;
    border: 0;
    background: transparent;
    color: var(--faint);
    cursor: pointer;
  }

  .icon:hover {
    color: var(--text);
  }

  .sum {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 16px;
    font-size: var(--text-md);
  }

  .sum strong {
    font-weight: var(--weight-strong);
  }

  h4 {
    margin: 10px 0 4px;
    color: var(--muted);
    font-size: var(--text-xs);
    font-weight: var(--weight-strong);
  }

  ul {
    display: flex;
    flex-direction: column;
    gap: 3px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li {
    display: grid;
    grid-template-columns: 18px 1fr auto 64px;
    align-items: center;
    gap: 8px;
    font-size: var(--text-md);
  }

  .pic {
    display: inline-grid;
    width: 18px;
  }

  .name {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 2px 6px;
    min-width: 0;
  }

  .why {
    font-size: var(--text-xs);
    color: var(--good);
  }

  .value {
    text-align: right;
  }

  .muted {
    color: var(--muted);
  }

  .quiet,
  .hint {
    margin: 6px 0 0;
    font-size: var(--text-md);
    color: var(--faint);
  }

  .hint {
    margin: 10px 0 0;
    font-size: var(--text-xs);
  }

  .warn {
    margin: 0;
    font-size: var(--text-md);
    color: var(--warn);
  }

  .caption {
    margin: 8px 0 0;
    font-size: var(--text-xs);
    color: var(--faint);
  }

  .spin {
    display: inline-grid;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
