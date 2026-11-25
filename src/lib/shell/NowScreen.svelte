<script lang="ts">
  // The Now tab: what matters while you play, with nothing to dig for. This session's two rates
  // big enough to read mid-fight, the quest you're on, and one-line ways into the map you're in
  // and your character. Everything here is also somewhere fuller; a row's arrow takes you there.
  import { ChevronRight } from "@lucide/svelte";

  import { formatCompact, formatNumber, mapLabel, objectiveProgress, partyLabel } from "$lib/format";
  import { mesosCoverage } from "$lib/training";
  import type { Analysis, SessionSummary } from "$lib/types";
  import Sparkline from "$lib/ui/Sparkline.svelte";

  import type { Tab } from "./tabs";
  import Welcome from "./Welcome.svelte";
  import TypedSessionCard from "$lib/components/TypedSessionCard.svelte";
  import ReadWindowCard from "$lib/components/ReadWindowCard.svelte";
  import { can } from "$lib/platform";
  import { live } from "$lib/readings.svelte";

  let {
    analysis,
    session,
    ongo,
  }: { analysis: Analysis | null; session: SessionSummary | null; ongo: (tab: Tab) => void } = $props();

  const hud = $derived(analysis?.hud ?? null);
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
  const mesosHeld = $derived(analysis?.mesos?.mesos ?? session?.mesos ?? null);
  const where = $derived([partyLabel(session?.party), session?.map].filter(Boolean).join(" · "));

  const quests = $derived((analysis?.quest?.quests ?? []).filter((q) => q.objectives.length));
  const quest = $derived(quests[0] ?? null);
  const map = $derived(analysis?.map ?? null);
  const level = $derived(hud?.level ?? session?.level ?? null);
  const job = $derived(hud?.job ?? session?.job ?? null);
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
        <span class="title">This session{#if where}<span class="where">{" "}· {where}</span>{/if}</span>
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
        <div class="cells">
          <div class="cell">
            <div class="label">Kills / hr</div>
            <div class="num v">{session.killsPerHour != null ? formatCompact(session.killsPerHour) : "—"}</div>
          </div>
          <div class="cell">
            <div class="label">{percentOnly ? "Levels gained" : "EXP gained"}</div>
            <div class="num v">{percentOnly ? `${session.levelUps}` : formatCompact(session.expGained)}</div>
          </div>
          <div class="cell">
            <div class="label">Mesos held</div>
            <div class="num v"><span class="coin" aria-hidden="true"></span>{mesosHeld != null ? formatNumber(mesosHeld) : "—"}</div>
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
          <span class="quest-title">{quest.title}</span>
          <span class="label">{quests.length > 1 ? `1 of ${quests.length} tracked` : "Tracked"}<ChevronRight size={13} /></span>
        </button>
        {#each quest.objectives as o (o.target)}
          {@const done = !o.unread && o.current >= o.required}
          <div class="objective">
            <span class="target">{o.target}</span>
            {#if !o.unread}
              <span class="num count" class:done>{o.current} <span class="of">/ {o.required}</span></span>
            {/if}
          </div>
          <div class="bar"><span class:done style:width="{o.unread ? 0 : objectiveProgress(o) * 100}%"></span></div>
        {/each}
      </section>
    {/if}

    <section class="card rows" aria-label="More">
      <button class="row" onclick={() => ongo("map")}>
        {#if map}
          <span class="text"><span class="mapname">{mapLabel(map)}</span> <span class="dim">· exits, monsters and routes</span></span>
        {:else}
          <span class="text"><span class="dim">Map:</span> the minimap isn't in view</span>
        {/if}
        <ChevronRight size={14} />
      </button>
      <button class="row" onclick={() => ongo("me")}>
        <span class="text">
          {#if level != null}<span class="dim">Lv {level}{job ? ` ${job}` : ""}:</span>{/if}
          stats, build and gear
        </span>
        <ChevronRight size={14} />
      </button>
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

  .cells {
    display: flex;
    margin-top: var(--space-2);
    border-top: 1px solid var(--border);
  }

  .cell {
    flex: 1;
    min-width: 0;
    padding: var(--space-2) var(--space-3);
    border-right: 1px solid var(--border);
  }

  .cell:last-child {
    border-right: 0;
  }

  .v {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    font-size: var(--text-lg);
    font-weight: var(--weight-strong);
    white-space: nowrap;
  }

  .coin {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: var(--accent);
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

  .rows {
    padding: 0;
    overflow: hidden;
  }

  .row {
    padding: var(--space-2) var(--space-3) var(--space-2) 14px;
    color: var(--muted);
  }

  .row + .row {
    border-top: 1px solid var(--border);
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
