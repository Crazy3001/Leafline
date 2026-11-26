<script lang="ts">
  // A backup or a records file beside the records as they are now, kind by kind, before the
  // player uses it in their place. The records now are kept aside first, never deleted.
  import { formatDay } from "$lib/format";
  import type { RecordsSummary } from "$lib/types";

  let {
    title,
    incoming,
    current,
    busy = false,
    onuse,
    oncancel,
  }: {
    /** What is being looked at: "The backup of Mon 24 Nov", "That file". */
    title: string;
    incoming: RecordsSummary;
    current: RecordsSummary | null;
    busy?: boolean;
    onuse: () => void;
    oncancel: () => void;
  } = $props();

  const KINDS: { key: keyof Omit<RecordsSummary, "newest">; label: string }[] = [
    { key: "sessions", label: "Sessions" },
    { key: "drops", label: "Drops logged" },
    { key: "builds", label: "Builds" },
    { key: "notes", label: "Notes" },
    { key: "spots", label: "Saved spots" },
    { key: "prices", label: "Prices" },
    { key: "characters", label: "Characters" },
  ];

  const rows = $derived(
    KINDS.map((k) => ({ ...k, then: incoming[k.key], now: current?.[k.key] ?? null })).filter((r) => r.then > 0 || (r.now ?? 0) > 0),
  );
  const lost = $derived(rows.filter((r) => r.now != null && r.then < r.now));
  const day = (at: number | null) => (at != null ? formatDay(at) : "none");
</script>

<div class="compare" role="alertdialog" aria-label="Use these records">
  <p class="title">{title}, beside your records now</p>
  <table>
    <thead>
      <tr><th></th><th class="num">In it</th><th class="num">Now</th></tr>
    </thead>
    <tbody>
      {#each rows as r (r.key)}
        <tr class:less={r.now != null && r.then < r.now} class:more={r.now != null && r.then > r.now}>
          <td>{r.label}</td>
          <td class="num">{r.then}</td>
          <td class="num">{r.now ?? "—"}</td>
        </tr>
      {/each}
      <tr>
        <td>Newest session or drop</td>
        <td class="num">{day(incoming.newest)}</td>
        <td class="num">{current ? day(current.newest) : "—"}</td>
      </tr>
    </tbody>
  </table>
  {#if !rows.length}<p class="help">It holds no records.</p>{/if}
  <p class="help">
    {#if lost.length}
      Not in it, recorded since: {lost.map((r) => `${r.now! - r.then} ${r.label.toLowerCase()}`).join(", ")}.
    {:else}
      Using it puts these records in place of the ones here.
    {/if}
    Your records now are kept aside first, so you can go back to them from the backups list.
  </p>
  <div class="actions">
    <button class="button primary" onclick={onuse} disabled={busy}>Use these records</button>
    <button class="button" onclick={oncancel} disabled={busy}>Cancel</button>
  </div>
</div>

<style>
  .compare {
    display: grid;
    gap: var(--space-2);
    margin: var(--space-2) 0;
    padding: var(--space-3);
    border: var(--line-w) solid var(--line);
    border-radius: var(--radius);
    background: var(--surface-2);
    font-size: var(--text-sm);
  }

  .title {
    margin: 0;
    font-weight: 600;
  }

  table {
    width: 100%;
    max-width: 420px;
    border-collapse: collapse;
  }

  th {
    color: var(--muted);
    font-weight: 500;
    text-align: left;
  }

  td,
  th {
    padding: 3px 0;
    border-bottom: var(--line-w) solid var(--line);
  }

  .num {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  tr.less td:nth-child(2) {
    color: var(--warn);
  }

  tr.more td:nth-child(2) {
    color: var(--good);
  }

  .help {
    margin: 0;
    color: var(--muted);
  }

  .actions {
    display: flex;
    gap: var(--space-2);
  }

  .button {
    height: var(--control-h);
    padding: 0 12px;
    border: var(--line-w) solid var(--line);
    border-radius: var(--radius);
    background: var(--surface);
    color: var(--text);
    font: inherit;
    cursor: pointer;
  }

  .button.primary {
    border-color: var(--accent);
    background: var(--accent);
    color: var(--accent-ink);
  }

  .button:disabled {
    opacity: 0.6;
    cursor: default;
  }
</style>
