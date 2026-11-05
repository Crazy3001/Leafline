<script lang="ts">
  // The Train tab: the session being played (rates, a graph of the whole session, how this map
  // compares with your best on it, the maps played), then where to train next. Before a session
  // starts it shows the last one instead.
  import SessionControls from "./SessionControls.svelte";
  import SpotList from "./SpotList.svelte";
  import TimersCard from "./TimersCard.svelte";
  import { backend } from "$lib/api";
  import { formatCompact, formatDuration, formatNumber, splitMapLabel } from "$lib/format";
  import { history, watchHistory } from "$lib/history.svelte";
  import { live } from "$lib/readings.svelte";
  import {
    chartPoints,
    eventMarkers,
    mapBands,
    mapStats,
    pointDetail,
    rateFormat,
    shortDay,
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
  }: { level: number | null; map?: MapReading | null; session?: SessionSummary | null } = $props();

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
      return { label: "Level per hour", value: session.percentPerHour != null ? `${session.percentPerHour.toFixed(1)}%` : "—", sub: null };
    return {
      label: "EXP per hour",
      value: formatCompact(session.expPerHour),
      sub: session.percentPerHour != null ? `${session.percentPerHour.toFixed(1)}% / hr` : null,
    };
  });
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
    (session?.maps ?? []).filter((m) => m.activeSeconds > 0 || m.expGained !== 0 || m.mesosGained !== 0),
  );
  const towns = $derived(new Set((session?.lines ?? []).filter((l) => l.town).map((l) => l.map)));

  // This map against your best on it, over the saved sessions.
  const stats = $derived(mapStats(history.value?.sessions ?? []));
  const here = $derived.by(() => {
    if (!session?.map || towns.has(session.map)) return null;
    const now = session.maps.find((m) => m.map === session.map);
    const saved = stats.find((s) => s.map === session.map);
    if (!now?.expPerHour && !saved) return null;
    return { name: splitMapLabel(session.map).name, now: now?.expPerHour ?? null, saved };
  });

  const MESOS_STALE = 300;
  const mesosStale = $derived(session != null && !session.paused && (session.mesosAge ?? 0) >= MESOS_STALE);
  const dropped = $derived(session ? session.rejected.exp + session.rejected.mesos : 0);

  // Before a session: the last few saved ones.
  const lastSessions = $derived((history.value?.sessions ?? []).slice(0, 3));
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

  function mainMap(maps: { map: string }[]) {
    return maps.length ? splitMapLabel(maps[0].map).name : "No map read";
  }
</script>

<Block title="Session">
  {#snippet aside()}
    {#if session?.held}
      <span class="state paused" title="Paused by hand">Paused</span>
    {:else if tracking && session?.paused}
      <span class="state paused" title="No EXP for a while, or the map changed: the clock waits for the next gain">Paused</span>
    {:else if tracking}
      <span class="state running"><span class="square" aria-hidden="true"></span>Tracking</span>
    {/if}
    {#if tracking && session}
      <span class="tabular">{formatDuration(session.activeSeconds)} active of {formatDuration(session.elapsedSeconds)}</span>
    {/if}
  {/snippet}

  <SessionControls />

  {#if !tracking}
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
        <span class="label">{headline.label}</span>
        <span class="figure">
          <span class="num hero">{headline.value}</span>
          {#if headline.sub}<span class="tabular sub">{headline.sub}</span>{/if}
        </span>
      </div>
      {#if recent}
        <div class="recent" title="Over the last five minutes of active time, against the whole session">
          <span class="label">Last 5 min</span>
          <span class="figure">
            <span class="num">{recent}</span>
            {#if pace != null && pace !== 0}
              <span class="pace" class:up={pace > 0} class:down={pace < 0}>{pace > 0 ? "▲" : "▼"} {Math.abs(pace)}%</span>
            {/if}
          </span>
        </div>
      {/if}
    </div>

    {#if lv != null && percent != null}
      <div class="level">
        <div class="level-line"><span class="muted">Lv {lv} to {lv + 1}</span><span class="tabular">{percent.toFixed(2)}%</span></div>
        <Meter value={percent / 100} label="EXP towards the next level" />
        {#if session.secondsToLevel != null}
          <span class="muted tabular">{formatDuration(session.secondsToLevel)} to level at this rate</span>
        {/if}
      </div>
    {/if}

    <div class="tiles">
      <Stat label="Mesos / hr" value={formatCompact(session.mesosPerHour)} />
      <Stat label="EXP gained" value={signed(session.expGained)} />
      <Stat label="Mesos gained" value={signed(session.mesosGained)} />
    </div>
  </Block>

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
    {#if points.filter((p) => p.y != null).length > 1}
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
        Each point is the rate over the 5 minutes before it.
        {#if session.events?.length}Green dots are level-ups, red ones deaths.{/if}
      </p>
    {:else}
      <p class="caption">The graph starts after a minute or so of EXP gains.</p>
    {/if}
  </Block>

  {#if here}
    <Block title="This map">
      {#snippet aside()}<span class="map-name">{here.name}</span>{/snippet}
      {@const best = here.saved?.best?.expPerHour ?? null}
      {@const top = Math.max(here.now ?? 0, best ?? 0, 1)}
      <div class="compare">
        <span class="muted">Now</span>
        <Meter value={(here.now ?? 0) / top} height={8} />
        <span class="tabular">{formatCompact(here.now)}</span>
        {#if best != null}
          <span class="muted">Your best here</span>
          <Meter value={best / top} height={8} color="var(--muted)" />
          <span class="tabular">{formatCompact(best)}</span>
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
                {#if row.map === session.map}<span class="here" title="You are here"></span>{/if}
                {row.map ? splitMapLabel(row.map).name : "Between maps"}
              </td>
              <td class="tabular">{formatDuration(row.activeSeconds)}</td>
              <td class="tabular">{town ? "town" : formatCompact(row.expPerHour)}</td>
              <td class="tabular">{town ? "town" : formatCompact(row.mesosPerHour)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
      {#if session.deaths > 0 || session.mapHidden || mesosStale || dropped > 0 || session.mesosExcluded !== 0}
        <ul class="notes">
          {#if session.mapHidden}<li class="attention">Can't see the minimap's name, so new gains may go to the wrong map.</li>{/if}
          {#if mesosStale}<li class="attention">Open your inventory now and then so mesos stay up to date.</li>{/if}
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
{:else if lastOne}
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
          <span class="what">{s.label || mainMap(s.maps)}</span>
          <span class="tabular">{formatCompact(s.expPerHour)}</span>
        </li>
      {/each}
    </ul>
    <div><Button onclick={() => backend.showTraining()}>All sessions</Button></div>
  </Block>
{/if}

<SpotList {level} {map} {session} />

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

  .sub {
    font-size: var(--text-sm);
    color: var(--muted);
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
