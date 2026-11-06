<script lang="ts">
  import { Check, ChevronRight, Gift, Hourglass, LoaderCircle, Search, UserRound } from "@lucide/svelte";
  import { untrack } from "svelte";

  import Card from "./Card.svelte";
  import ObjectiveHint from "./quest/ObjectiveHint.svelte";
  import QuestSteps from "./quest/QuestSteps.svelte";
  import { backend } from "$lib/api";
  import { formatCompact, formatDuration, objectiveProgress, splitMapLabel } from "$lib/format";
  import { measuredRates, watchHistory } from "$lib/history.svelte";
  import { showItemNamed } from "$lib/items.svelte";
  import { showMonsterNamed } from "$lib/monsters.svelte";
  import { showNpcNamed } from "$lib/places.svelte";
  import { OSMS_URL, WIKI_URL, mapData, watchMapData } from "$lib/mapdata.svelte";
  import { payout, questView, toggleExpanded } from "$lib/quests.svelte";
  import type { Guide, MapRef, MapReading, ObjectiveGuide, Objective, QuestReading, QuestWhere, SessionSummary } from "$lib/types";

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
  /** The data's page for each quest, by id. */
  let sheets = $state<Record<number, QuestWhere>>({});

  watchHistory();
  // EXP rates from saved sessions and this one, for a rough pace where no counter was timed.
  const measured = $derived(measuredRates(session));

  const queries = $derived(
    quest.quests.map((q) => ({ title: q.title, objectives: q.objectives.map((o) => o.target) })),
  );
  const queryKey = $derived(JSON.stringify([queries, map?.name, map?.street, map?.twin, measured]));

  async function refresh() {
    const key = queryKey;
    const result = await backend.questGuide(queries, map?.name ?? null, map?.street ?? null, map?.twin ?? null, measured);
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
    if (left <= 0 || read.unread) return null;
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

  const sheetKey = $derived(
    JSON.stringify([guide?.quests.map((g) => g.quest?.quest.id), map?.name, map?.street, map?.twin]),
  );

  // Follows the key alone: every live reading brings new objects with the same content.
  $effect(() => {
    const asked = sheetKey;
    untrack(() => {
      const ids = (guide?.quests ?? []).flatMap((g) => (g.quest ? [g.quest.quest.id] : []));
      const place = [map?.name ?? null, map?.street ?? null, map?.twin ?? null] as const;
      Promise.all(ids.map((id) => backend.questSheet(id, ...place)))
        .then((found) => {
          if (asked !== sheetKey) return;
          sheets = Object.fromEntries(found.flatMap((where, i) => (where ? [[ids[i], where]] : [])));
        })
        .catch(() => {
          // The quest still shows, with no payout or steps.
        });
    });
  });

  /** Unfinished item objectives, which wait for the drop table to be downloaded. */
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
      await backend.downloadDrops();
      await refresh();
    } catch (e) {
      lookupError = String(e);
    } finally {
      lookingUp = false;
    }
  }
</script>

{#snippet who(person: string, maps: MapRef[])}
  <button class="ref ref-npc" onclick={() => showNpcNamed(person)}>{person}</button>{#if maps.length}<span class="faint">{" · "}</span>{#each maps as m, k (m.id)}{#if k}<span class="faint">, </span>{/if}{#if m.known}<button class="ref ref-map" onclick={() => backend.showPlace("maps", m.id)}>{m.name}</button>{:else}<span class="faint">{m.name}</span>{/if}{/each}{/if}
{/snippet}

<Card title="Quests" {plain}>
  {#snippet aside()}
    {#if quest.page}<span class="tabular">{quest.page[0]} tracked</span>{/if}
  {/snippet}

  {#each quest.quests as { title, objectives }, i}
    {@const info = guide?.quests[i]}
    {@const finished = objectives.length > 0 && objectives.every((o) => objectiveProgress(o) >= 1 && !o.unread)}
    {@const id = info?.quest?.quest.id}
    {@const sheet = id != null ? sheets[id] : null}
    {@const expanded = id != null && questView.expanded.includes(id)}
    <div class="quest" class:dense={plain}>
      <!-- The map data's spelling, once the title matched a quest there: OCR garbles some letters. -->
      <h3 class="title">
        {#if id != null}
          <button class="open" aria-expanded={expanded} onclick={() => toggleExpanded(id)}>
            <span class="chevron" class:expanded><ChevronRight size={13} /></span>
            <span>{info?.quest?.quest.name}</span>
          </button>
        {:else if title}
          {title}
        {:else}
          <!-- Leafline blanks a title too rough to name until it matches a quest. -->
          <span class="unread">Can't read this quest yet</span>
        {/if}
      </h3>
      {#if info?.quest}
        {@const turnIn = finished ? info.quest.turnIn : null}
        {@const person = turnIn ?? info.quest.giver}
        {@const maps = turnIn ? info.quest.turnInMaps : info.quest.giverMaps}
        {#if person}
          <p class="giver" class:finished>
            <UserRound size={11} />
            <span>
              {turnIn ? "Turn in to" : finished ? "Done · from" : "From"}
              {@render who(person, maps)}
            </span>
          </p>
        {/if}
        {#if !finished && info.quest.turnIn && info.quest.turnIn !== info.quest.giver}
          <p class="giver">
            <UserRound size={11} />
            <span>Turn in to {@render who(info.quest.turnIn, info.quest.turnInMaps)}</span>
          </p>
        {/if}
      {/if}
      {#if objectives.length}
        <ul>
          {#each objectives as objective, j}
            {@const progress = objectiveProgress(objective)}
            {@const done = progress >= 1 && !objective.unread}
            {@const hint = info?.objectives[j]}
            {@const eta = timeLeft(objective, hint)}
            <li class:done>
              <div class="row">
                {#if hint?.kind === "monster"}
                  <button class="ref ref-mob" onclick={() => showMonsterNamed(objective.target)}>{objective.target}</button>
                {:else if hint?.kind === "item"}
                  <button class="ref ref-item" onclick={() => showItemNamed(hint.item ?? objective.target)}>{objective.target}</button>
                {:else}
                  <span>{objective.target}</span>
                {/if}
                <!-- The game's Quest Helper shows the counts; here only that a line is done. -->
                {#if done}<span class="count" aria-label="Done"><Check size={13} /></span>{/if}
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
              {#if hint && !done && id == null}<ObjectiveHint guide={hint} from={guide?.from ?? null} />{/if}
            </li>
          {/each}
        </ul>
      {:else if !expanded}
        <p class="empty">Its lines could not be read.</p>
      {/if}
      {#if sheet && payout(sheet.sheet).length}
        <p class="payout">
          <Gift size={11} />
          <span>
            <!-- Kept on few lines: the separators carry their own spaces. -->
            {#each payout(sheet.sheet) as part, k}{#if k}<span class="sep">{" · "}</span>{/if}{#if part.kind === "item"}
                <span class="nowrap"><button class="ref ref-item" onclick={() => backend.showItem(part.id, null)}>{part.name}</button>{#if part.count > 1}<span class="tabular">{" "}×{part.count}</span>{/if}</span
              >{:else if part.kind === "more" && id != null}<button class="more" onclick={() => toggleExpanded(id)}
                  >{expanded ? part.text : `${part.text} ›`}</button
                >{:else}<span class="{part.kind} nowrap tabular">{part.text}</span>{/if}{/each}
          </span>
        </p>
      {/if}
      {#if expanded && sheet}
        <div class="details"><QuestSteps sheet={sheet.sheet} from={sheet.from} tracked={objectives} /></div>
      {/if}
    </div>
  {:else}
    <p class="empty">
      {quest.page?.[0] === 0 ? "No quests tracked." : "The Quest Helper was found, but its text could not be read."}
    </p>
  {/each}

  {#if quest.quests.length}
    {#if quest.quests.some((q) => !q.title)}
      <!-- Only while something can't be read, so it says why without nagging. -->
      <p class="footer faint">Scenery behind the Quest Helper can hide its text for a moment. UI Size: Ideal Ratio reads best.</p>
    {/if}
    {#if !ready}
      <p class="footer">Download map data from the Map tab to see where to hunt for each objective.</p>
    {:else if unknownItems.length}
      <div class="footer lookup">
        <button onclick={lookUp} disabled={lookingUp}>
          {#if lookingUp}<span class="spin"><LoaderCircle size={12} /></span>{:else}<Search size={12} />{/if}
          Find who drops {unknownItems.length === 1 ? "the quest item" : `${unknownItems.length} quest items`}
        </button>
        <span class="faint">Downloads mapleclassic.wiki's drop table</span>
      </div>
      {#if lookupError}<p class="footer error">{lookupError}</p>{/if}
    {/if}
    {#if guide}
      <p class="credit">
        Maps: <button onclick={() => backend.openLink(OSMS_URL)}>OSMS</button> (CC BY 4.0){" · "}{showsDrops ? "Drops and turn-ins" : "Turn-ins"} from <button onclick={() => backend.openLink(WIKI_URL)}>mapleclassic.wiki</button>
      </p>
    {/if}
  {/if}
</Card>

<style>
  .open {
    display: flex;
    align-items: center;
    gap: 3px;
    width: 100%;
    margin-left: -3px;
    padding: 0;
    border: 0;
    background: transparent;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .open:hover,
  .open:focus-visible {
    color: var(--accent-strong);
    outline: none;
  }

  .chevron {
    display: grid;
    flex: none;
    color: var(--faint);
    transition: transform 0.15s ease;
  }

  .chevron.expanded {
    transform: rotate(90deg);
  }

  .payout {
    display: flex;
    align-items: baseline;
    gap: 5px;
    margin: 7px 0 0;
    color: var(--muted);
    font-size: var(--text-sm);
  }

  .payout :global(svg) {
    flex: none;
    align-self: center;
    color: var(--faint);
  }

  .payout .sep {
    color: var(--faint);
    white-space: pre;
  }

  .payout .exp {
    color: var(--exp);
  }

  .payout .mesos {
    color: var(--mesos);
  }

  .nowrap {
    white-space: nowrap;
  }

  .more {
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--muted);
    font: inherit;
    cursor: pointer;
  }

  .more:hover,
  .more:focus-visible {
    color: var(--text);
    text-decoration: underline;
    outline: none;
  }

  .details {
    margin-top: 6px;
    padding-top: 2px;
    border-top: 1px solid var(--border);
  }

  .quest + .quest {
    margin-top: 12px;
    padding-top: 10px;
    border-top: 1px solid var(--border);
  }

  .title {
    margin: 0 0 8px;
    font-family: var(--font-display);
    font-size: var(--text-lg);
    font-weight: 600;
  }

  .unread {
    color: var(--muted);
    font-weight: 400;
  }

  .giver {
    display: flex;
    align-items: center;
    gap: 5px;
    margin: -5px 0 8px;
    color: var(--muted);
    font-size: var(--text-sm);
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
    display: flex;
    flex: none;
    align-items: center;
    color: var(--muted);
  }

  .done .count {
    color: var(--good);
  }

  .eta {
    display: flex;
    align-items: center;
    gap: 5px;
    margin: 4px 0 0;
    color: var(--muted);
    font-size: var(--text-xs);
  }

  .eta :global(svg) {
    flex: none;
    color: var(--faint);
  }

  .eta .faint {
    color: var(--faint);
  }

  /* Pinned: the same layout, packed tighter. */
  .dense .title {
    margin-bottom: 4px;
    font-size: var(--text-md);
  }

  .dense .giver {
    margin: -2px 0 5px;
  }

  .dense ul {
    gap: 5px;
  }

  .dense .row {
    margin-bottom: 2px;
    font-size: var(--text-sm);
  }

  .quest.dense + .quest.dense {
    margin-top: 8px;
    padding-top: 7px;
  }

  .empty {
    margin: 0;
    color: var(--faint);
    font-size: var(--text-sm);
  }

  .footer {
    margin: 12px 0 0;
    padding-top: 10px;
    border-top: 1px solid var(--border);
    color: var(--faint);
    font-size: var(--text-sm);
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
    font-size: var(--text-sm);
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
    font-size: var(--text-xs);
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
