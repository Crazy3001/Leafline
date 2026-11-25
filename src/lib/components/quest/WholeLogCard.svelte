<script lang="ts">
  // Every quest in the data, by region, with where each stands for the character: what the game's
  // Quest window showed where it was read, and the data's levels and earlier quests for the rest.
  // The data doesn't know every condition (jobs, events), so "can take" from it alone is hedged.
  import { untrack } from "svelte";
  import { Check, ChevronDown, ChevronRight, CircleCheck, CircleDot, Lock, Undo2 } from "@lucide/svelte";

  import Card from "../Card.svelte";
  import Ref from "../Ref.svelte";
  import Segmented from "$lib/ui/Segmented.svelte";
  import { backend } from "$lib/api";
  import { formatAgo } from "$lib/format";
  import { mapData } from "$lib/mapdata.svelte";
  import { following } from "$lib/quests.svelte";
  import { questLog, watchQuestLog } from "$lib/questlog.svelte";
  import type { Stand, WholeLog, WholeQuest } from "$lib/types";

  let {
    level = null,
    helper = [],
  }: {
    /** The character's level, for what they can't take yet. */
    level?: number | null;
    /** The quest titles the Quest Helper shows now: they name the step of a series. */
    helper?: string[];
  } = $props();

  watchQuestLog();

  type Filter = "all" | "inProgress" | "canTake" | "notYet" | "done";
  const FILTER_KEY = "leafline.quests.whole";
  const GROUP: Record<Stand, Filter> = {
    ready: "inProgress",
    inProgress: "inProgress",
    canTake: "canTake",
    notYet: "notYet",
    done: "done",
    gone: "done",
  };
  const LABEL: Record<Filter, string> = {
    all: "All",
    inProgress: "In progress",
    canTake: "Can take",
    notYet: "Not yet",
    done: "Done",
  };
  const STAND_LABEL: Record<Stand, string> = {
    ready: "Ready to hand in",
    inProgress: "In progress",
    canTake: "Can take",
    notYet: "Not yet",
    done: "Done",
    gone: "Handed in or dropped",
  };

  function storedFilter(): Filter {
    try {
      const saved = localStorage.getItem(FILTER_KEY);
      if (saved && saved in LABEL) return saved as Filter;
    } catch {
      // Starts on everything.
    }
    return "all";
  }
  let filter = $state<Filter>(storedFilter());
  function setFilter(next: Filter) {
    filter = next;
    try {
      localStorage.setItem(FILTER_KEY, next);
    } catch {
      // A convenience only.
    }
  }

  let whole = $state<WholeLog | null>(null);
  const helperKey = $derived(helper.join("\n"));

  // Asks again whenever the log, the level, the Quest Helper or the map data changes.
  $effect(() => {
    void questLog.log;
    void mapData.status?.dataUpdatedAt;
    const asked = `${following.character}|${level}|${helperKey}`;
    const name = following.character ?? null;
    const at = level;
    const titles = helper;
    untrack(() =>
      backend
        .wholeQuestLog(name, at, titles)
        .then((found) => {
          if (asked === `${following.character}|${level}|${helperKey}`) whole = found;
        })
        .catch(() => {}),
    );
  });

  const counts = $derived.by(() => {
    const n: Record<Filter, number> = { all: 0, inProgress: 0, canTake: 0, notYet: 0, done: 0 };
    for (const r of whole?.regions ?? [])
      for (const q of r.quests) {
        n.all += 1;
        n[GROUP[q.stand]] += 1;
      }
    return n;
  });
  const options = $derived(
    (Object.keys(LABEL) as Filter[]).map((id) => ({ id, label: `${LABEL[id]} ${counts[id].toLocaleString()}` })),
  );

  /** In progress first, then what can be taken, what can't yet, and what's done. */
  const ORDER: Record<Stand, number> = { ready: 0, inProgress: 1, canTake: 2, notYet: 3, gone: 4, done: 5 };
  const regions = $derived(
    (whole?.regions ?? [])
      .map((r) => ({
        ...r,
        shown: r.quests
          .filter((q) => filter === "all" || GROUP[q.stand] === filter)
          .sort((a, b) => ORDER[a.stand] - ORDER[b.stand]),
        active: r.quests.filter((q) => GROUP[q.stand] === "inProgress").length,
      }))
      .filter((r) => r.shown.length)
      .sort((a, b) => b.active - a.active || a.region.localeCompare(b.region)),
  );

  /** Regions opened or closed by hand; the rest follow the filter. */
  let opened = $state<Record<string, boolean>>({});
  const isOpen = (region: string, active: number) => opened[region] ?? (filter !== "all" || active > 0);
  function toggle(region: string, active: number) {
    opened = { ...opened, [region]: !isOpen(region, active) };
  }

  const ago = (at: number) => formatAgo(Date.now() / 1000 - at);
  const repeatLabel = (r: string | null) => (r === "daily" ? "Daily" : r === "weekly" ? "Weekly" : r ? "Repeatable" : null);
  const key = (q: WholeQuest) => (q.questId != null ? `#${q.questId}` : `t:${q.title}`);
</script>

<Card title="Every quest">
  {#snippet aside()}
    {#if whole?.level != null}<span>Lv {whole.level}</span>{/if}
  {/snippet}

  {#if !whole}
    <p class="lead">Loading the quest data…</p>
  {:else if !whole.regions.length}
    <p class="lead">Every quest shows here once the map data is downloaded.</p>
  {:else}
    <p class="lead">
      {#if whole.seen}
        What you've done and are on, as your Quest window last showed it. The rest is from the quest data, which knows levels
        and earlier quests but not jobs or events, so "can take" may be wrong until you look through your Available tab.
      {:else}
        From the quest data alone. Open the Quest window in game and Leafline marks what you've done and what you're on.
      {/if}
      {#if whole.level == null}Your level isn't read yet, so levels don't count against a quest.{/if}
    </p>

    <div class="filters"><Segmented label="Which quests" size="sm" options={options} value={filter} onchange={setFilter} /></div>

    {#each regions as r (r.region)}
      {@const open = isOpen(r.region, r.active)}
      <section>
        <button class="region" aria-expanded={open} onclick={() => toggle(r.region, r.active)}>
          {#if open}<ChevronDown size={13} />{:else}<ChevronRight size={13} />{/if}
          <span class="region-name">{r.region}</span>
          <span class="tabular done-count">{r.done}<span class="faint">{` of ${r.total} done`}</span></span>
          <span class="bar"><span style:width="{r.total ? Math.min(100, (100 * r.done) / r.total) : 0}%"></span></span>
        </button>
        {#if open}
          <ul class="rows">
            {#each r.shown as q (key(q))}
              {@const repeat = repeatLabel(q.repeat)}
              <li class="stand-{q.stand}">
                <span class="icon" title={STAND_LABEL[q.stand]} aria-label={STAND_LABEL[q.stand]}>
                  {#if q.stand === "done"}<Check size={12} strokeWidth={2.6} />
                  {:else if q.stand === "ready"}<CircleCheck size={12} strokeWidth={2.4} />
                  {:else if q.stand === "inProgress"}<CircleDot size={12} strokeWidth={2.4} />
                  {:else if q.stand === "gone"}<Undo2 size={12} strokeWidth={2.4} />
                  {:else if q.stand === "notYet"}<Lock size={11} strokeWidth={2.4} />
                  {:else}<span class="bang">!</span>{/if}
                </span>
                <div class="body">
                  <div class="name">
                    {#if q.questId != null}<Ref kind="quest" id={q.questId} name={q.title} mark={false} />{:else}<span>{q.title}</span>{/if}
                  </div>
                  <p class="sub">
                    {#if q.stand === "ready"}<span class="good">Ready to hand in</span>{/if}
                    {#if q.series}<span>Listed as {q.series}{q.stand === "inProgress" ? ", likely this step" : ""}</span>{/if}
                    {#if q.notYet?.level != null}<span class="warn">Lv {q.notYet.level}</span>{/if}
                    {#if q.notYet?.after.length}
                      <span
                        >After {#each q.notYet.after as before, i (before.id)}{#if i}{", "}{/if}<Ref
                            kind="quest"
                            id={before.id}
                            name={before.name}
                            mark={false}
                          />{/each}</span
                      >
                    {/if}
                    {#if q.notYet?.notOfferedAt != null}
                      <span>Not on your Available tab {ago(q.notYet.notOfferedAt)}</span>
                    {/if}
                    {#if q.inferred}<span>Done, since a later quest is</span>{/if}
                    {#if q.questId == null}<span>Not in the quest data</span>{/if}
                    {#if q.levelMin != null && q.notYet?.level == null}<span>Lv {q.levelMin}</span>{/if}
                    {#if q.giver && (q.stand === "canTake" || q.stand === "notYet")}<span>From {q.giver}</span>{/if}
                    {#if repeat}<span>{repeat}</span>{/if}
                  </p>
                </div>
              </li>
            {/each}
          </ul>
        {/if}
      </section>
    {/each}
  {/if}
</Card>

<style>
  .lead {
    margin: 0 0 8px;
    color: var(--muted);
    font-size: var(--text-md);
  }

  .filters {
    margin-bottom: 8px;
  }

  section + section {
    border-top: 1px solid var(--border);
  }

  .region {
    display: grid;
    grid-template-columns: auto 1fr auto 64px;
    align-items: center;
    gap: 6px;
    width: 100%;
    padding: 6px 0;
    border: 0;
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: var(--text-md);
    text-align: left;
    cursor: pointer;
  }

  .region:hover .region-name,
  .region:focus-visible .region-name {
    text-decoration: underline;
    text-decoration-color: var(--border-strong);
  }

  .region:focus-visible {
    outline: none;
  }

  .region-name {
    font-weight: var(--weight-strong);
  }

  .done-count {
    font-size: var(--text-xs);
  }

  .bar {
    display: block;
    height: 4px;
    background: var(--surface-3);
    overflow: hidden;
  }

  .bar span {
    display: block;
    height: 100%;
    background: var(--good);
  }

  .rows {
    margin: 0 0 6px;
    padding: 0 0 0 19px;
    list-style: none;
  }

  .rows li {
    display: grid;
    grid-template-columns: 16px 1fr;
    gap: 4px;
    padding: 3px 0;
  }

  .icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    height: 18px;
    color: var(--faint);
  }

  .stand-done .icon,
  .stand-ready .icon {
    color: var(--good);
  }

  .stand-inProgress .icon {
    color: var(--accent-text);
  }

  .stand-canTake .icon {
    color: var(--warn);
  }

  .bang {
    font-size: var(--text-xs);
    font-weight: var(--weight-strong);
  }

  .name {
    font-size: var(--text-md);
    font-weight: var(--weight-strong);
  }

  .stand-done .name,
  .stand-gone .name {
    font-weight: var(--weight-body);
  }

  .sub {
    display: flex;
    flex-wrap: wrap;
    gap: 0 8px;
    margin: 0;
    color: var(--muted);
    font-size: var(--text-xs);
  }

  .sub:empty {
    display: none;
  }

  .good {
    color: var(--good);
  }

  .warn {
    color: var(--warn);
  }

  .faint {
    color: var(--faint);
  }
</style>
