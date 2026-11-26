<script lang="ts">
  // The desk view's quest card on Now: quests ready to hand in first, then the ones the Quest
  // Helper tracks, each with its first step still to do and how far along it is.
  import { Flag } from "@lucide/svelte";

  import { questTitle } from "$lib/format";
  import { questLog, readyToHandIn, watchQuestLog } from "$lib/questlog.svelte";
  import { live } from "$lib/readings.svelte";
  import TitledCard from "$lib/ui/TitledCard.svelte";

  let { ongo }: { ongo: () => void } = $props();

  watchQuestLog();

  const SHOWN = 4;
  const tracked = $derived((live.analysis?.quest?.quests ?? []).filter((q) => q.title));

  type Row = { key: string; title: string; tag: string | null; ready: boolean; line: string; current?: number; required?: number };

  const rows = $derived.by(() => {
    const out: Row[] = [];
    const seen = new Set<string>();
    for (const q of tracked) {
      const done = q.objectives.length > 0 && q.objectives.every((o) => !o.unread && o.current >= o.required);
      const next = q.objectives.find((o) => !o.unread && o.current < o.required) ?? q.objectives.find((o) => o.unread);
      const t = questTitle(q.title);
      seen.add(q.title.toLowerCase());
      out.push({
        key: `t:${q.title}`,
        title: t.name,
        tag: t.tag,
        ready: done,
        line: done ? "Ready to hand in" : next ? next.target : "Counts not read yet",
        current: !done && next && !next.unread ? next.current : undefined,
        required: !done && next && !next.unread ? next.required : undefined,
      });
    }
    for (const q of questLog.log?.quests ?? []) {
      if (!readyToHandIn(q) || seen.has(q.title.toLowerCase())) continue;
      const t = questTitle(q.title);
      out.push({ key: `l:${q.key}`, title: t.name, tag: t.tag, ready: true, line: "Ready to hand in" });
    }
    // Ready ones first, then the game's order.
    return out.sort((a, b) => Number(b.ready) - Number(a.ready)).slice(0, SHOWN);
  });
  const total = $derived(rows.length);
</script>

<TitledCard title="Quests" icon={Flag} tone="rose" more={total ? "All quests" : "Quests"} onmore={ongo}>
  {#if rows.length}
    <ul class="rows">
      {#each rows as r (r.key)}
        <li class:ready={r.ready}>
          <div class="top">
            <span class="name">{r.title}</span>
            {#if r.tag}<span class="badge">{r.tag}</span>{/if}
            {#if r.ready}<span class="badge good">Ready</span>{:else if r.current != null}<span class="count num">{r.current}<span class="of"> / {r.required}</span></span>{/if}
          </div>
          <div class="line">{r.line}</div>
          {#if r.current != null && r.required}
            <div class="bar" aria-hidden="true"><span style:width="{Math.min(1, r.current / r.required) * 100}%"></span></div>
          {/if}
        </li>
      {/each}
    </ul>
  {:else}
    <p class="empty">No quests tracked. Open the Quest Helper in game, or follow a quest from the Quests page.</p>
  {/if}
</TitledCard>

<style>
  .rows {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li {
    padding: var(--space-3);
    border-radius: var(--radius);
    background: var(--surface-2);
  }

  li.ready {
    background: color-mix(in srgb, var(--good) 13%, var(--surface-2));
  }

  .top {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
  }

  .name {
    min-width: 0;
    overflow: hidden;
    font-weight: var(--weight-strong);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .count {
    margin-left: auto;
    flex: none;
    font-weight: var(--weight-strong);
  }

  .top .badge.good {
    margin-left: auto;
  }

  .of {
    color: var(--faint);
    font-weight: var(--weight-body);
  }

  .line {
    margin-top: var(--space-0-5);
    overflow: hidden;
    color: var(--muted);
    font-size: var(--text-xs);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .bar {
    height: 6px;
    margin-top: var(--space-2);
    border-radius: var(--radius-pill);
    background: var(--track);
    overflow: hidden;
  }

  .bar span {
    display: block;
    height: 100%;
    border-radius: var(--radius-pill);
    background: var(--ref-item);
  }

  .empty {
    margin: 0;
    color: var(--muted);
  }
</style>
