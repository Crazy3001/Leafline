<script lang="ts">
  import { Hourglass, LoaderCircle, Search, UserRound } from "@lucide/svelte";
  import { untrack } from "svelte";

  import Card from "./Card.svelte";
  import ObjectiveHint from "./quest/ObjectiveHint.svelte";
  import { backend } from "$lib/api";
  import { formatCompact, formatDuration, splitMapLabel } from "$lib/format";
  import { measuredRates, watchHistory } from "$lib/history.svelte";
  import { OSMS_URL, WIKI_URL, mapData, watchMapData } from "$lib/mapdata.svelte";
  import type { Guide, MapReading, ObjectiveGuide, Objective, QuestReading, SessionSummary } from "$lib/types";

  let {
    quest,
    map = null,
    session = null,
    plain = false,
  }: {
    quest: QuestReading;
    /** The minimap reading, so routes can start from the current map. */
    map?: MapReading | null;
    /** The session so far, for the player's own pace on each objective. */
    session?: SessionSummary | null;
    plain?: boolean;
  } = $props();

  watchMapData();
  const ready = $derived(mapData.status?.ready ?? false);

  let guide = $state<Guide | null>(null);
  let lookingUp = $state(false);
  let lookupError = $state<string | null>(null);

  watchHistory();
  // EXP rates from saved sessions and this one, for a rough pace where no counter was timed.
  const measured = $derived(measuredRates(session));

  const queries = $derived(
    quest.quests.map((q) => ({ title: q.title, objectives: q.objectives.map((o) => o.target) })),
  );
  const queryKey = $derived(JSON.stringify([queries, map?.name, map?.street, measured]));

  async function refresh() {
    const key = queryKey;
    const result = await backend.questGuide(queries, map?.name ?? null, map?.street ?? null, measured);
    if (key === queryKey) guide = result;
  }

  /** Below this many kills or items an hour, an estimate says more about luck than pace. */
  const MIN_PER_HOUR = 1;

  /**
   * Time left on an objective at the player's measured pace, or null when nothing was measured.
   * Best is the counter's own rise, timed by the session tracker (on this map if it was timed
   * here); otherwise, for a monster, the EXP rate measured on one of its maps turned into kills.
   */
  function timeLeft(read: Objective, hint: ObjectiveGuide | undefined) {
    const left = read.required - read.current;
    if (left <= 0) return null;
    const timed = (session?.counters ?? [])
      .filter((c) => c.target === read.target && c.required === read.required && (c.perHour ?? 0) >= MIN_PER_HOUR)
      .sort((a, b) => Number(b.map === session?.map) - Number(a.map === session?.map) || b.activeSeconds - a.activeSeconds);
    const counter = timed[0];
    if (counter?.perHour) {
      const where = counter.map && counter.map !== session?.map ? ` on ${splitMapLabel(counter.map).name}` : "";
      return {
        seconds: (left / counter.perHour) * 3600,
        where,
        basis: "counter" as const,
        title:
          `Your counter rose by ${counter.gained} in ${formatDuration(counter.activeSeconds)} of active play` +
          `${counter.map ? ` on ${splitMapLabel(counter.map).name}` : ""}. A guide only: pace varies.`,
      };
    }
    if (hint?.kind !== "monster") return null;
    const grounds = hint.hunts[0]?.maps.filter((g) => g.pace && g.pace.killsPerHour >= MIN_PER_HOUR) ?? [];
    const ground =
      grounds.find((g) => g.map.id === guide?.from?.id) ??
      [...grounds].sort((a, b) => b.pace!.activeSeconds - a.pace!.activeSeconds)[0];
    if (!ground?.pace) return null;
    return {
      seconds: (left / ground.pace.killsPerHour) * 3600,
      where: ground.map.id === guide?.from?.id ? "" : ` on ${ground.map.name}`,
      basis: "exp" as const,
      title:
        `Estimated from the ${formatCompact(ground.pace.expPerHour)} EXP/hr you measured on ${ground.map.name} ` +
        `over ${formatDuration(ground.pace.activeSeconds)}, assuming you defeat its monsters in proportion to how many ` +
        `spawn, at the map data's base EXP. EXP events make it read short; skipping other monsters makes it read long.`,
    };
  }

  // Follows the key alone: every live reading brings new objects with the same content.
  $effect(() => {
    void queryKey;
    void mapData.status?.dataUpdatedAt;
    if (!ready) {
      guide = null;
      return;
    }
    untrack(refresh).catch(() => (guide = null));
  });

  /** Unfinished item objectives nobody has asked the wiki about yet. */
  const unknownItems = $derived(
    guide
      ? guide.quests.flatMap((g, i) =>
          g.objectives.filter((o, j) => {
            const read = quest.quests[i]?.objectives[j];
            return o.kind === "item" && !o.lookedUp && read && read.current < read.required;
          }),
        )
      : [],
  );
  const showsDrops = $derived(guide?.quests.some((g) => g.objectives.some((o) => o.kind === "item" && o.hunts.length)));

  async function lookUp() {
    lookingUp = true;
    lookupError = null;
    try {
      await backend.lookUpQuestItems([...new Set(unknownItems.map((o) => o.target))]);
      await refresh();
    } catch (e) {
      lookupError = String(e);
    } finally {
      lookingUp = false;
    }
  }
</script>

<Card title="Quests" {plain}>
  {#snippet aside()}
    {#if quest.page && !plain}<span class="tabular">{quest.page[0]} tracked</span>{/if}
  {/snippet}

  {#each quest.quests as { title, objectives }, i}
    {@const info = guide?.quests[i]}
    {@const finished = objectives.length > 0 && objectives.every((o) => o.current >= o.required)}
    <div class="quest" class:dense={plain}>
      <!-- The map data's spelling, once the title matched a quest there: OCR garbles some letters. -->
      <h3 class="title" title={info?.quest ? `Read on screen as "${title}"` : undefined}>
        {info?.quest?.quest.name || title || "Untitled quest"}
      </h3>
      {#if info?.quest?.giver}
        <p class="giver" class:finished>
          <UserRound size={11} />
          <span>
            {finished ? "Turn in to" : "From"}
            {info.quest.giver}{#if info.quest.giverMaps.length}<span class="faint"
                >{" · "}{info.quest.giverMaps.map((m) => m.name).join(", ")}</span
              >{/if}
          </span>
        </p>
      {/if}
      {#if objectives.length}
        <ul>
          {#each objectives as objective, j}
            {@const done = objective.current >= objective.required}
            {@const hint = info?.objectives[j]}
            {@const eta = timeLeft(objective, hint)}
            <li class:done>
              <div class="row">
                <span>{objective.target}</span>
                <span class="count tabular">{objective.current} / {objective.required}</span>
              </div>
              <div class="track">
                <div class="fill" style:width="{Math.min(1, objective.current / objective.required) * 100}%"></div>
              </div>
              {#if eta}
                <p class="eta" title={eta.title}>
                  <Hourglass size={10} />
                  <span>
                    About {formatDuration(Math.max(60, eta.seconds))} left{eta.where}
                    <span class="faint">· {eta.basis === "counter" ? "your pace" : "rough"}</span>
                  </span>
                </p>
              {/if}
              {#if hint && !done}<ObjectiveHint guide={hint} from={guide?.from ?? null} />{/if}
            </li>
          {/each}
        </ul>
      {:else}
        <p class="empty">Counters could not be read.</p>
      {/if}
    </div>
  {:else}
    <p class="empty">The Quest Helper was found, but its text could not be read.</p>
  {/each}

  {#if quest.quests.length}
    {#if !ready}
      {#if !plain}
        <p class="footer">Download map data from the Map tab to see where to hunt for each objective.</p>
      {/if}
    {:else if unknownItems.length}
      <div class="footer lookup">
        <button onclick={lookUp} disabled={lookingUp}>
          {#if lookingUp}<span class="spin"><LoaderCircle size={12} /></span>{:else}<Search size={12} />{/if}
          Find who drops {unknownItems.length === 1 ? "the quest item" : `${unknownItems.length} quest items`}
        </button>
        <span class="faint">Asks mapleclassic.wiki</span>
      </div>
      {#if lookupError}<p class="footer error">{lookupError}</p>{/if}
    {/if}
    {#if guide && !plain}
      <p class="credit">
        Maps: <button onclick={() => backend.openLink(OSMS_URL)}>OSMS</button> (CC BY 4.0){#if showsDrops}{" · "}Drop rates players logged on <button onclick={() => backend.openLink(WIKI_URL)}>mapleclassic.wiki</button>{/if}
      </p>
    {/if}
  {/if}
</Card>

<style>
  .quest + .quest {
    margin-top: 12px;
    padding-top: 10px;
    border-top: 1px solid var(--border);
  }

  .title {
    margin: 0 0 8px;
    font-family: var(--font-display);
    font-size: 14px;
    font-weight: 600;
  }

  .giver {
    display: flex;
    align-items: center;
    gap: 5px;
    margin: -5px 0 8px;
    color: var(--muted);
    font-size: 11.5px;
  }

  .giver.finished {
    color: var(--good);
  }

  .faint {
    color: var(--faint);
  }

  ul {
    display: grid;
    gap: 8px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .row {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 4px;
  }

  .count {
    flex: none;
    color: var(--muted);
  }

  .done .count {
    color: var(--good);
  }

  .track {
    height: 4px;
    border-radius: var(--radius-sm);
    background: rgba(255, 255, 255, 0.06);
    overflow: hidden;
  }

  .fill {
    height: 100%;
    background: var(--accent);
  }

  .eta {
    display: flex;
    align-items: center;
    gap: 5px;
    margin: 4px 0 0;
    color: var(--muted);
    font-size: 11px;
  }

  .eta :global(svg) {
    flex: none;
    color: var(--faint);
  }

  .eta .faint {
    color: var(--faint);
  }

  .done .fill {
    background: var(--good);
  }

  /* Pinned: the same layout, packed tighter. */
  .dense .title {
    margin-bottom: 4px;
    font-size: 12.5px;
  }

  .dense .giver {
    margin: -2px 0 5px;
  }

  .dense ul {
    gap: 5px;
  }

  .dense .row {
    margin-bottom: 2px;
    font-size: 12px;
  }

  .dense .track {
    height: 3px;
  }

  .quest.dense + .quest.dense {
    margin-top: 8px;
    padding-top: 7px;
  }

  .empty {
    margin: 0;
    color: var(--faint);
    font-size: 12px;
  }

  .footer {
    margin: 12px 0 0;
    padding-top: 10px;
    border-top: 1px solid var(--border);
    color: var(--faint);
    font-size: 11.5px;
  }

  .lookup {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }

  .lookup button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: rgba(255, 255, 255, 0.04);
    color: var(--text);
    font-size: 12px;
    cursor: pointer;
  }

  .lookup button:hover:not(:disabled) {
    border-color: var(--accent);
  }

  .lookup button:disabled {
    cursor: progress;
    opacity: 0.7;
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

  .error {
    border-top: 0;
    padding-top: 0;
    margin-top: 6px;
    color: var(--bad);
  }

  .credit {
    margin: 10px 0 0;
    color: var(--faint);
    font-size: 10.5px;
  }

  .credit button {
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--muted);
    font-size: inherit;
    cursor: pointer;
  }

  .credit button:hover {
    color: var(--text);
    text-decoration: underline;
  }
</style>
