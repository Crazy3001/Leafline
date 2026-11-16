<script lang="ts">
  // One quest from the search: who it comes from and how it goes, step by step, and where it
  // stands in the character's quest log, with anything the game showed that the data says
  // otherwise.
  import { Check, LoaderCircle, Plus } from "@lucide/svelte";
  import { untrack } from "svelte";

  import Card from "./Card.svelte";
  import QuestMark from "./quest/QuestMark.svelte";
  import QuestSteps from "./quest/QuestSteps.svelte";
  import { backend } from "$lib/api";
  import { formatAgo, formatNumber } from "$lib/format";
  import { follow, isFollowed, unfollow } from "$lib/quests.svelte";
  import { fold, logged, markOf, watchQuestLog } from "$lib/questlog.svelte";
  import type { MapReading, Objective, QuestReading, QuestWhere } from "$lib/types";

  let {
    id,
    map = null,
    quest = null,
    plain = false,
    compact = false,
  }: {
    id: number;
    map?: MapReading | null;
    /** What the Quest Helper shows, for this quest's counts while it is tracked. */
    quest?: QuestReading | null;
    plain?: boolean;
    /** A link card: the credits stay in the full view. */
    compact?: boolean;
  } = $props();

  let found = $state<QuestWhere | null>(null);
  let error = $state<string | null>(null);

  const key = $derived(JSON.stringify([id, map?.name, map?.street, map?.twin]));

  // Follows the key alone: every live reading brings new objects with the same content.
  $effect(() => {
    const asked = key;
    error = null;
    untrack(() => backend.questSheet(id, map?.name ?? null, map?.street ?? null, map?.twin ?? null))
      .then((result) => {
        if (asked === key) found = result;
      })
      .catch((e) => (error = String(e)));
  });

  watchQuestLog();

  const sheet = $derived(found?.sheet.row.id === id ? found.sheet : null);
  /** Where the quest stands in the character's quest log, as last seen. */
  const entry = $derived(logged(id));
  const mark = $derived(markOf(entry));
  /** The Quest Helper's counts for it while it shows this quest, or else the Quest window's last ones. */
  const helperCounts = $derived(sheet ? (quest?.quests.find((q) => q.title === sheet.row.name)?.objectives ?? []) : []);
  const logCounts = $derived<Objective[]>(
    entry?.state === "inProgress" ? entry.counters.map((c) => ({ target: c.name, current: c.have, required: c.need })) : [],
  );
  const progress = $derived(helperCounts.length ? helperCounts : logCounts);
  /** Steps start at the objectives once the quest has been taken. */
  const taken = $derived(helperCounts.length > 0 || entry?.state === "inProgress");
  const ago = (at: number | null) => (at ? formatAgo(Date.now() / 1000 - at) : null);
  const standing = $derived.by(() => {
    if (!entry || !mark) return null;
    if (mark === "done") return entry.doneOn ? `Completed on ${entry.doneOn}` : `Seen on the Completed tab ${ago(entry.doneSeenAt) ?? ""}`;
    if (mark === "available") return `On the Available tab ${ago(entry.stateSeenAt) ?? ""}`;
    if (mark === "gone") return `Gone from In Progress ${ago(entry.stateSeenAt) ?? ""}: handed in or dropped`;
    return progress === logCounts && entry.detailSeenAt ? `Counts as of ${ago(entry.detailSeenAt)}` : `In progress as of ${ago(entry.stateSeenAt) ?? "the last look"}`;
  });

  /** What the game showed for this quest that the data says otherwise. Shown only, kept on this PC. */
  const differences = $derived.by(() => {
    if (!sheet || !entry) return [];
    const out: string[] = [];
    if (entry.level != null && sheet.row.levelMin != null && entry.level !== sheet.row.levelMin) {
      out.push(`The game shows Lv ${entry.level}+, the data Lv ${sheet.row.levelMin}.`);
    }
    for (const c of entry.counters) {
      const o = sheet.objectives.find((o) => fold(o.guide.target) === fold(c.name));
      if (o && o.count !== c.need) out.push(`The game wants ${c.need} ${c.name}, the data ${o.count}.`);
    }
    for (const r of entry.rewards) {
      const d = sheet.rewards.find((d) => fold(d.name) === fold(r.name));
      if (!d) out.push(`The game gives ${r.name} ×${r.count}, which the data doesn't list.`);
      else if (d.count !== r.count) out.push(`The game gives ${r.name} ×${r.count}, the data ×${d.count}.`);
    }
    if (entry.rewardExp != null && entry.rewardExp !== sheet.row.exp) {
      out.push(`The game gives ${formatNumber(entry.rewardExp)} EXP, the data ${formatNumber(sheet.row.exp)}.`);
    }
    if (entry.rewardMesos != null && entry.rewardMesos !== sheet.row.mesos) {
      out.push(`The game gives ${formatNumber(entry.rewardMesos)} mesos, the data ${formatNumber(sheet.row.mesos)}.`);
    }
    return out;
  });
  const meta = $derived(
    sheet
      ? [
          sheet.row.levelMin ? `Lv ${sheet.row.levelMin}` : null,
          sheet.row.region,
          sheet.row.repeat ? sheet.row.repeat[0].toUpperCase() + sheet.row.repeat.slice(1) : null,
          sheet.chain,
        ].filter(Boolean)
      : [],
  );
</script>

<Card title="Quest" {plain}>
  {#if sheet}
    <h3 class="name">{sheet.row.name}</h3>
    {#if meta.length}<p class="meta">{meta.join(" · ")}</p>{/if}
    {#if standing}
      <p class="standing"><QuestMark quest={entry} label /><span>{standing}</span></p>
    {/if}
    {#if isFollowed(sheet.row.id)}
      <p class="following">
        <Check size={12} /><span>Following in the Quests tab</span>
        <button class="stop" onclick={() => unfollow(sheet.row.id)}>Stop</button>
      </p>
    {:else}
      <button class="follow" onclick={() => follow(sheet.row.id, sheet.row.name)}><Plus size={12} />Follow</button>
    {/if}

    {#if progress.length}
      <ul class="progress">
        {#each progress as objective (objective.target)}
          {@const done = !objective.unread && objective.current >= objective.required}
          <li class:done>
            <span>{objective.target}</span>
            <span class="tabular">{objective.unread ? "–" : objective.current} / {objective.required}</span>
          </li>
        {/each}
      </ul>
    {/if}

    <QuestSteps {sheet} from={found?.from ?? null} tracked={taken ? progress : null} />

    {#if differences.length}
      <div class="differs">
        <p class="differs-head">The game and the data differ</p>
        <ul>
          {#each differences as line (line)}<li>{line}</li>{/each}
        </ul>
        <p class="differs-foot">Read from the Quest window and kept on this PC. The game is usually right; OCR can misread a number.</p>
      </div>
    {/if}

  {:else if error}
    <p class="note">{error}</p>
  {:else}
    <p class="note"><span class="spin"><LoaderCircle size={13} /></span> Looking it up…</p>
  {/if}
</Card>

<style>
  .progress {
    margin: 0 0 10px;
    padding: 0;
    list-style: none;
  }

  .progress li {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    padding: 5px 10px;
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    font-size: var(--text-sm);
  }

  .progress li + li {
    margin-top: 3px;
  }

  .progress li.done span:last-child {
    color: var(--good);
  }

  .standing {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    margin: 0 0 8px;
    color: var(--muted);
    font-size: var(--text-xs);
  }

  .standing :global(.mark) {
    margin-left: 0;
  }

  .differs {
    margin-top: 10px;
    padding: 8px 10px;
    border-radius: var(--radius-sm);
    background: var(--warn-soft);
    font-size: var(--text-sm);
  }

  .differs ul {
    margin: 4px 0;
    padding-left: 16px;
  }

  .differs-head {
    margin: 0;
    color: var(--warn);
    font-weight: 600;
  }

  .differs-foot {
    margin: 0;
    color: var(--muted);
    font-size: var(--text-xs);
  }

  .name {
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--text-lg);
    font-weight: 600;
    line-height: 1.25;
  }

  .meta {
    margin: 2px 0 6px;
    color: var(--muted);
    font-size: var(--text-sm);
  }

  /* Keeps a quest in the Quests tab, with its hints, while the Quest Helper isn't showing it. */
  .follow {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 26px;
    margin: 0 0 8px;
    padding: 0 9px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: transparent;
    font-size: var(--text-sm);
    cursor: pointer;
  }

  .follow:hover,
  .follow:focus-visible {
    background: var(--surface-3);
    outline: none;
  }

  .following {
    display: flex;
    align-items: center;
    gap: 5px;
    min-height: 26px;
    margin: 0 0 8px;
    color: var(--good);
    font-size: var(--text-sm);
  }

  .stop {
    margin-left: 4px;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--muted);
    font-size: var(--text-sm);
    text-decoration: underline;
    text-decoration-color: var(--border-strong);
    cursor: pointer;
  }

  .stop:hover,
  .stop:focus-visible {
    color: var(--text);
    outline: none;
  }

  .note {
    margin: 0;
    color: var(--faint);
    font-size: var(--text-sm);
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

</style>
