<script lang="ts">
  // Every quest the character's quest log shows in progress, not only the Quest Helper's five,
  // shown like the tracked ones: what to hunt or collect and where, and who takes it in. Counts are
  // the detail pane's, as of the last time it was open on the quest; a quest the Quest Helper or
  // the followed list already shows is left to them.
  import { untrack } from "svelte";

  import QuestCard from "../QuestCard.svelte";
  import { backend } from "$lib/api";
  import { formatAgo } from "$lib/format";
  import { followedQuests } from "$lib/quests.svelte";
  import { fold, questLog, watchQuestLog } from "$lib/questlog.svelte";
  import type { LoggedQuest, MapReading, Quest, QuestReading, QuestSheet, SessionSummary } from "$lib/types";

  let {
    tracked = null,
    map = null,
    session = null,
    plain = false,
  }: {
    /** What the Quest Helper shows now. */
    tracked?: QuestReading | null;
    map?: MapReading | null;
    session?: SessionSummary | null;
    plain?: boolean;
  } = $props();

  watchQuestLog();

  let sheets = $state<Record<number, QuestSheet>>({});

  const inHelper = $derived(new Set((tracked?.quests ?? []).map((q) => fold(q.title)).filter(Boolean)));
  const followedIds = $derived(new Set(followedQuests().map((f) => f.id)));
  const shown = $derived(
    (questLog.log?.quests ?? []).filter(
      (q) => q.state === "inProgress" && q.questId != null && !inHelper.has(fold(q.title)) && !followedIds.has(q.questId),
    ),
  );
  const ids = $derived(shown.map((q) => q.questId!));
  const key = $derived(ids.join(","));

  // Follows the key alone: every live reading brings new objects with the same content.
  $effect(() => {
    const asked = key;
    untrack(() => {
      const missing = ids.filter((id) => !sheets[id]);
      Promise.all(missing.map((id) => backend.questSheet(id, null, null, null)))
        .then((found) => {
          if (asked !== key) return;
          const next = { ...sheets };
          found.forEach((where) => {
            if (where) next[where.sheet.row.id] = where.sheet;
          });
          sheets = next;
        })
        .catch(() => {
          // A quest the data no longer has stays out until it's found again.
        });
    });
  });

  /** The data's objectives, with the counts the detail pane last showed where its names match. */
  function lines(sheet: QuestSheet, logged: LoggedQuest): Quest {
    return {
      title: sheet.row.name,
      objectives: sheet.objectives.map((o) => {
        const seen = logged.counters.find((c) => fold(c.name) === fold(o.guide.target));
        return seen
          ? { target: o.guide.target, current: seen.have, required: seen.need }
          : { target: o.guide.target, current: 0, required: o.count, unread: true };
      }),
    };
  }

  const ready = $derived(shown.filter((q) => sheets[q.questId!]));
  const reading = $derived<QuestReading>({
    quests: ready.map((q) => lines(sheets[q.questId!], q)),
    page: null,
    panel: { x: 0, y: 0, w: 0, h: 0 },
  });
  const seenAt = $derived(Math.max(0, ...ready.map((q) => q.detailSeenAt ?? q.stateSeenAt ?? 0)));
</script>

{#if ready.length}
  <QuestCard
    quest={reading}
    heading="In your quest log"
    note={seenAt ? `Counts as of ${formatAgo(Date.now() / 1000 - seenAt)}` : "No counts"}
    {map}
    {session}
    {plain}
  />
{/if}
