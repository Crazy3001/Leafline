<script lang="ts">
  // The player's own price records for an item, and a form to add one. There is no market data:
  // every number here is something the player logged. Market sites only open in the browser;
  // their terms bar reading their prices into an app without written permission. A price lookup
  // can sit beside this once a price source gives permission.
  import { ExternalLink, Plus, X } from "@lucide/svelte";
  import { untrack } from "svelte";

  import { backend } from "$lib/api";
  import { formatDay, formatMesos, formatNumber, parsePrice } from "$lib/format";
  import type { ItemTooltip, PriceKind, PriceLog, PriceStats } from "$lib/types";

  let { item }: { item: ItemTooltip } = $props();

  const SHOWN = 3;
  const MARKETS = [
    { label: "MeowDB", url: "https://meowdb.com/msclassic/price-check" },
    { label: "meso.watch", url: "https://meso.watch" },
  ];

  let log = $state<PriceLog | null>(null);
  let adding = $state(false);
  let all = $state(false);
  let priceText = $state("");
  let kind = $state<PriceKind>("traded");
  let note = $state("");
  let error = $state<string | null>(null);
  let saving = $state(false);
  let input = $state<HTMLInputElement>();

  const key = $derived(`${item.name}|${item.upgrades}|${item.remainingEnhancements}`);
  const stats = $derived(item.stats.map((s) => `${s.label} ${s.value >= 0 ? "+" : ""}${s.value}`).join(", "));
  const variant = $derived(
    [
      (item.upgrades ?? 0) > 0 ? `+${item.upgrades}` : item.plain ? null : item.upgraded ? "upgraded" : "no upgrades",
      slotsText(item.remainingEnhancements),
    ]
      .filter(Boolean)
      .join(", "),
  );
  const parsed = $derived(parsePrice(priceText));

  // Follows the key alone: a live reading of the same item must not close the form being typed in.
  $effect(() => {
    const asked = key;
    log = null;
    adding = false;
    untrack(() => {
      if (!item.name) return;
      backend
        .priceLog(item.name, item.upgrades, item.remainingEnhancements)
        .then((result) => {
          if (asked === key) log = result;
        })
        .catch((e) => (error = String(e)));
    });
  });

  $effect(() => {
    if (adding) input?.focus();
  });

  /** Opens a market site with the item's name ready to paste into its search. */
  async function openMarket(url: string) {
    try {
      await navigator.clipboard.writeText(item.name);
    } catch {
      // The site still opens; the name can be typed in.
    }
    await backend.openLink(url);
  }

  function slotsText(slots: number | null) {
    return slots == null ? "" : `${slots} slot${slots === 1 ? "" : "s"} left`;
  }

  function range(s: PriceStats) {
    return s.low === s.high ? formatMesos(s.low) : `${formatMesos(s.low)}–${formatMesos(s.high)}`;
  }

  function records(s: PriceStats) {
    return `${s.count} record${s.count === 1 ? "" : "s"}`;
  }

  function open() {
    adding = true;
    error = null;
  }

  async function save(event: SubmitEvent) {
    event.preventDefault();
    if (parsed == null) {
      error = "Enter a price such as 85000, 85,000 or 85k.";
      return;
    }
    saving = true;
    error = null;
    try {
      log = await backend.recordPrice({
        item: item.name,
        upgrades: item.upgrades,
        slots: item.remainingEnhancements,
        stats,
        price: parsed,
        kind,
        note: note.trim() || null,
      });
      adding = false;
      priceText = "";
      note = "";
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }

  async function remove(id: number) {
    await backend.deletePrice(id);
    log = await backend.priceLog(item.name, item.upgrades, item.remainingEnhancements);
  }

  function onKey(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.stopPropagation();
      adding = false;
    }
  }
</script>

<div class="price">
  <div class="head">
    <span class="eyebrow">Your prices</span>
    {#if item.name && !adding}
      <button class="add" onclick={open}><Plus size={12} /> Log a price</button>
    {/if}
  </div>

  {#if !item.name}
    <p class="muted">The name wasn't read, so prices can't be matched.</p>
  {:else if log?.all}
    <div class="summary">
      <span class="median tabular">{formatMesos(log.all.median)}</span>
      <span class="label">
        {log.all.count === 1 ? "1 record" : `median of ${log.all.count} records`}
      </span>
    </div>
    {#if log.all.count > 1}
      <p class="spread tabular">
        {range(log.all)} · {formatDay(log.all.firstAt)}{formatDay(log.all.firstAt) !== formatDay(log.all.lastAt) ? ` – ${formatDay(log.all.lastAt)}` : ""}
      </p>
    {/if}
    {#if log.likeThis && log.likeThis.count !== log.all.count}
      <p class="like tabular">
        Same {variant}: <b>{formatMesos(log.likeThis.median)}</b>
        <span class="muted">({log.likeThis.count === 1 ? "1 record" : `${log.likeThis.count} records, ${range(log.likeThis)}`})</span>
      </p>
    {:else if !log.likeThis}
      <p class="like muted">None logged with {variant}.</p>
    {/if}

    <ul class="history">
      {#each all ? log.entries : log.entries.slice(0, SHOWN) as entry (entry.id)}
        <li>
          <span class="day tabular">{formatDay(entry.recordedAt)}</span>
          <span class="amount tabular">{formatNumber(entry.price)}</span>
          <span class="kind" class:asking={entry.kind === "asking"}>{entry.kind === "traded" ? "traded" : "asking"}</span>
          <span class="what" title={[entry.stats, entry.note].filter(Boolean).join(" · ")}>
            {[(entry.upgrades ?? 0) > 0 ? `+${entry.upgrades}` : null, slotsText(entry.slots), entry.note].filter(Boolean).join(" · ")}
          </span>
          <button class="remove" title="Delete this record" aria-label="Delete this record" onclick={() => remove(entry.id)}
            ><X size={11} /></button
          >
        </li>
      {/each}
    </ul>
    {#if log.entries.length > SHOWN}
      <button class="link" onclick={() => (all = !all)}>{all ? "Show fewer" : `Show all ${log.entries.length}`}</button>
    {/if}
  {:else if log}
    <p class="muted">No prices logged for this item yet.</p>
  {/if}

  {#if item.name}
    <div class="markets">
      <span class="muted">Look it up on</span>
      {#each MARKETS as market (market.label)}
        <button
          title="Opens {market.label} in your browser and copies the item's name, ready to paste"
          onclick={() => openMarket(market.url)}>{market.label}<ExternalLink size={11} /></button
        >
      {/each}
    </div>
  {/if}

  {#if adding}
    <form class="form" onsubmit={save}>
      <div class="fields">
        <label class="amount-field">
          <span class="sr">Price in mesos</span>
          <input
            bind:this={input}
            bind:value={priceText}
            onkeydown={onKey}
            inputmode="decimal"
            placeholder="85k"
            autocomplete="off"
          />
          <span class="unit">{parsed != null ? formatNumber(parsed) : "mesos"}</span>
        </label>
        <div class="kinds" role="radiogroup" aria-label="Kind of price">
          <button type="button" role="radio" aria-checked={kind === "traded"} onclick={() => (kind = "traded")}
            title="The item sold or was bought at this price">Traded</button
          >
          <button type="button" role="radio" aria-checked={kind === "asking"} onclick={() => (kind = "asking")}
            title="Someone listed it at this price">Asking</button
          >
        </div>
      </div>
      <input class="note" bind:value={note} onkeydown={onKey} placeholder="Note (optional), e.g. FM shop" maxlength="80" />
      <p class="for">For {item.name} · {variant}{stats ? ` · ${stats}` : ""}</p>
      <div class="actions">
        <button type="button" class="link" onclick={() => (adding = false)}>Cancel</button>
        <button type="submit" class="save" disabled={saving || parsed == null}>Save</button>
      </div>
    </form>
  {/if}
  {#if error}<p class="error">{error}</p>{/if}
</div>

<style>
  .price {
    margin-top: 12px;
    padding-top: 10px;
    border-top: 1px solid var(--border);
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 6px;
  }

  .add {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 8px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--muted);
    font-size: 11.5px;
    cursor: pointer;
  }

  .add:hover {
    border-color: var(--accent);
    color: var(--text);
  }

  .summary {
    display: flex;
    align-items: baseline;
    gap: 6px;
  }

  .markets {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    margin-top: 8px;
  }

  .markets button {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 22px;
    padding: 0 7px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--muted);
    font-size: 11px;
    cursor: pointer;
  }

  .markets button:hover {
    border-color: var(--border-strong);
    color: var(--text);
  }

  .median {
    font-family: var(--font-display);
    font-size: 17px;
    font-weight: 600;
  }

  .label,
  .spread,
  .muted {
    color: var(--faint);
    font-size: 11.5px;
  }

  p {
    margin: 0;
  }

  .like {
    margin-top: 3px;
    font-size: 11.5px;
    color: var(--muted);
  }

  .like b {
    color: var(--text);
    font-weight: 600;
  }

  .history {
    display: grid;
    gap: 1px;
    margin: 8px 0 0;
    padding: 0;
    list-style: none;
  }

  .history li {
    display: flex;
    align-items: baseline;
    gap: 8px;
    padding: 2px 0;
    font-size: 11.5px;
  }

  .day {
    width: 42px;
    flex: none;
    color: var(--faint);
  }

  .amount {
    width: 62px;
    flex: none;
    text-align: right;
    font-weight: 500;
  }

  .kind {
    flex: none;
    color: var(--good);
    font-size: 10.5px;
  }

  .kind.asking {
    color: var(--muted);
  }

  .what {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    color: var(--faint);
  }

  .remove {
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    padding: 0;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--faint);
    opacity: 0;
    cursor: pointer;
  }

  .history li:hover .remove,
  .remove:focus-visible {
    opacity: 1;
  }

  .remove:hover {
    color: var(--bad);
  }

  .link {
    margin-top: 4px;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--muted);
    font-size: 11.5px;
    cursor: pointer;
  }

  .link:hover {
    color: var(--text);
    text-decoration: underline;
  }

  .form {
    display: grid;
    gap: 6px;
    margin-top: 8px;
    padding: 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface);
  }

  .fields {
    display: flex;
    gap: 6px;
  }

  .amount-field {
    position: relative;
    flex: 1;
    min-width: 0;
  }

  input {
    width: 100%;
    padding: 5px 8px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--bg);
    color: var(--text);
    font: inherit;
    font-size: 12.5px;
  }

  input:focus {
    outline: none;
    border-color: var(--accent);
  }

  .amount-field input {
    padding-right: 74px;
    font-variant-numeric: tabular-nums;
  }

  .unit {
    position: absolute;
    top: 50%;
    right: 8px;
    transform: translateY(-50%);
    color: var(--faint);
    font-size: 11px;
    pointer-events: none;
  }

  .kinds {
    display: flex;
    flex: none;
    padding: 2px;
    border-radius: var(--radius-sm);
    background: rgba(255, 255, 255, 0.04);
  }

  .kinds button {
    padding: 2px 8px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--muted);
    font-size: 11.5px;
    cursor: pointer;
  }

  .kinds button[aria-checked="true"] {
    background: var(--surface-3);
    color: var(--text);
  }

  .note {
    font-size: 12px;
  }

  .for {
    color: var(--faint);
    font-size: 11px;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 12px;
  }

  .actions .link {
    margin: 0;
  }

  .save {
    padding: 4px 14px;
    border: 0;
    border-radius: var(--radius-sm);
    background: var(--accent);
    color: #1a1204;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }

  .save:disabled {
    opacity: 0.45;
    cursor: default;
  }

  .add:focus-visible,
  .kinds button:focus-visible,
  .save:focus-visible,
  .link:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  .error {
    margin-top: 6px;
    color: var(--bad);
    font-size: 11.5px;
  }

  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
</style>
