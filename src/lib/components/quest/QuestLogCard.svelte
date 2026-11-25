<script lang="ts">
  // The character's quest log at a glance, as the game's Quest window last showed it: what is
  // ready to hand in and to whom, what else is in progress, and how much of each region is done.
  // Only true as of the last look, which the card says.
  import { untrack } from "svelte";
  import { CircleCheck } from "@lucide/svelte";

  import Card from "../Card.svelte";
  import Ref from "../Ref.svelte";
  import GoButton from "../GoButton.svelte";
  import { backend } from "$lib/api";
  import { can } from "$lib/platform";
  import { npcPlace } from "$lib/destination.svelte";
  import { formatAgo } from "$lib/format";
  import { hasLog, look, questLog, readyToHandIn, watchQuestLog } from "$lib/questlog.svelte";
  import type { LoggedQuest, QuestSheet, QuestTab } from "$lib/types";

  let { plain = false }: { plain?: boolean } = $props();

  watchQuestLog();

  const TABS: { tab: QuestTab; label: string }[] = [
    { tab: "available", label: "Available" },
    { tab: "inProgress", label: "In Progress" },
    { tab: "completed", label: "Completed" },
  ];
  /** Regions listed before "Show all". */
  const REGIONS_SHOWN = 6;

  const log = $derived(questLog.log);
  const quests = $derived(log?.quests ?? []);
  const inProgress = $derived(quests.filter((q) => q.state === "inProgress"));
  const ready = $derived(inProgress.filter(readyToHandIn));
  const rest = $derived(inProgress.filter((q) => !readyToHandIn(q)));
  const lastLook = $derived(Math.max(0, ...(log?.looks ?? []).map((l) => l.seenAt)));

  /** The data's page for each quest ready to hand in, for who takes it. */
  let sheets = $state<Record<number, QuestSheet>>({});
  const readyIds = $derived(ready.map((q) => q.questId).filter((id): id is number => id != null));
  const readyKey = $derived(readyIds.join(","));
  $effect(() => {
    const asked = readyKey;
    untrack(() => {
      const missing = readyIds.filter((id) => !sheets[id]);
      Promise.all(missing.map((id) => backend.questSheet(id, null, null, null)))
        .then((found) => {
          if (asked !== readyKey) return;
          const next = { ...sheets };
          found.forEach((where) => {
            if (where) next[where.sheet.row.id] = where.sheet;
          });
          sheets = next;
        })
        .catch(() => {});
    });
  });

  /**
   * Each region with what is done there: the Completed tab's own count when its bar read,
   * otherwise the done quests the log holds there, against what the data files under it.
   */
  const regions = $derived.by(() => {
    const totals = new Map(questLog.totals.map((t) => [t.region, t.quests]));
    const counted = new Map<string, number>();
    for (const r of log?.regions ?? []) if (r.tab === "completed") counted.set(r.region, r.count);
    for (const q of quests) {
      if (!q.done || !q.region || (log?.regions ?? []).some((r) => r.tab === "completed" && r.region === q.region)) continue;
      counted.set(q.region, (counted.get(q.region) ?? 0) + 1);
    }
    return [...counted.entries()]
      .map(([region, done]) => ({ region, done, total: totals.get(region) ?? null }))
      .sort((a, b) => b.done - a.done || a.region.localeCompare(b.region));
  });
  let allRegions = $state(false);

  const ago = (at: number) => formatAgo(Date.now() / 1000 - at);

  function counters(q: LoggedQuest) {
    return q.counters.map((c) => `${c.name} ${c.have}/${c.need}`).join(" · ");
  }

  async function forget() {
    if (log?.character) await backend.forgetQuestLog(log.character);
  }
</script>

<Card title="Quest log" {plain}>
  {#snippet aside()}
    {#if lastLook}<span>As of {ago(lastLook)}</span>{/if}
  {/snippet}

  {#if !hasLog() && !can.liveReading}
    <p class="lead">
      Share the game window on Now (Read the game), then open your Quest window and scroll each tab to its end: the
      website keeps what it lists. A log the Windows program read also comes along in a records file (Settings, Data).
    </p>
  {:else if !hasLog()}
    <p class="lead">
      Open the Quest window in game and Leafline reads it while it's open: what you've done, what's in progress and what
      you can take. Scroll each tab to its end once so it sees every row.
    </p>
  {:else}
    <p class="looks">
      {#each TABS as { tab, label }, i (tab)}
        {@const seen = look(tab)}
        {#if i}<span class="faint">{" · "}</span>{/if}
        <span class:whole={!!seen?.wholeAt} class:unseen={!seen}>{label}</span>
        <span class="faint">{" "}{#if !seen}not seen{:else if seen.wholeAt}{ago(seen.wholeAt)}{:else}partly seen{/if}</span>
      {/each}
    </p>

    {#if ready.length}
      <h4><CircleCheck size={12} /> Ready to hand in</h4>
      <ul class="rows">
        {#each ready as q (q.key)}
          {@const info = q.questId != null ? sheets[q.questId]?.info : null}
          <li>
            <div class="name">
              {#if q.questId != null}<Ref kind="quest" id={q.questId} name={q.title} />{:else}<span>{q.title}</span>{/if}
            </div>
            <p class="sub">
              {#if info?.turnIn}
                Turn in to <Ref kind="npc" name={info.turnIn} />
                {#if info.turnInMaps[0]}<span class="faint">{" · "}</span><Ref kind="map" id={info.turnInMaps[0].id} name={info.turnInMaps[0].name} />{/if}
                <GoButton place={npcPlace(info.turnIn, info.turnInMaps)} />
              {:else if info?.giver}
                Hand it in <span class="faint">· given by</span> <Ref kind="npc" name={info.giver} />
                <GoButton place={npcPlace(info.giver, info.giverMaps)} />
              {:else if q.counters.length}
                {counters(q)}
              {:else}
                <span class="faint">The Quest window shows it ready</span>
              {/if}
            </p>
          </li>
        {/each}
      </ul>
    {/if}

    {#if rest.length}
      <h4>In progress</h4>
      <ul class="rows">
        {#each rest as q (q.key)}
          <li>
            <div class="name">
              {#if q.questId != null}<Ref kind="quest" id={q.questId} name={q.title} />{:else}<span>{q.title}{q.cut ? "…" : ""}</span>{/if}
              {#if q.onHelper}<span class="faint small">{" · on the Quest Helper"}</span>{/if}
            </div>
            <p class="sub">
              {#if q.counters.length}
                {counters(q)} <span class="faint">{`· ${ago(q.detailSeenAt ?? 0)}`}</span>
              {:else if q.questId == null}
                <span class="faint">Not in the map data</span>
              {:else}
                <span class="faint">Pick it in the Quest window to read its counts</span>
              {/if}
            </p>
          </li>
        {/each}
      </ul>
    {/if}

    {#if regions.length}
      <h4>Done by region</h4>
      <ul class="regions">
        {#each allRegions ? regions : regions.slice(0, REGIONS_SHOWN) as r (r.region)}
          <li>
            <span>{r.region}</span>
            <span class="tabular">{r.done}{#if r.total != null}<span class="faint">{` of ${r.total}`}</span>{/if}</span>
            {#if r.total}<span class="bar"><span style:width="{Math.min(100, (100 * r.done) / r.total)}%"></span></span>{/if}
          </li>
        {/each}
      </ul>
      {#if regions.length > REGIONS_SHOWN}
        <button class="link" onclick={() => (allRegions = !allRegions)}>{allRegions ? "Show fewer" : `Show all ${regions.length}`}</button>
      {/if}
    {/if}

    <p class="foot faint">
      Read from the Quest window, so true as of the last look.
      <button class="link" onclick={forget}>Forget this log</button>
    </p>
  {/if}
</Card>

<style>
  .lead {
    margin: 0;
    color: var(--muted);
    font-size: var(--text-sm);
  }

  .looks {
    margin: 0 0 6px;
    color: var(--muted);
    font-size: var(--text-xs);
  }

  .looks .whole {
    color: var(--text);
  }

  .looks .unseen {
    color: var(--faint);
  }

  h4 {
    display: flex;
    align-items: center;
    gap: 5px;
    margin: 10px 0 4px;
    color: var(--muted);
    font-family: var(--font-display);
    font-size: var(--text-sm);
    font-weight: 600;
  }

  h4 :global(svg) {
    color: var(--good);
  }

  .rows,
  .regions {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .rows li {
    padding: 4px 0;
  }

  .rows li + li {
    border-top: 1px solid var(--border);
  }

  .name {
    font-size: var(--text-md);
    font-weight: 600;
  }

  .sub {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0 4px;
    margin: 1px 0 0;
    color: var(--muted);
    font-size: var(--text-xs);
  }

  .small {
    font-size: var(--text-xs);
    font-weight: 400;
  }

  .regions li {
    display: grid;
    grid-template-columns: 1fr auto 64px;
    align-items: center;
    gap: 8px;
    padding: 2px 0;
    font-size: var(--text-sm);
  }

  .bar {
    height: 4px;
    border-radius: 999px;
    background: var(--surface-3);
    overflow: hidden;
  }

  .bar span {
    display: block;
    height: 100%;
    border-radius: 999px;
    background: var(--good);
  }

  .link {
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--muted);
    font-size: var(--text-xs);
    text-decoration: underline;
    text-decoration-color: var(--border-strong);
    cursor: pointer;
  }

  .link:hover,
  .link:focus-visible {
    color: var(--text);
    outline: none;
  }

  .foot {
    margin: 10px 0 0;
    font-size: var(--text-xs);
  }

  .faint {
    color: var(--faint);
  }
</style>
