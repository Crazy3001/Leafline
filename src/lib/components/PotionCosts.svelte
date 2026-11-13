<script lang="ts">
  // Potions for one session: how many of each the player had at the start and has now, what that
  // cost, and mesos per hour once it is taken out. The player names the slots and sets prices;
  // Leafline reads nothing about potions from the game.
  import { ChevronRight, Plus, X } from "@lucide/svelte";

  import { backend } from "$lib/api";
  import { formatCompact, formatNumber } from "$lib/format";
  import { MAX_SLOTS, netMesosPerHour, potions, watchPotions } from "$lib/potions.svelte";
  import type { PotionSlot, SessionPotion } from "$lib/types";
  import Button from "$lib/ui/Button.svelte";

  let {
    startedAt,
    activeSeconds,
    mesosPerHour,
    collapsible = true,
  }: {
    /** The session's start, which its counts are kept by. */
    startedAt: number | null;
    activeSeconds: number;
    mesosPerHour: number | null;
    /** Folds behind a one-line summary, as on the Train tab. */
    collapsible?: boolean;
  } = $props();

  const OPEN_KEY = "leafline.train.potionsOpen";

  watchPotions();

  let counts = $state<SessionPotion[]>([]);
  let opened = $state(stored(OPEN_KEY) === "true");
  const open = $derived(!collapsible || opened);
  let editing = $state(false);
  let draft = $state<{ slot: number; name: string; price: string }[]>([]);
  let problem = $state<string | null>(null);

  function stored(key: string) {
    try {
      return localStorage.getItem(key);
    } catch {
      return null;
    }
  }

  function load() {
    const at = startedAt;
    if (at == null) {
      counts = [];
      return;
    }
    backend
      .sessionPotions(at)
      .then((rows) => {
        if (startedAt === at) counts = rows;
      })
      .catch(() => {});
  }

  $effect(() => {
    void startedAt;
    load();
  });

  $effect(() => {
    const stop = backend.onPotions(load);
    return () => {
      stop.then((unlisten) => unlisten());
    };
  });

  /** Every slot set up now, plus counts typed under a slot since removed. */
  const rows = $derived.by(() => {
    const out: { slot: number; name: string; price: number; typed: SessionPotion | null; gone: boolean }[] =
      potions.slots.map((s) => ({ ...s, typed: counts.find((c) => c.slot === s.slot) ?? null, gone: false }));
    for (const c of counts)
      if (!potions.slots.some((s) => s.slot === c.slot)) out.push({ slot: c.slot, name: c.name, price: c.price, typed: c, gone: true });
    return out.sort((a, b) => a.slot - b.slot);
  });
  const cost = $derived(counts.reduce((a, c) => a + c.used * c.price, 0));
  const net = $derived(netMesosPerHour(mesosPerHour, cost, activeSeconds));
  const anyUsed = $derived(counts.some((c) => c.start != null && c.now != null));

  function toggle() {
    opened = !opened;
    try {
      localStorage.setItem(OPEN_KEY, String(opened));
    } catch {
      // A convenience only.
    }
  }

  const count = (value: string) => {
    const n = Math.round(Number(value));
    return value.trim() === "" || !Number.isFinite(n) || n < 0 ? null : n;
  };

  async function setCount(slot: number, which: "start" | "now", value: string) {
    if (startedAt == null) return;
    const typed = counts.find((c) => c.slot === slot);
    const start = which === "start" ? count(value) : (typed?.start ?? null);
    const now = which === "now" ? count(value) : (typed?.now ?? null);
    problem = null;
    try {
      counts = await backend.setSessionPotion(startedAt, slot, start, now);
    } catch (e) {
      problem = String(e);
    }
  }

  function startEditing() {
    draft = potions.slots.map((s) => ({ slot: s.slot, name: s.name, price: String(s.price) }));
    if (!draft.length) draft = [{ slot: 0, name: "", price: "" }];
    problem = null;
    editing = true;
  }

  async function saveSlots() {
    const slots: PotionSlot[] = draft
      .filter((d) => d.name.trim() || d.price.trim())
      .map((d) => ({ slot: d.slot, name: d.name.trim(), price: Math.round(Number(d.price)) }));
    const bad = slots.find((s) => !s.name || !Number.isFinite(s.price) || s.price < 0);
    if (bad) {
      problem = bad.name ? `${bad.name}: set a price of 0 or more.` : "Each potion needs a name.";
      return;
    }
    try {
      potions.slots = await backend.setPotionSlots(slots);
      editing = false;
      problem = null;
    } catch (e) {
      problem = String(e);
    }
  }
</script>

<div class="potions">
  {#if collapsible}
    <button class="head" aria-expanded={open} onclick={toggle}>
      <span class="chevron" class:open><ChevronRight size={12} /></span>
      <span class="title">Potions</span>
      <span class="grow"></span>
      {#if anyUsed && net != null}
        <span class="sum tabular"><span class="muted">after potions</span> {formatCompact(net)} mesos / hr</span>
      {:else if !potions.slots.length}
        <span class="muted">Not set up</span>
      {/if}
    </button>
  {/if}

  {#if open}
    <div class="body">
      {#if editing}
        <p class="muted">Name the potions you hunt with and what one costs you. Prices are yours; nothing is filled in for you.</p>
        <div class="edit">
          {#each draft as d, i (i)}
            <div class="edit-row">
              <input class="name" placeholder="Potion" aria-label="Potion name" bind:value={d.name} />
              <input class="price" placeholder="Price" inputmode="numeric" aria-label="Price in mesos" bind:value={d.price} />
              <button class="icon" aria-label="Remove {d.name || 'this potion'}" onclick={() => (draft = draft.filter((_, j) => j !== i))}>
                <X size={12} />
              </button>
            </div>
          {/each}
        </div>
        <div class="actions">
          {#if draft.length < MAX_SLOTS}
            <Button variant="ghost" size="sm" onclick={() => (draft = [...draft, { slot: 0, name: "", price: "" }])}>
              <Plus size={12} />Add potion
            </Button>
          {/if}
          <span class="grow"></span>
          <Button variant="ghost" size="sm" onclick={() => (editing = false)}>Cancel</Button>
          <Button size="sm" onclick={saveSlots}>Save</Button>
        </div>
      {:else if !potions.slots.length && !counts.length}
        <p class="muted">Name the potions you hunt with and set their prices to see your mesos per hour after potions.</p>
        <div class="actions"><Button size="sm" onclick={startEditing}>Set up potions</Button></div>
      {:else}
        {#if startedAt == null}
          <p class="muted">Counts are kept per session; they open once a session starts.</p>
        {:else}
          <table>
            <thead><tr><th>Potion</th><th>Start</th><th>Now</th><th>Used</th><th>Cost</th></tr></thead>
            <tbody>
              {#each rows as row (row.slot)}
                <tr class:gone={row.gone}>
                  <td>
                    <span class="pname">{row.name}</span>
                    <span class="muted tabular">{formatNumber(row.price)}</span>
                  </td>
                  <td>
                    <input
                      class="count"
                      inputmode="numeric"
                      aria-label="{row.name} at the start"
                      value={row.typed?.start ?? ""}
                      onchange={(e) => setCount(row.slot, "start", e.currentTarget.value)}
                    />
                  </td>
                  <td>
                    <input
                      class="count"
                      inputmode="numeric"
                      aria-label="{row.name} now"
                      value={row.typed?.now ?? ""}
                      onchange={(e) => setCount(row.slot, "now", e.currentTarget.value)}
                    />
                  </td>
                  <td class="tabular">{row.typed && row.typed.start != null && row.typed.now != null ? formatNumber(row.typed.used) : "—"}</td>
                  <td class="tabular">{row.typed?.used ? formatNumber(row.typed.used * row.typed.price) : "—"}</td>
                </tr>
              {/each}
            </tbody>
          </table>
          <div class="totals tabular">
            <span><span class="muted">Potions</span> {cost ? `−${formatNumber(cost)}` : "0"}</span>
            <span>
              <span class="muted">Mesos / hr after potions</span>
              <strong>{net != null ? formatCompact(net) : "—"}</strong>
            </span>
          </div>
          <p class="muted small">Type your count when you start and again when you check. Bought more while hunting? Add them to Start.</p>
        {/if}
        <div class="actions"><Button variant="ghost" size="sm" onclick={startEditing}>Edit potions</Button></div>
      {/if}
      {#if problem}<p class="warn">{problem}</p>{/if}
    </div>
  {/if}
</div>

<style>
  .potions {
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

  .totals strong {
    font-weight: 600;
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

  p.small {
    font-size: var(--text-xs);
  }

  .warn {
    color: var(--warn);
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

  td:first-child {
    display: flex;
    flex-direction: column;
  }

  td:first-child .muted {
    font-size: var(--text-xs);
  }

  tr.gone .pname {
    color: var(--muted);
    text-decoration: line-through;
  }

  input {
    height: 22px;
    padding: 0 4px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
  }

  input.count {
    width: 52px;
    text-align: right;
  }

  .totals {
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
    gap: 8px;
    font-size: var(--text-sm);
  }

  .edit {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .edit-row {
    display: flex;
    gap: 4px;
  }

  .edit-row .name {
    flex: 1;
    min-width: 0;
  }

  .edit-row .price {
    width: 70px;
    text-align: right;
  }

  .icon {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    padding: 0;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--muted);
    cursor: pointer;
  }

  .icon:hover {
    background: rgba(255, 255, 255, 0.07);
    color: var(--text);
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  @media (prefers-reduced-motion: reduce) {
    .chevron {
      transition: none;
    }
  }
</style>
