<script lang="ts">
  // The Now tab: what matters while you play, with nothing to dig for. This session's two rates
  // big enough to read mid-fight, the quest you're on with where to hunt each objective, and the
  // map you're in with what spawns there. Who you are and how far through the level are in the
  // header, so nothing here repeats them. A row's arrow opens the fuller tab.
  import { ChevronRight } from "@lucide/svelte";

  import { backend } from "$lib/api";
  import Ref from "$lib/components/Ref.svelte";
  import { formatCompact, formatNumber, groundDistance, mapLabel, monsterLabel, objectiveProgress, partyLabel, questTitle } from "$lib/format";
  import { mapData, watchMapData } from "$lib/mapdata.svelte";
  import { huntOrder, orderGrounds } from "$lib/quests.svelte";
  import { mesosCoverage } from "$lib/training";
  import type { Analysis, Guide, MobRow, Objective, SessionSummary } from "$lib/types";
  import Sparkline from "$lib/ui/Sparkline.svelte";

  import type { Tab } from "./tabs";
  import Welcome from "./Welcome.svelte";
  import TypedSessionCard from "$lib/components/TypedSessionCard.svelte";
  import ReadWindowCard from "$lib/components/ReadWindowCard.svelte";
  import { can } from "$lib/platform";
  import { live } from "$lib/readings.svelte";
  import { untrack } from "svelte";

  let {
    analysis,
    session,
    ongo,
  }: { analysis: Analysis | null; session: SessionSummary | null; ongo: (tab: Tab) => void } = $props();

  const percentOnly = $derived(session?.percentOnly ?? false);
  const tracking = $derived(session != null && session.frames > 0);

  /** A session's length as a clock: `24:10`, `1:02:45`. */
  function clock(seconds: number): string {
    const total = Math.max(0, Math.floor(seconds));
    const h = Math.floor(total / 3600);
    const m = Math.floor((total % 3600) / 60);
    const s = String(total % 60).padStart(2, "0");
    return h ? `${h}:${String(m).padStart(2, "0")}:${s}` : `${m}:${s}`;
  }

  /** Full digits while they fit the tile, short form past a million. */
  const big = (value: number | null | undefined) => (value == null ? "—" : Math.abs(value) >= 1e6 ? formatCompact(value) : formatNumber(Math.round(value)));

  const expRate = $derived.by(() => {
    if (!session) return "—";
    if (percentOnly) return session.percentPerHour != null ? `${session.percentPerHour.toFixed(1)}%` : "—";
    return big(session.expPerHour);
  });

  /** The last five minutes against the whole session, in percent. */
  const pace = $derived.by(() => {
    const recent = percentOnly ? session?.recentPercentPerHour : session?.recentExpPerHour;
    const whole = percentOnly ? session?.percentPerHour : session?.expPerHour;
    if (recent == null || !whole) return null;
    return Math.round((recent / whole - 1) * 100);
  });

  const coverage = $derived(session ? mesosCoverage(session) : null);
  const mesosTrend = $derived((session?.timeline ?? []).map((p) => p.mesosPerHour));
  const party = $derived(partyLabel(session?.party));

  const quests = $derived((analysis?.quest?.quests ?? []).filter((q) => q.objectives.length));
  const quest = $derived(quests[0] ?? null);
  /** Its name, with any bracketed series in front shown as a tag. */
  const questName = $derived(quest ? questTitle(quest.title) : null);
  const map = $derived(analysis?.map ?? null);

  // With map data, each objective of the quest shown gets its best place, and the map you're in
  // its monsters. Both follow the text read, not each new reading, so nothing flickers.
  watchMapData();
  const ready = $derived(mapData.status?.ready ?? false);
  let guide = $state<Guide | null>(null);
  let mobs = $state<MobRow[]>([]);

  const guideKey = $derived(
    JSON.stringify([quest ? { title: quest.title, objectives: quest.objectives.map((o) => o.target) } : null, map?.name, map?.street, map?.twin]),
  );
  $effect(() => {
    const key = guideKey;
    void mapData.status?.dataUpdatedAt;
    if (!ready || !quest) {
      guide = null;
      return;
    }
    const asked = { title: quest.title, objectives: quest.objectives.map((o) => o.target) };
    untrack(() =>
      backend
        .questGuide([asked], map?.name ?? null, map?.street ?? null, map?.twin ?? null, [])
        .then((g) => {
          if (key === guideKey) guide = g;
        })
        .catch(() => (guide = null)),
    );
  });

  const mapKey = $derived(JSON.stringify([map?.name, map?.street, map?.twin]));
  $effect(() => {
    const key = mapKey;
    void mapData.status?.dataUpdatedAt;
    if (!ready || !map?.name) {
      mobs = [];
      return;
    }
    untrack(() =>
      backend
        .findMap(map.name, map.street ?? null, map.twin ?? null)
        .then((m) => {
          if (key === mapKey) mobs = m && m.score >= 0.85 ? [...m.card.mobs].sort((a, b) => b.count - a.count).slice(0, 4) : [];
        })
        .catch(() => (mobs = [])),
    );
  });

  /** Where to go for one objective: its best map, or that you're on it. */
  function hintFor(o: Objective) {
    const g = guide?.quests[0]?.objectives.find((x) => x.target === o.target);
    const hunt = g?.hunts.find((h) => h.monster.known && h.maps.length);
    if (!g || !hunt) return null;
    const best = orderGrounds(hunt.maps, huntOrder.value)[0];
    return {
      map: best.map,
      here: best.hops === 0,
      mob: g.kind === "item" ? monsterLabel(hunt.monster.name) : null,
      distance: groundDistance(best, guide?.from != null),
    };
  }
</script>

<div class="now">
  <!-- The website reads the game only from a window the player shares with it. -->
  {#if can.readWindow}<ReadWindowCard compact />{/if}
  {#if !analysis && !session}
    <!-- Without reading, you type the session in. So can the program, with live reading off. -->
    {#if can.liveReading}<Welcome />{/if}
    {#if can.liveReading ? live.liveStatus.state === "off" : live.liveStatus.state !== "reading" && live.liveStatus.state !== "waiting"}<TypedSessionCard />{/if}
  {:else}
    <section class="card session" aria-label="This session">
      <div class="head">
        <span class="title">This session{#if party}<span class="where">{" "}· {party}</span>{/if}</span>
        {#if session}
          <span class="clock num" class:paused={session.paused}>{session.paused ? "Paused · " : ""}{clock(session.activeSeconds)}</span>
        {/if}
      </div>

      {#if tracking && session}
        <div class="tiles">
          <div class="tile t1">
            <div class="label">{percentOnly ? "Level per hour" : "EXP per hour"}</div>
            <div class="rate num">{expRate}</div>
            <Sparkline values={session.trend} color="var(--tile-exp-line)" />
            <div class="note" class:good={pace != null && pace > 0}>
              {#if session.measuring}
                First estimate
              {:else if pace == null}
                Whole session
              {:else if pace === 0}
                Steady over the last 5 min
              {:else}
                {pace > 0 ? "Up" : "Down"} {Math.abs(pace)}% in the last 5 min
              {/if}
            </div>
          </div>
          <div class="tile t2">
            <div class="label">Mesos per hour</div>
            <div class="rate num mesos" class:thin={coverage?.thin}>{big(session.mesosPerHour)}</div>
            <Sparkline values={mesosTrend} color="var(--tile-mesos-line)" />
            <div class="note">
              {#if coverage?.partial}
                Counted {coverage.label}
              {:else}
                {session.mesosGained >= 0 ? "+" : ""}{formatCompact(session.mesosGained)} this session
              {/if}
            </div>
          </div>
        </div>
      {:else}
        <p class="quiet">Rates start once the EXP bar has moved. Hunt a little with the HUD in view.</p>
      {/if}
      <button class="more" onclick={() => ongo("train")}>Maps, goals, potions and loot<ChevronRight size={14} /></button>
    </section>

    {#if quest}
      <section class="card quest" aria-label="Tracked quest">
        <button class="quest-head" onclick={() => ongo("quests")}>
          <span class="quest-title">{questName?.name}{#if questName?.tag}<span class="badge qtag">{questName.tag}</span>{/if}</span>
          <span class="label">{quests.length > 1 ? `1 of ${quests.length} tracked` : "Tracked"}<ChevronRight size={13} /></span>
        </button>
        {#each quest.objectives as o (o.target)}
          {@const done = !o.unread && o.current >= o.required}
          {@const hint = done ? null : hintFor(o)}
          <div class="objective">
            <span class="target">{o.target}</span>
            {#if !o.unread}
              <span class="num count" class:done>{o.current} <span class="of">/ {o.required}</span></span>
            {/if}
          </div>
          <div class="bar"><span class:done style:width="{o.unread ? 0 : objectiveProgress(o) * 100}%"></span></div>
          {#if hint}
            <div class="hint">
              {#if hint.mob}<Ref kind="mob" name={hint.mob} />{" "}drops it{#if !hint.here}{" "}· <Ref kind="map" id={hint.map.id} name={hint.map.name} />{/if}
              {:else if hint.here}On this map
              {:else}Most at <Ref kind="map" id={hint.map.id} name={hint.map.name} />{/if}
              {#if hint.distance && !hint.here}<span class="far">{" "}· {hint.distance}</span>{/if}
            </div>
          {/if}
        {/each}
      </section>
    {/if}

    <section class="card here" aria-label="This map">
      <button class="row" onclick={() => ongo("map")}>
        {#if map}
          <span class="text"><span class="mapname">{mapLabel(map)}</span></span>
          <span class="go">Map<ChevronRight size={14} /></span>
        {:else}
          <span class="text"><span class="dim">Map:</span> the minimap isn't in view</span>
          <ChevronRight size={14} />
        {/if}
      </button>
      {#if mobs.length}
        <div class="mobs">
          {#each mobs as m (m.id)}
            <span class="mob"><Ref kind="mob" id={m.id} name={m.name} /><span class="lv num">Lv {m.level}</span></span>
          {/each}
        </div>
      {/if}
    </section>
  {/if}
</div>

<style>
  .now {
    display: grid;
    align-content: start;
    gap: var(--space-2);
    padding: 0 var(--space-3) var(--space-2);
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-4) 0;
    color: var(--muted);
    font-size: var(--text-md);
  }

  .title {
    min-width: 0;
    overflow: hidden;
    font-weight: var(--weight-strong);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .where {
    color: var(--faint);
    font-weight: var(--weight-body);
  }

  .clock {
    flex: none;
    font-weight: var(--weight-strong);
  }

  .clock.paused {
    color: var(--warn);
  }

  .tiles {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3) 0;
  }

  .tile {
    min-width: 0;
    padding: var(--space-2) var(--space-3);
    border: var(--line-w) solid var(--tile-border);
    border-radius: 12px;
    color: var(--tile-text);
  }

  .t1 {
    background: var(--tile-1);
  }

  .t2 {
    background: var(--tile-2);
  }

  .tile .label {
    color: var(--tile-label);
  }

  .rate {
    overflow: hidden;
    font-size: var(--text-hero);
    font-weight: var(--weight-strong);
    line-height: 34px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .rate.mesos {
    color: var(--tile-mesos);
  }

  .rate.thin {
    opacity: 0.55;
  }

  .note {
    margin-top: var(--space-0-5);
    overflow: hidden;
    color: var(--tile-label);
    font-size: var(--text-xs);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .note.good {
    color: var(--tile-good);
    font-weight: var(--weight-strong);
  }

  .quiet {
    margin: 0;
    padding: var(--space-2) var(--space-4) var(--space-1);
    color: var(--faint);
    font-size: var(--text-md);
  }

  .more,
  .row,
  .quest-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    width: 100%;
    border: 0;
    background: transparent;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .more {
    padding: var(--space-2) var(--space-3) var(--space-2) 14px;
    border-top: 1px solid var(--border);
    border-radius: 0 0 var(--radius-lg) var(--radius-lg);
    color: var(--faint);
    font-size: var(--text-md);
  }

  .more:hover,
  .row:hover {
    background: var(--hover);
    color: var(--text);
  }

  .more:focus-visible,
  .row:focus-visible,
  .quest-head:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }

  .quest {
    padding: var(--space-3) var(--space-4);
  }

  .quest-head {
    align-items: baseline;
    margin-bottom: var(--space-2);
    padding: 0;
  }

  .quest-title {
    min-width: 0;
    overflow: hidden;
    font-size: var(--text-lg);
    font-weight: var(--weight-strong);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .qtag {
    margin-left: var(--space-1-5);
    font-family: var(--font-sans);
    vertical-align: 2px;
  }

  .quest-head .label {
    display: inline-flex;
    flex: none;
    align-items: center;
    gap: var(--space-0-5);
  }

  .quest-head:hover .label {
    color: var(--text);
  }

  .objective {
    display: flex;
    justify-content: space-between;
    gap: var(--space-2);
    margin-bottom: var(--space-1);
  }

  .target {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .count {
    flex: none;
    font-weight: var(--weight-strong);
  }

  .count.done {
    color: var(--good);
  }

  .of {
    color: var(--faint);
    font-weight: var(--weight-body);
  }

  .count.done .of {
    color: inherit;
  }

  .bar {
    height: 6px;
    margin-bottom: var(--space-2);
    border-radius: var(--radius-pill);
    background: var(--track);
    overflow: hidden;
  }

  .bar:last-child {
    margin-bottom: 0;
  }

  .bar span {
    display: block;
    height: 100%;
    border-radius: var(--radius-pill);
    background: var(--ref-item);
    transition: width 0.3s var(--ease);
  }

  .bar span.done {
    background: var(--good);
  }

  .hint {
    margin: calc(-1 * var(--space-1)) 0 var(--space-2);
    overflow: hidden;
    color: var(--faint);
    font-size: var(--text-xs);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .hint:last-child {
    margin-bottom: 0;
  }

  .far {
    color: var(--faint);
  }

  .here {
    padding: 0;
    overflow: hidden;
  }

  .go {
    display: inline-flex;
    flex: none;
    align-items: center;
    gap: var(--space-0-5);
    color: var(--faint);
    font-size: var(--text-xs);
  }

  .row:hover .go {
    color: var(--text);
  }

  .mobs {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1-5) var(--space-3);
    padding: 0 var(--space-4) var(--space-3);
    font-size: var(--text-md);
  }

  .mob {
    display: inline-flex;
    align-items: baseline;
    gap: var(--space-1);
    min-width: 0;
  }

  .lv {
    color: var(--faint);
    font-size: var(--text-xs);
  }

  .row {
    padding: var(--space-2) var(--space-3) var(--space-2) 14px;
    color: var(--muted);
  }

  .text {
    min-width: 0;
    overflow: hidden;
    color: var(--text);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .dim {
    color: var(--faint);
    font-size: var(--text-xs);
  }

  .mapname {
    color: var(--ref-map);
    font-weight: var(--weight-strong);
  }
</style>
