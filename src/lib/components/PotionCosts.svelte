<script lang="ts">
  // Potions for one session: how many of each went, what that cost, and mesos per hour once it is
  // taken out. The player names the slots and sets prices. A potion put on a quick slot is
  // counted from the number the game shows there; any other is typed, start and now.
  import { ChevronRight, Plus, X } from "@lucide/svelte";

  import { backend } from "$lib/api";
  import { formatCompact, formatNumber } from "$lib/format";
  import { MAX_SLOTS, netMesosPerHour, QUICK_SLOTS } from "$lib/potions.svelte";
  import type { PotionSlot, QuickSlotUse, SessionPotion } from "$lib/types";
  import Button from "$lib/ui/Button.svelte";

  let {
    startedAt,
    activeSeconds,
    mesosPerHour,
    character = null,
    collapsible = true,
    quickSlots = null,
    lootPerHour = null,
  }: {
    /** The session's start, which its counts are kept by. */
    startedAt: number | null;
    activeSeconds: number;
    mesosPerHour: number | null;
    /** Whose potions these are: slots are kept per character. */
    character?: string | null;
    /** Folds behind a one-line summary, as on the Train tab. */
    collapsible?: boolean;
    /** The running session's quick-slot counts, newer than what was last saved. */
    quickSlots?: QuickSlotUse[] | null;
    /** What the loot picked up is worth an hour, added in before potions come off. */
    lootPerHour?: number | null;
  } = $props();

  const OPEN_KEY = "leafline.train.potionsOpen";

  let slots = $state<PotionSlot[]>([]);
  let counts = $state<SessionPotion[]>([]);
  let opened = $state(stored(OPEN_KEY) === "true");
  const open = $derived(!collapsible || opened);
  let editing = $state(false);
  let draft = $state<{ slot: number; name: string; price: string; quickSlot: string }[]>([]);
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

  function loadSlots() {
    const who = character;
    backend
      .potionSlots(who)
      .then((rows) => {
        if (character === who) slots = rows;
      })
      .catch(() => {});
  }

  $effect(() => {
    void startedAt;
    load();
  });

  $effect(() => {
    void character;
    loadSlots();
  });

  $effect(() => {
    const stop = backend.onPotions(() => {
      load();
      loadSlots();
    });
    return () => {
      stop.then((unlisten) => unlisten());
    };
  });

  interface Row {
    slot: number;
    name: string;
    price: number;
    quickSlot: number | null;
    typed: SessionPotion | null;
    gone: boolean;
    /** The count on its quick slot now, while a session reads it. */
    reading: number | null;
    /** How many went, counted from the quick slot; null when nothing was read. */
    read: number | null;
    /** What counts: read, else typed. */
    used: number | null;
  }

  /** Every slot set up now, plus counts kept under a slot since removed. */
  const rows = $derived.by(() => {
    const row = (s: { slot: number; name: string; price: number; quickSlot?: number | null }, typed: SessionPotion | null, gone: boolean): Row => {
      const quick = s.quickSlot ?? null;
      const live = quick != null ? (quickSlots?.find((q) => q.slot === quick) ?? null) : null;
      const read = live?.used ?? typed?.read ?? null;
      const typedUsed = typed && typed.start != null && typed.now != null ? typed.used : null;
      return { slot: s.slot, name: s.name, price: s.price, quickSlot: quick, typed, gone, reading: live?.count ?? null, read, used: read ?? typedUsed };
    };
    const out = slots.map((s) => row(s, counts.find((c) => c.slot === s.slot) ?? null, false));
    for (const c of counts) if (!slots.some((s) => s.slot === c.slot)) out.push(row(c, c, true));
    return out.sort((a, b) => a.slot - b.slot);
  });
  const cost = $derived(rows.reduce((a, r) => a + (r.used ?? 0) * (r.typed?.price ?? r.price), 0));
  const net = $derived(netMesosPerHour(mesosPerHour, cost, activeSeconds, lootPerHour));
  const anyUsed = $derived(rows.some((r) => r.used != null));
  const anyRead = $derived(slots.some((s) => s.quickSlot != null));

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
      counts = await backend.setSessionPotion(character, startedAt, slot, start, now);
    } catch (e) {
      problem = String(e);
    }
  }

  function startEditing() {
    draft = slots.map((s) => ({ slot: s.slot, name: s.name, price: String(s.price), quickSlot: s.quickSlot != null ? String(s.quickSlot) : "" }));
    if (!draft.length) draft = [{ slot: 0, name: "", price: "", quickSlot: "" }];
    problem = null;
    editing = true;
  }

  async function saveSlots() {
    const next: PotionSlot[] = draft
      .filter((d) => d.name.trim() || d.price.trim())
      .map((d) => ({
        slot: d.slot,
        name: d.name.trim(),
        price: Math.round(Number(d.price)),
        quickSlot: d.quickSlot === "" ? null : Number(d.quickSlot),
      }));
    const doubled = next.find((s, i) => s.quickSlot != null && next.findIndex((o) => o.quickSlot === s.quickSlot) !== i);
    if (doubled) {
      problem = `Two potions are on ${QUICK_SLOTS[doubled.quickSlot!]}.`;
      return;
    }
    const bad = next.find((s) => !s.name || !Number.isFinite(s.price) || s.price < 0);
    if (bad) {
      problem = bad.name ? `${bad.name}: set a price of 0 or more.` : "Each potion needs a name.";
      return;
    }
    try {
      slots = await backend.setPotionSlots(character, next);
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
        <span class="sum tabular"
          ><span class="muted">{lootPerHour != null ? "with loot, after potions" : "after potions"}</span>
          {formatCompact(net)} mesos / hr</span
        >
      {:else if !slots.length}
        <span class="muted">Not set up</span>
      {/if}
    </button>
  {/if}

  {#if open}
    <div class="body">
      {#if editing}
        <p class="muted">
          Name the potions you hunt with and what one costs you. Prices are yours; nothing is filled in for you. Pick the
          quick slot a potion sits on and Leafline counts what you drink from the number the game shows there.
        </p>
        <div class="edit">
          {#each draft as d, i (i)}
            <div class="edit-row">
              <input class="name" placeholder="Potion" aria-label="Potion name" bind:value={d.name} />
              <input class="price" placeholder="Price" inputmode="numeric" aria-label="Price in mesos" bind:value={d.price} />
              <select class="where" aria-label="Quick slot for {d.name || 'this potion'}" bind:value={d.quickSlot}>
                <option value="">Type counts</option>
                {#each QUICK_SLOTS as label, q (q)}<option value={String(q)}>{label}</option>{/each}
              </select>
              <button class="icon" aria-label="Remove {d.name || 'this potion'}" onclick={() => (draft = draft.filter((_, j) => j !== i))}>
                <X size={12} />
              </button>
            </div>
          {/each}
        </div>
        <div class="actions">
          {#if draft.length < MAX_SLOTS}
            <Button variant="ghost" size="sm" onclick={() => (draft = [...draft, { slot: 0, name: "", price: "", quickSlot: "" }])}>
              <Plus size={12} />Add potion
            </Button>
          {/if}
          <span class="grow"></span>
          <Button variant="ghost" size="sm" onclick={() => (editing = false)}>Cancel</Button>
          <Button size="sm" onclick={saveSlots}>Save</Button>
        </div>
      {:else if !slots.length && !counts.length}
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
                  {#if row.read != null}
                    <td class="read muted" colspan="2">
                      {row.quickSlot != null ? QUICK_SLOTS[row.quickSlot].split(" · ")[1] : "Quick slot"}{row.reading != null
                        ? `, ${formatNumber(row.reading)} left`
                        : ""}
                    </td>
                  {:else}
                    <td>
                      <input
                        class="count"
                        inputmode="numeric"
                        aria-label="{row.name} at the start"
                        placeholder={row.quickSlot != null ? "read" : ""}
                        value={row.typed?.start ?? ""}
                        onchange={(e) => setCount(row.slot, "start", e.currentTarget.value)}
                      />
                    </td>
                    <td>
                      <input
                        class="count"
                        inputmode="numeric"
                        aria-label="{row.name} now"
                        placeholder={row.quickSlot != null ? "read" : ""}
                        value={row.typed?.now ?? ""}
                        onchange={(e) => setCount(row.slot, "now", e.currentTarget.value)}
                      />
                    </td>
                  {/if}
                  <td class="tabular">{row.used != null ? formatNumber(row.used) : "—"}</td>
                  <td class="tabular">{row.used ? formatNumber(row.used * (row.typed?.price ?? row.price)) : "—"}</td>
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
          <p class="muted small">
            {#if anyRead}Potions on a quick slot are counted as you drink them; buying more doesn't count as drunk. Keep the
              quick-slot bar in view. For the rest, type{:else}Type{/if} your count when you start and again when you check. Bought more
            while hunting? Add them to Start.
          </p>
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
    font-size: var(--text-md);
    text-align: left;
    cursor: pointer;
  }

  .head:hover .title {
    color: var(--accent-text-strong);
  }

  .title {
    font-weight: var(--weight-strong);
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
    font-weight: var(--weight-strong);
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
    font-size: var(--text-md);
  }

  p.small {
    font-size: var(--text-xs);
  }

  .warn {
    color: var(--warn);
  }

  .read {
    font-size: var(--text-xs);
  }

  select.where {
    height: var(--control-h);
    padding: 0 4px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    color: var(--text);
    font: inherit;
    font-size: var(--text-xs);
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--text-md);
  }

  th {
    padding: 0 0 4px;
    font-size: var(--text-xs);
    font-weight: var(--weight-body);
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
    font-size: var(--text-md);
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
    font-size: var(--text-md);
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
    background: var(--hover);
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
