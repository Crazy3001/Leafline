<script lang="ts">
  // Quests the player follows by hand, shown like the tracked ones: what to hunt or collect and
  // where, and who takes it in. Their lines come from the map data, since the Quest Helper isn't
  // showing them, so there are no counts. A quest the Quest Helper does show is left to it.
  import { untrack } from "svelte";

  import QuestCard from "../QuestCard.svelte";
  import { backend } from "$lib/api";
  import { followedQuests } from "$lib/quests.svelte";
  import type { MapReading, Quest, QuestReading, QuestSheet, SessionSummary } from "$lib/types";

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

  /** The data's page for each followed quest, by id. */
  let sheets = $state<Record<number, QuestSheet>>({});

  const name = (text: string) => text.trim().toLowerCase();
  const inHelper = $derived(new Set((tracked?.quests ?? []).map((q) => name(q.title)).filter(Boolean)));
  const shown = $derived(followedQuests().filter((f) => !inHelper.has(name(f.name))));
  const ids = $derived(shown.map((f) => f.id));
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

  function lines(sheet: QuestSheet): Quest {
    return {
      title: sheet.row.name,
      objectives: sheet.objectives.map((o) => ({ target: o.guide.target, current: 0, required: o.count, unread: true })),
    };
  }

  const ready = $derived(shown.filter((f) => sheets[f.id]));
  const reading = $derived<QuestReading>({
    quests: ready.map((f) => lines(sheets[f.id])),
    page: null,
    panel: { x: 0, y: 0, w: 0, h: 0 },
  });
</script>

{#if ready.length}
  <QuestCard quest={reading} followed={ready.map((f) => f.id)} {map} {session} {plain} />
{/if}
