<script lang="ts">
  // Where a quest stands in the character's quest log, as the game's Quest window last showed it:
  // done, in progress, ready to hand in, or available. Shows nothing for a quest the log hasn't
  // seen, or with no log yet.
  import { Check, CircleDot, CircleCheck, Undo2 } from "@lucide/svelte";

  import { tip } from "$lib/tip.svelte";
  import { formatAgo } from "$lib/format";
  import { MARK_LABEL, logged, markOf, markSeenAt, watchQuestLog } from "$lib/questlog.svelte";
  import type { LoggedQuest } from "$lib/types";

  let {
    id = null,
    quest = null,
    label = false,
  }: {
    /** The quest in the data. */
    id?: number | null;
    /** Or the logged quest itself, for one the data doesn't have. */
    quest?: LoggedQuest | null;
    /** Say it in words beside the icon. */
    label?: boolean;
  } = $props();

  watchQuestLog();

  const entry = $derived(quest ?? logged(id));
  const mark = $derived(markOf(entry));
  const seen = $derived(entry ? markSeenAt(entry) : null);
  const words = $derived.by(() => {
    if (!mark || !entry) return "";
    const when = seen != null ? `, as of ${formatAgo(Date.now() / 1000 - seen)} in the Quest window` : "";
    const on = mark === "done" && entry.doneOn ? ` on ${entry.doneOn}` : "";
    return `${MARK_LABEL[mark]}${on}${when}`;
  });
</script>

{#if mark}
  <span class="mark mark-{mark}" class:label use:tip={label ? null : words} aria-label={label ? undefined : words}>
    {#if mark === "done"}<Check size={11} strokeWidth={2.6} />
    {:else if mark === "ready"}<CircleCheck size={11} strokeWidth={2.4} />
    {:else if mark === "gone"}<Undo2 size={11} strokeWidth={2.4} />
    {:else if mark === "available"}<span class="bang">!</span>
    {:else}<CircleDot size={11} strokeWidth={2.4} />{/if}{#if label}<span>{MARK_LABEL[mark]}</span>{/if}
  </span>
{/if}

<style>
  .mark {
    display: inline-flex;
    flex: none;
    align-items: center;
    gap: 3px;
    margin-left: 4px;
    color: var(--faint);
    font-size: var(--text-xs);
    font-weight: var(--weight-strong);
    line-height: 1;
    vertical-align: -1px;
  }

  .mark.label {
    padding: 2px 6px 2px 5px;
    border-radius: 999px;
    background: var(--surface-3);
  }

  .mark-done,
  .mark-ready {
    color: var(--good);
  }

  .mark-done.label,
  .mark-ready.label {
    background: var(--good-soft);
  }

  .mark-inProgress {
    color: var(--accent-text);
  }

  .mark-available {
    color: var(--warn);
  }

  .mark-available.label {
    background: var(--warn-soft);
  }

  .bang {
    width: 11px;
    font-weight: var(--weight-strong);
    text-align: center;
  }
</style>
