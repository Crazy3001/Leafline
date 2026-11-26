<script lang="ts">
  // Settings → Data → Backups: the records' daily copies (the last 7) and the records kept aside
  // before the last restore or import, each with what it holds and how that differs from now. One
  // picked is set beside the records now before anything changes; using it keeps the records now
  // aside first, so a restore can always be undone from this same list.
  import { backend } from "$lib/api";
  import { formatDay } from "$lib/format";
  import { RECORDS_NOTE } from "$lib/recordsfile";
  import type { Backup, RecordsSummary } from "$lib/types";
  import RecordsCompare from "./RecordsCompare.svelte";

  let { onrestored }: { onrestored: (message: string) => void } = $props();

  let backups = $state<Backup[] | null>(null);
  let current = $state<RecordsSummary | null>(null);
  let problem = $state<string | null>(null);
  let busy = $state(false);
  /** The backup picked, taken in and ready to use. */
  let picked = $state<{ backup: Backup; summary: RecordsSummary } | null>(null);

  async function load() {
    problem = null;
    try {
      [backups, current] = await Promise.all([backend.listBackups(), backend.recordsSummary().catch(() => null)]);
    } catch (err) {
      problem = String(err);
      backups = [];
    }
  }

  $effect(() => {
    void load();
  });

  const name = (b: Backup) =>
    b.beforeRestore ? "Before your last restore or import" : b.day ? `${weekday(b.day)} ${formatDay(Date.parse(`${b.day}T12:00:00`) / 1000)}` : b.name;
  const weekday = (day: string) => new Date(`${day}T12:00:00`).toLocaleDateString("en-GB", { weekday: "short" });

  /** How a backup differs from now, in a few words. */
  function differs(s: RecordsSummary | null): string {
    if (!s || !current) return "";
    const kinds: [keyof Omit<RecordsSummary, "newest">, string][] = [
      ["sessions", "session"],
      ["drops", "drop"],
      ["builds", "build"],
      ["notes", "note"],
      ["spots", "spot"],
      ["prices", "price"],
    ];
    const out = kinds
      .map(([k, word]) => {
        const d = s[k] - current![k];
        if (!d) return null;
        return `${d > 0 ? "+" : "−"}${Math.abs(d)} ${word}${Math.abs(d) === 1 ? "" : "s"}`;
      })
      .filter((x): x is string => x != null);
    return out.length ? out.join(", ") : "Same counts as now";
  }

  async function pick(b: Backup) {
    busy = true;
    problem = null;
    try {
      picked = { backup: b, summary: await backend.checkBackup(b.name) };
    } catch (err) {
      problem = String(err);
    } finally {
      busy = false;
    }
  }

  async function use() {
    if (!picked) return;
    busy = true;
    problem = null;
    const what = picked.backup.beforeRestore ? "the records from before your last restore" : `the backup of ${name(picked.backup)}`;
    try {
      // Every window reloads once the records are replaced; Settings says this when it is back.
      sessionStorage.setItem(RECORDS_NOTE, `Restored ${what}. The records you had are kept as "Before your last restore or import".`);
      await backend.useRecords();
      picked = null;
      onrestored(`Restored ${what}. The records you had are kept as "Before your last restore or import".`);
      await load();
    } catch (err) {
      problem = String(err);
    } finally {
      busy = false;
    }
  }
</script>

<div class="backups">
  {#if backups == null}
    <p class="help">Looking at the backups…</p>
  {:else if !backups.length}
    <p class="help">No backups yet. Leafline makes one the first time it opens each day.</p>
  {:else}
    <ul>
      {#each backups as b (b.name)}
        <li class:on={picked?.backup.name === b.name}>
          <span class="name">{name(b)}</span>
          <span class="what">
            {#if b.problem}Can't be read: {b.problem}{:else}{differs(b.summary)}{/if}
          </span>
          {#if !b.problem}
            <button class="button" onclick={() => pick(b)} disabled={busy}>Preview</button>
          {/if}
        </li>
        {#if picked?.backup.name === b.name}
          <RecordsCompare
            title={b.beforeRestore ? "The records from before your last restore" : `The backup of ${name(b)}`}
            incoming={picked.summary}
            {current}
            {busy}
            onuse={use}
            oncancel={() => (picked = null)}
          />
        {/if}
      {/each}
    </ul>
  {/if}
  {#if problem}<p class="warn">{problem}</p>{/if}
</div>

<style>
  .backups {
    display: grid;
    gap: var(--space-2);
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li {
    display: grid;
    grid-template-columns: minmax(150px, auto) 1fr auto;
    align-items: center;
    gap: var(--space-3);
    padding: 5px 0;
    border-bottom: var(--line-w) solid var(--line);
    font-size: var(--text-sm);
  }

  .name {
    font-weight: 500;
  }

  .what,
  .help {
    color: var(--muted);
    font-size: var(--text-sm);
  }

  .help,
  .warn {
    margin: 0;
  }

  .warn {
    color: var(--warn);
    font-size: var(--text-sm);
  }

  .button {
    height: 26px;
    padding: 0 10px;
    border: var(--line-w) solid var(--line);
    border-radius: var(--radius);
    background: var(--surface);
    color: var(--text);
    font: inherit;
    font-size: var(--text-xs);
    cursor: pointer;
  }

  .button:disabled {
    opacity: 0.6;
    cursor: default;
  }
</style>
