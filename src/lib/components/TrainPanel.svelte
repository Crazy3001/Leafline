<script lang="ts">
  // The Train tab: the session being played (rates, the level goal, a graph of the whole session,
  // how this map compares with your bests on it, the maps played), then where to train next, with
  // a jump to it from the top. Before a session starts: the goal, where to train, your best maps
  // and the last sessions.
  import { tip } from "$lib/tip.svelte";
  import Ref from "$lib/components/Ref.svelte";
  import SessionControls from "./SessionControls.svelte";
  import { ArrowDown } from "@lucide/svelte";

  import CharacterPicker from "./CharacterPicker.svelte";
  import LevelGoal from "./LevelGoal.svelte";
  import PotionCosts from "./PotionCosts.svelte";
  import SaveSpot from "./SaveSpot.svelte";
  import SpotList, { SPOTS_ID } from "./SpotList.svelte";
  import TimersCard from "./TimersCard.svelte";
  import { openTrainingLog } from "$lib/nav.svelte";
  import { formatClockIn, formatCompact, formatDuration, formatNumber, splitMapLabel } from "$lib/format";
  import { characters } from "$lib/characters.svelte";
  import { history, watchHistory } from "$lib/history.svelte";
  import { live } from "$lib/readings.svelte";
  import {
    chartPoints,
    eventMarkers,
    mapBands,
    mapStats,
    mesosCoverage,
    mesosReminder,
    pointDetail,
    rateFormat,
    shortDay,
    showUnplaced,
    type Metric,
  } from "$lib/training";
  import type { MapReading, SessionSummary } from "$lib/types";
  import Block from "$lib/ui/Block.svelte";
  import Button from "$lib/ui/Button.svelte";
  import LineChart from "$lib/ui/LineChart.svelte";
  import Meter from "$lib/ui/Meter.svelte";
  import Segmented from "$lib/ui/Segmented.svelte";
  import Stat from "$lib/ui/Stat.svelte";

  let {
    level,
    map = null,
    session = null,
    glance = false,
  }: {
    level: number | null;
    map?: MapReading | null;
    session?: SessionSummary | null;
    /** A pinned window's short form: the session's rates and controls, and timers. The graph,
     *  the maps, where to train and past sessions stay in the tab. */
    glance?: boolean;
  } = $props();

  watchHistory();

  const METRIC_KEY = "leafline.train.metric";
  let metric = $state<Metric>(storedMetric());

  const tracking = $derived(session != null && session.activeSeconds > 0);
  const percentOnly = $derived(session?.percentOnly ?? false);
  const signed = (value: number) => (value > 0 ? `+${formatNumber(value)}` : formatNumber(value));

  /** How the last five minutes compare with the whole session, in percent. */
  const pace = $derived.by(() => {
    const recent = percentOnly ? session?.recentPercentPerHour : session?.recentExpPerHour;
    const whole = percentOnly ? session?.percentPerHour : session?.expPerHour;
    if (recent == null || !whole) return null;
    return Math.round((recent / whole - 1) * 100);
  });

  const headline = $derived.by(() => {
    if (!session) return null;
    if (percentOnly)
      return { label: "Level % per hour, this session", value: session.percentPerHour != null ? `${session.percentPerHour.toFixed(1)}%` : "—" };
    return { label: "EXP per hour, this session", value: formatCompact(session.expPerHour) };
  });
  /** The clock time the level comes at this pace. Worked out again with each reading. */
  const levelAt = $derived(session?.secondsToLevel != null && !session.paused ? formatClockIn(session.secondsToLevel) : "");
  /** The time to level, with its range and breaks, as one line and a fuller tooltip. */
  const toLevel = $derived.by(() => {
    const s = session;
    if (s?.secondsToLevel == null) return null;
    const range =
      s.secondsToLevelLow != null && s.secondsToLevelHigh != null
        ? `${formatDuration(s.secondsToLevelLow)}–${formatDuration(s.secondsToLevelHigh)}`
        : null;
    const breaks = s.secondsToLevelWithBreaks != null ? `about ${formatDuration(s.secondsToLevelWithBreaks)} with breaks` : null;
    const why = [
      "At this map's pace, less EXP lost to deaths.",
      s.paceFromEstimate ? "Early on it leans on an estimate for this map, from your past play and your stats; your own pace takes over as you play." : null,
      range ? `Four times in five it should take ${range} of play.` : null,
      breaks ? `Counting breaks the way this session has had them, ${breaks}.` : null,
    ];
    const line = [`${formatDuration(s.secondsToLevel)} to level here`, range ? `(${range})` : null].filter(Boolean).join(" ");
    return { line: levelAt ? `${line}, about ${levelAt}` : line, breaks, tip: why.filter(Boolean).join(" ") };
  });
  /** Share of a level an hour, said in words next to the time to level. */
  const levelPace = $derived(
    !percentOnly && session?.percentPerHour != null ? `${session.percentPerHour.toFixed(1)}% of a level an hour` : null,
  );

  /** The graph's points are five-minute rates, so it waits for five minutes of play. */
  const GRAPH_AFTER = 300;
  const recent = $derived(
    percentOnly
      ? session?.recentPercentPerHour != null
        ? `${session.recentPercentPerHour.toFixed(1)}%`
        : null
      : session?.recentExpPerHour != null
        ? formatCompact(session.recentExpPerHour)
        : null,
  );

  const lv = $derived(session?.level ?? level);
  const percent = $derived(session?.percent ?? null);

  const points = $derived(session ? chartPoints(session.timeline ?? [], metric, percentOnly) : []);
  const chartColor = $derived(metric === "exp" ? "var(--exp)" : "var(--accent)");

  // Rows worth showing: some time or some gain, towns marked.
  const sessionMaps = $derived(
    (session?.maps ?? []).filter(
      (m) => (m.activeSeconds > 0 || m.expGained !== 0 || m.mesosGained !== 0) && showUnplaced(m, session?.activeSeconds ?? 0),
    ),
  );
  const towns = $derived(new Set((session?.lines ?? []).filter((l) => l.town).map((l) => l.map)));

  // This map against your best on it, over the saved sessions.
  const stats = $derived(mapStats(characters.sessions(history.value?.sessions ?? [])));
  const here = $derived.by(() => {
    if (!session?.map || towns.has(session.map)) return null;
    const now = session.maps.find((m) => m.map === session.map);
    const saved = stats.find((s) => s.map === session.map);
    if (!now?.expPerHour && !saved) return null;
    return { name: splitMapLabel(session.map).name, now: now?.expPerHour ?? null, saved };
  });

  const coverage = $derived(session ? mesosCoverage(session) : null);
  const reminder = $derived(session ? mesosReminder(session) : null);

  // How Mesos / hr works, said once until the player dismisses it.
  const MESOS_HINT_KEY = "leafline.train.mesosHintSeen";
  let mesosHintSeen = $state(storedFlag(MESOS_HINT_KEY));

  function storedFlag(key: string): boolean {
    try {
      return localStorage.getItem(key) === "1";
    } catch {
      return false;
    }
  }

  function dismissMesosHint() {
    mesosHintSeen = true;
    try {
      localStorage.setItem(MESOS_HINT_KEY, "1");
    } catch {
      // A convenience only.
    }
  }

  const dropped = $derived(session ? session.rejected.exp + session.rejected.mesos : 0);

  /** Your best maps by EXP an hour, for before a session. */
  const BESTS_SHOWN = 5;
  const bests = $derived(
    stats
      .filter((s) => s.best)
      .sort((a, b) => b.best!.expPerHour - a.best!.expPerHour)
      .slice(0, BESTS_SHOWN),
  );

  // Before a session: the last few saved ones.
  const lastSessions = $derived(characters.sessions(history.value?.sessions ?? []).slice(0, 3));
  const lastOne = $derived(lastSessions[0] ?? null);

  const liveState = $derived(live.liveStatus.state);

  function storedMetric(): Metric {
    try {
      return localStorage.getItem(METRIC_KEY) === "mesos" ? "mesos" : "exp";
    } catch {
      return "exp";
    }
  }

  function pickMetric(next: Metric) {
    metric = next;
    try {
      localStorage.setItem(METRIC_KEY, next);
    } catch {
      // A convenience only.
    }
  }

  function jumpToSpots() {
    const reduce = window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;
    document.getElementById(SPOTS_ID)?.scrollIntoView({ behavior: reduce ? "auto" : "smooth", block: "start" });
  }

  function mainMap(maps: { map: string }[]) {
    return maps.length ? splitMapLabel(maps[0].map).name : "No map read";
  }
</script>

{#snippet jump()}
  <Button variant="ghost" size="sm" onclick={jumpToSpots}>
    Where to train <ArrowDown size={12} />
  </Button>
{/snippet}

{#if !glance}<CharacterPicker />{/if}
<Block title="Session">
  {#snippet aside()}
    {#if session?.held}
      <span class="state paused">Paused</span>
    {:else if tracking && session?.paused}
      <span class="state idle">Idle</span>
    {:else if tracking}
      <span class="state running"><span class="square" aria-hidden="true"></span>Tracking</span>
    {/if}
    {#if tracking && session}
      <span class="tabular">{formatDuration(session.activeSeconds)} active of {formatDuration(session.elapsedSeconds)}</span>
    {/if}
  {/snippet}

  <SessionControls trailing={tracking && !glance ? jump : undefined} />

  {#if !tracking && glance}
    <p class="caption">Starts with your first EXP.</p>
  {:else if !tracking}
    <div class="empty">
      <strong>Starts with your first EXP</strong>
      <span>
        {#if liveState === "reading"}
          Hunt as usual. Leafline starts the clock on the first gain and pauses it when you stop.
        {:else if liveState === "waiting"}
          Open the game. Leafline starts the clock on your first EXP gain.
        {:else}
          Turn on live reading and Leafline times your play from the first EXP gain.
        {/if}
      </span>
    </div>
  {/if}
</Block>

{#if tracking && session && headline}
  <Block>
    <div class="headline">
      <div class="big">
        <span class="label"
          >{headline.label}{#if session.measuring}<span
              class="measuring"
              use:tip={"A first estimate from your first kills. It settles after about a minute of play."}
            >
              · still measuring</span
            >{/if}</span
        >
        <span class="num hero" class:early={session.measuring}>{headline.value}</span>
      </div>
      {#if recent}
        <div class="recent">
          <span class="label">Last 5 min</span>
          <span class="figure">
            <span class="num">{recent}</span>
            {#if pace != null && pace !== 0}
              <span class="pace" class:up={pace > 0} class:down={pace < 0}>{pace > 0 ? "▲" : "▼"} {Math.abs(pace)}% vs session</span>
            {/if}
          </span>
        </div>
      {/if}
    </div>

    {#if lv != null && percent != null}
      <div class="level">
        <div class="level-line"><span class="muted">Lv {lv} to {lv + 1}</span><span class="tabular">{percent.toFixed(2)}%</span></div>
        <Meter value={percent / 100} label="EXP towards the next level" />
        {#if toLevel || levelPace}
          <span class="muted tabular" use:tip={toLevel?.tip ?? ""}>
            {#if toLevel}{toLevel.line}{/if}{#if toLevel && levelPace}{" · "}{/if}{#if levelPace}{levelPace}{/if}
          </span>
          {#if toLevel?.breaks}<span class="muted tabular">{toLevel.breaks}</span>{/if}
        {/if}
      </div>
    {/if}

    {#if !glance}
      <LevelGoal
        level={lv}
        {percent}
        character={session.name ?? characters.active}
        rate={{ expPerHour: session.expPerHour, percentPerHour: session.percentPerHour }}
      />
    {/if}

    <div class="tiles">
      <Stat
        label="Mesos / hr"
        value={formatCompact(session.mesosPerHour)}
        sub={coverage?.partial ? coverage.label : null}
        faint={coverage?.partial && coverage.thin}
      />
      <Stat
        label="EXP gained"
        value={signed(session.expGained)}
        sub={session.expOther > 0 ? `+${formatCompact(session.expOther)} from quests` : null}
      />
      {#if session.killsPerHour != null}
        <Stat label="Kills / hr" value={formatCompact(session.killsPerHour)} sub="on this map" />
      {/if}
      <Stat label="Mesos gained" value={signed(session.mesosGained)} />
    </div>

    {#if !glance}
      <PotionCosts
        startedAt={session.startedAt}
        activeSeconds={session.activeSeconds}
        mesosPerHour={session.mesosPerHour}
        character={session.name ?? characters.active}
      />
    {/if}

    {#if reminder}
      <p class="reminder">{reminder}</p>
    {/if}
    {#if glance && session.mapHidden}
      <p class="attention">Can't see the minimap's name, so new gains may go to the wrong map.</p>
    {/if}
    {#if !mesosHintSeen && !glance}
      <div class="hint">
        <span>
          Mesos / hr needs a look at your mesos when you start and again before you spend. Your inventory or a shop window
          both count, and you don't need to keep either open.
        </span>
        <Button size="sm" variant="ghost" onclick={dismissMesosHint}>Got it</Button>
      </div>
    {/if}
  </Block>

  {#if !glance}
  {#if here}
    <Block title="This map">
      {#snippet aside()}<span class="map-name">{here.name}</span>{/snippet}
      {@const best = here.saved?.best?.expPerHour ?? null}
      {@const top = Math.max(here.now ?? 0, best ?? 0, 1)}
      {@const bestMesos = here.saved?.bestMesos?.mesosPerHour ?? null}
      {@const mesosNow = session.maps.find((m) => m.map === session.map)?.mesosPerHour ?? null}
      {@const mesosTop = Math.max(mesosNow ?? 0, bestMesos ?? 0, 1)}
      <div class="compare">
        <span class="muted">EXP now</span>
        <Meter value={(here.now ?? 0) / top} height={8} />
        <span class="tabular">{formatCompact(here.now)}</span>
        {#if best != null}
          <span class="muted">Best here</span>
          <Meter value={best / top} height={8} color="var(--muted)" />
          <span class="tabular">{formatCompact(best)}</span>
        {/if}
        {#if bestMesos != null}
          <span class="muted">Mesos now</span>
          <Meter value={(mesosNow ?? 0) / mesosTop} height={8} color="var(--accent)" />
          <span class="tabular">{formatCompact(mesosNow)}</span>
          <span class="muted">Best here</span>
          <Meter value={bestMesos / mesosTop} height={8} color="var(--muted)" />
          <span class="tabular">{formatCompact(bestMesos)}</span>
        {/if}
      </div>
      {#if here.saved?.best}
        <p class="caption">
          Best: {here.saved.best.level != null ? `Lv ${here.saved.best.level}, ` : ""}{(here.saved.best.party ?? 1) > 1
            ? `party of ${here.saved.best.party}`
            : "solo"}, {shortDay(here.saved.best.at)}. {formatDuration(here.saved.activeSeconds)} here over {here.saved.sessions}
          session{here.saved.sessions === 1 ? "" : "s"}.
        </p>
      {:else}
        <p class="caption">Your first time logged here. Your best shows once a run here passes 10 minutes.</p>
      {/if}
    </Block>
  {/if}

  {#if sessionMaps.length}
    <Block title="Maps this session">
      <table>
        <thead><tr><th>Map</th><th>Time</th><th>EXP / hr</th><th>Mesos / hr</th></tr></thead>
        <tbody>
          {#each sessionMaps as row (row.map)}
            {@const town = row.map != null && towns.has(row.map)}
            <tr class:town>
              <td>
                {#if row.map === session.map}<span class="here" use:tip={"You are here"}></span>{/if}
                {#if row.map}<Ref kind="map" name={row.map}>{splitMapLabel(row.map).name}</Ref
                  >{#if !town} <SaveSpot label={row.map} name={splitMapLabel(row.map).name} />{/if}{:else}<span
                    class="unplaced"
                    >Not placed on a map</span
                  >{/if}
              </td>
              <td class="tabular">{formatDuration(row.activeSeconds)}</td>
              <td class="tabular">{town ? "town" : formatCompact(row.expPerHour)}</td>
              <td class="tabular">{town ? "town" : formatCompact(row.mesosPerHour)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
      {#if sessionMaps.length > 1}
        <p class="caption">Each map's rate counts only your time on it. The session rate at the top counts all of it, towns too.</p>
      {/if}
      {#if session.deaths > 0 || session.mapHidden || dropped > 0 || session.mesosExcluded !== 0}
        <ul class="notes">
          {#if session.mapHidden}<li class="attention">Can't see the minimap's name, so new gains may go to the wrong map.</li>{/if}
          {#if session.deaths > 0}
            <li>
              {session.deaths} death{session.deaths === 1 ? "" : "s"}, {session.percentOnly
                ? `${session.percentLost.toFixed(2)}% of a level`
                : `${formatNumber(session.expLost)} EXP`} lost and left out of the rates.
            </li>
          {/if}
          {#if session.mesosExcluded !== 0}
            <li>{signed(session.mesosExcluded)} mesos moved through shops, storage or trades and are left out.</li>
          {/if}
          {#if dropped > 0}<li>{dropped} misread{dropped === 1 ? "" : "s"} dropped.</li>{/if}
        </ul>
      {/if}
    </Block>
  {/if}
  <!-- Where to go next sits under the live numbers; the rate graph is detail, last. -->
  <SpotList {level} {map} {session} />
  <Block title="This session">
    {#snippet aside()}
      <Segmented
        size="sm"
        label="Graph shows"
        value={metric}
        options={[
          { id: "exp", label: percentOnly ? "% / hr" : "EXP / hr" },
          { id: "mesos", label: "Mesos / hr" },
        ]}
        onchange={pickMetric}
      />
    {/snippet}
    {#if session.activeSeconds >= GRAPH_AFTER && points.filter((p) => p.y != null).length > 1}
      <LineChart
        {points}
        xMax={Math.max(session.activeSeconds, points[points.length - 1]?.x ?? 0)}
        color={chartColor}
        format={rateFormat(metric, percentOnly)}
        bands={mapBands(session.timeline)}
        markers={eventMarkers(session.events ?? [])}
        detail={(i) => pointDetail(session.timeline[i])}
        label="{metric === 'exp' ? 'EXP' : 'Mesos'} per hour over this session"
      />
      <p class="caption">
        Each point is the rate over the 5 minutes before it; the first 2½ minutes are left out.
        {#if metric === "mesos"}Mesos are spread evenly between your looks at them, so the line reaches only your last look.{/if}
        {#if session.events?.length}Green dots are level-ups, red ones deaths.{/if}
      </p>
    {:else}
      <p class="caption">The graph starts after 5 minutes of play.</p>
    {/if}
  </Block>
  {/if}
{:else if !glance}
  <!-- No session yet: the goal and where to train come first, your bests and the last sessions after. -->
  <Block title="Level goal">
    <LevelGoal {level} percent={session?.percent ?? null} character={session?.name ?? characters.active} />
  </Block>
  <SpotList {level} {map} {session} last={!lastOne && !bests.length} />
  {#if bests.length}
    <Block title="Your best maps" last={!lastOne}>
      {#snippet aside()}<span>Best single run, 10 min or more</span>{/snippet}
      <table>
        <thead><tr><th>Map</th><th>EXP / hr</th><th>Mesos / hr</th><th>When</th></tr></thead>
        <tbody>
          {#each bests as m (m.map)}
            <tr>
              <td><Ref kind="map" name={m.map}>{m.name}</Ref> <SaveSpot label={m.map} name={m.name} /></td>
              <td class="tabular">{formatCompact(m.best!.expPerHour)}</td>
              <td class="tabular">{m.bestMesos ? formatCompact(m.bestMesos.mesosPerHour) : "—"}</td>
              <td class="tabular muted">{m.best!.level != null ? `Lv ${m.best!.level} · ` : ""}{shortDay(m.best!.at)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </Block>
  {/if}
{/if}

{#if !(tracking && session && headline) && lastOne && !glance}
  <Block title="Last time">
    {#snippet aside()}
      {#if lastOne.character}<span>{lastOne.character}{lastOne.job ? ` · ${lastOne.job}` : ""}</span>{/if}
    {/snippet}
    <div class="tiles">
      <Stat label={lastOne.expPerHour != null ? "EXP / hr" : "% / hr"} value={lastOne.expPerHour != null ? formatCompact(lastOne.expPerHour) : `${lastOne.percentPerHour?.toFixed(1) ?? "—"}%`} />
      <Stat label="Mesos / hr" value={formatCompact(lastOne.mesosPerHour)} />
      <Stat label="Played" value={formatDuration(lastOne.activeSeconds)} />
    </div>
    <ul class="recent-list">
      {#each lastSessions as s (s.id)}
        <li>
          <span class="tabular muted day">{shortDay(s.startedAt ?? s.savedAt)}</span>
          <span class="what"
            >{#if !s.label && s.maps.length}<Ref kind="map" name={s.maps[0].map}>{mainMap(s.maps)}</Ref
              >{:else}{s.label || mainMap(s.maps)}{/if}</span
          >
          <span class="tabular">{formatCompact(s.expPerHour)}</span>
        </li>
      {/each}
    </ul>
    <div><Button onclick={openTrainingLog}>All sessions</Button></div>
  </Block>
{/if}

<TimersCard />

<style>
  .state {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: var(--text-xs);
  }

  .running {
    color: var(--good);
  }

  .paused {
    color: var(--warn);
  }

  .idle {
    color: var(--muted);
    cursor: help;
  }

  .square {
    width: 6px;
    height: 6px;
    background: currentColor;
  }

  .empty {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: var(--space-3);
    background: var(--surface-2);
    font-size: var(--text-sm);
    color: var(--muted);
  }

  .empty strong {
    font-size: var(--text-md);
    color: var(--text);
  }

  .headline {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: var(--space-2);
  }

  .big,
  .recent {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .recent {
    align-items: flex-end;
  }

  .measuring {
    color: var(--warn);
  }

  .hero.early {
    opacity: 0.7;
  }

  .label {
    font-size: var(--text-xs);
    color: var(--muted);
  }

  .figure {
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
  }

  .hero {
    font-size: var(--text-hero);
    font-weight: 600;
    line-height: 1;
  }

  .recent .num {
    font-size: var(--text-lg);
    font-weight: 600;
  }

  .pace {
    font-size: var(--text-xs);
    color: var(--muted);
  }

  .pace.up {
    color: var(--good);
  }

  .pace.down {
    color: var(--warn);
  }

  .level {
    display: flex;
    flex-direction: column;
    gap: 5px;
    font-size: var(--text-sm);
  }

  .level-line {
    display: flex;
    justify-content: space-between;
  }

  .muted {
    color: var(--muted);
  }

  .tiles {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 6px;
  }

  .reminder {
    margin: 0;
    padding: 6px var(--space-2);
    border-left: 2px solid var(--warn);
    background: var(--surface-2);
    font-size: var(--text-xs);
    color: var(--text);
  }

  .hint {
    display: flex;
    align-items: flex-start;
    gap: var(--space-2);
    font-size: var(--text-xs);
    color: var(--muted);
  }

  .hint span {
    flex: 1;
    min-width: 0;
  }

  .caption {
    margin: 0;
    font-size: var(--text-xs);
    color: var(--faint);
  }

  .map-name {
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--text);
  }

  .compare {
    display: grid;
    grid-template-columns: 96px minmax(0, 1fr) 44px;
    gap: 6px var(--space-2);
    align-items: center;
    font-size: var(--text-sm);
  }

  .compare > :nth-child(3n) {
    text-align: right;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--text-sm);
  }

  th {
    padding: 0 0 4px;
    font-size: var(--text-xs);
    font-weight: 400;
    color: var(--faint);
    text-align: right;
  }

  td {
    padding: 5px 0;
    border-top: 1px solid var(--border);
    text-align: right;
    white-space: nowrap;
  }

  th:first-child,
  td:first-child {
    max-width: 130px;
    overflow: hidden;
    text-align: left;
    text-overflow: ellipsis;
  }

  .unplaced {
    color: var(--muted);
    cursor: help;
  }

  .town td {
    color: var(--faint);
  }

  .here {
    display: inline-block;
    width: 6px;
    height: 6px;
    margin-right: 6px;
    vertical-align: 1px;
    background: var(--accent);
  }

  .notes {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin: 0;
    padding: 0;
    list-style: none;
    font-size: var(--text-xs);
    color: var(--muted);
  }

  .attention {
    color: var(--warn);
  }

  .recent-list {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .recent-list li {
    display: flex;
    gap: var(--space-2);
    padding: 7px 0;
    border-top: 1px solid var(--border);
    font-size: var(--text-sm);
  }

  .day {
    width: 56px;
    flex: none;
  }

  .what {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
</style>
