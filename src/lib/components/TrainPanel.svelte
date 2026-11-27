<script lang="ts">
  // The Train tab, one of three views at a time, picked by chips. Session: the session being
  // played (rates, the level goal, how this map compares with your bests on it, the maps played, a
  // graph of the whole session), or before one starts, how it starts and the goal. Where to
  // train: the maps for your level. History: your best maps and the last sessions.
  import { can } from "$lib/platform";
  import TypedSessionCard from "./TypedSessionCard.svelte";
  import { tip } from "$lib/tip.svelte";
  import Ref from "$lib/components/Ref.svelte";
  import SessionControls from "./SessionControls.svelte";
  import { ArrowDown, Info } from "@lucide/svelte";

  import LevelGoal from "./LevelGoal.svelte";
  import PotionCosts from "./PotionCosts.svelte";
  import LootLog from "./LootLog.svelte";
  import { lootPerHour as lootRate } from "$lib/loot.svelte";
  import SaveSpot from "./SaveSpot.svelte";
  import SpotList, { SPOTS_ID, type BetterMap } from "./SpotList.svelte";
  import GoButton from "$lib/components/GoButton.svelte";
  import { mapPlace } from "$lib/destination.svelte";
  import TimersCard from "./TimersCard.svelte";
  import { openTrainingLog } from "$lib/nav.svelte";
  import { formatClockIn, formatCompact, formatDuration, formatNumber, partyLabel, splitMapLabel } from "$lib/format";
  import { characters } from "$lib/characters.svelte";
  import { history, watchHistory } from "$lib/history.svelte";
  import { live } from "$lib/readings.svelte";
  import {
    chartPoints,
    eventMarkers,
    mapBands,
    mapStats,
    longEnough,
    mesosCoverage,
    mesosReminder,
    pointDetail,
    rateFormat,
    shortDay,
    showUnplaced,
    deathCost,
    type Metric,
  } from "$lib/training";
  import type { MapReading, SessionSummary } from "$lib/types";
  import Block from "$lib/ui/Block.svelte";
  import Button from "$lib/ui/Button.svelte";
  import LineChart from "$lib/ui/LineChart.svelte";
  import Meter from "$lib/ui/Meter.svelte";
  import ExpBar from "$lib/ui/ExpBar.svelte";
  import Segmented from "$lib/ui/Segmented.svelte";
  import Stat from "$lib/ui/Stat.svelte";

  let {
    level,
    map = null,
    session = null,
    glance = false,
    whole = false,
    glanced = false,
    only = null,
  }: {
    level: number | null;
    map?: MapReading | null;
    session?: SessionSummary | null;
    /** A pinned window's short form: the session's rates and controls, and timers. The graph,
     *  the maps, where to train and past sessions stay in the tab. */
    glance?: boolean;
    /** Where there is room, as in the desk view: every view stacked, with no chips. */
    whole?: boolean;
    /** The desk view's player bar shows the big rate, the level line and the session buttons. */
    glanced?: boolean;
    /**
     * One part alone, for the desk view, which lays the parts out itself: the session's details,
     * where to train, this map against your bests with the maps played, or the session's graph.
     */
    only?: "session" | "where" | "maps" | "chart" | null;
  } = $props();

  watchHistory();

  const METRIC_KEY = "leafline.train.metric";
  let metric = $state<Metric>(storedMetric());

  const tracking = $derived(session != null && session.activeSeconds > 0);

  type View = "session" | "where" | "history";
  const VIEW_KEY = "leafline.train.view";
  const VIEWS: { id: View; label: string }[] = [
    { id: "session", label: "Session" },
    { id: "where", label: "Where to train" },
    { id: "history", label: "History" },
  ];
  function storedView(): View {
    try {
      const saved = localStorage.getItem(VIEW_KEY);
      if (VIEWS.some((v) => v.id === saved)) return saved as View;
    } catch {
      // Starts on the session.
    }
    return "session";
  }
  let view = $state<View>(storedView());
  function setView(next: View) {
    view = next;
    try {
      localStorage.setItem(VIEW_KEY, next);
    } catch {
      // A convenience only.
    }
  }
  /** What shows. A pinned window has only the session; the desk view stacks all of it. */
  const onSession = $derived(only ? only === "session" : glance || whole || view === "session");
  const onWhere = $derived(only ? only === "where" : !glance && (whole || view === "where"));
  const onHistory = $derived(!only && !glance && (whole ? !tracking : view === "history"));
  /** This map against your bests, and the maps played. */
  const onMaps = $derived(only ? only === "session" || only === "maps" : onSession);
  const onChart = $derived(only ? only === "session" || only === "chart" : onSession);
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
    // Your best here as you play now, solo or in a party: a party's EXP is your share of it.
    const inParty = (session.party ?? 1) > 1;
    const same = inParty ? saved?.party : saved?.solo;
    const other = inParty ? saved?.solo : saved?.party;
    return { name: splitMapLabel(session.map).name, now: now?.expPerHour ?? null, saved, inParty, best: same?.best ?? null, other };
  });

  /** Another map the forecast now puts above this one, from Where to train. */
  let better = $state<BetterMap | null>(null);
  let spotList = $state<{ wave: () => void } | null>(null);

  const coverage = $derived(session ? mesosCoverage(session) : null);
  // Mesos picked up, from the Pickup Log: the rate when the inventory's mesos were never seen
  // twice, and a check on it when they were.
  const pickedMesos = $derived(session?.loot?.watched ? (session.loot.mesosPerHour ?? null) : null);
  const mesosRate = $derived(session?.mesosPerHour ?? pickedMesos);
  const mesosSub = $derived.by(() => {
    if (!session) return null;
    if (session.mesosPerHour == null && pickedMesos != null) return "picked up";
    if (coverage?.partial) return coverage.label;
    if (pickedMesos != null) return `${formatCompact(pickedMesos)} picked up`;
    return null;
  });
  const lootValueRate = $derived(session?.loot?.watched ? lootRate(session.loot.items, session.loot.seconds) : null);
  const reminder = $derived(session ? mesosReminder(session) : null);

  /** How Mesos / hr is counted, behind the tile's info mark. */
  const MESOS_HELP =
    "Counts your mesos when you start and again before you spend. Your inventory or a shop window both count, and you don't need to keep either open.";

  const dropped = $derived(session ? session.rejected.exp + session.rejected.mesos : 0);
  /** About how much play the deaths undid, at this map's pace. */
  const deathPlay = $derived(
    session ? (deathCost({ ...session, expPerHour: session.paceExpPerHour ?? session.expPerHour })?.play ?? null) : null,
  );

  /** Your best maps by EXP an hour, for before a session. */
  const BESTS_SHOWN = 5;
  const bests = $derived(
    stats
      .filter((s) => s.best)
      .sort((a, b) => b.best!.expPerHour - a.best!.expPerHour)
      .slice(0, BESTS_SHOWN),
  );

  // Before a session: the last few saved ones.
  const lastSessions = $derived(characters.sessions(history.value?.sessions ?? []).filter(longEnough).slice(0, 3));
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

{#snippet nudge(b: BetterMap)}
  <p class="nudge">
    <span
      >At Lv {b.level}, <Ref kind="map" id={b.spot.map.id} name={b.spot.map.name} /> should beat this map now: about
      <span class="tabular">{formatCompact(b.there)}</span> EXP / hr against <span class="tabular">{formatCompact(b.here)}</span> here,
      by the forecast in Where to train.</span
    >
    <span class="nudge-actions">
      <GoButton place={mapPlace(b.spot.map)} />
      <button class="link" onclick={() => spotList?.wave()}>Not now</button>
    </span>
  </p>
{/snippet}

{#snippet mapsNote()}
  <span class="info" use:tip={"Each map's rate counts only your time on it. The session rate counts all of it, towns too."}
    ><Info size={13} aria-label="How these rates are counted" /></span
  >
{/snippet}

{#snippet jump()}
  <Button variant="ghost" size="sm" onclick={jumpToSpots}>
    Where to train <ArrowDown size={12} />
  </Button>
{/snippet}

{#if !glance && !whole && !only}
  <div class="views"><Segmented label="Train shows" options={VIEWS} value={view} onchange={setView} /></div>
{/if}
{#snippet sessionState()}
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

<!-- Before a session: how it starts. Once one runs, its heading and buttons head the card below. -->
{#if onSession && !(tracking && session && headline)}
<Block title="Session" aside={sessionState}>
  {#if !glanced}<SessionControls />{/if}

  {#if !tracking && glance}
    <p class="caption">Starts with your first EXP.</p>
  {:else if !tracking && !can.liveReading}
    <!-- The website reads nothing: a session is typed in. -->
    {#if !glanced}<TypedSessionCard />{/if}
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
{/if}

{#if tracking && session && headline && onSession}
  <Block title={glanced ? undefined : "Session"} aside={glanced ? undefined : sessionState}>
    {#if !glanced}
    <SessionControls trailing={whole && !glance ? jump : undefined} />
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
        <!-- The window's header has the level's bar; a pinned window has no header, so it draws its own. -->
        {#if glance}
          <div class="level-line"><span class="muted">Lv {lv} to {lv + 1}</span><span class="tabular">{percent.toFixed(2)}%</span></div>
          <ExpBar {percent} points={session.exp?.points ?? null} needed={session.expNeeded} />
        {/if}
        {#if toLevel || levelPace}
          <span class="muted tabular" use:tip={toLevel?.tip ?? ""}>
            {#if toLevel}{toLevel.line}{/if}{#if toLevel && levelPace}{" · "}{/if}{#if levelPace}{levelPace}{/if}
          </span>
          {#if toLevel?.breaks}<span class="muted tabular">{toLevel.breaks}</span>{/if}
        {/if}
      </div>
    {/if}
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
      <!-- On the desk, the player bar over the view has the rate. -->
      {#if !glanced}<Stat
        label="Mesos / hr"
        help={MESOS_HELP}
        value={formatCompact(mesosRate)}
        sub={mesosSub}
        faint={session.mesosPerHour != null && coverage?.partial && coverage.thin}
      />{/if}
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
        mesosPerHour={mesosRate}
        character={session.name ?? characters.active}
        quickSlots={session.quickSlots ?? null}
        lootPerHour={lootValueRate}
      />
      <LootLog loot={session.loot} reading={liveState === "reading"} />
    {/if}

    {#if reminder}
      <p class="reminder">{reminder}</p>
    {/if}
    {#if glance && session.mapHidden}
      <p class="attention">Can't see the minimap's name, so new gains may go to the wrong map.</p>
    {/if}
  </Block>

{:else if onSession && !glance}
  <!-- No session yet: how it starts, above, then the goal. -->
  <Block title="Level goal">
    <LevelGoal {level} percent={session?.percent ?? null} character={session?.name ?? characters.active} />
  </Block>
{/if}

<!-- This map against your bests, and every map this session. -->
{#if tracking && session && onMaps && !glance}
  {#if better && !here}
    <Block title="This map">
      {@render nudge(better)}
    </Block>
  {/if}
  {#if here}
    <Block title="This map">
      {#snippet aside()}<span class="map-name">{here.name}</span>{/snippet}
      {#if better}{@render nudge(better)}{/if}
      {@const best = here.best?.expPerHour ?? null}
      {@const top = Math.max(here.now ?? 0, best ?? 0, 1)}
      {@const bestMesos = here.saved?.bestMesos?.mesosPerHour ?? null}
      {@const mesosNow = session.maps.find((m) => m.map === session.map)?.mesosPerHour ?? null}
      {@const mesosTop = Math.max(mesosNow ?? 0, bestMesos ?? 0, 1)}
      <div class="compare">
        <span class="muted">EXP now</span>
        <Meter value={(here.now ?? 0) / top} height={8} />
        <span class="tabular">{formatCompact(here.now)}</span>
        {#if best != null}
          <span class="muted">{here.inParty ? "Party best" : "Solo best"}</span>
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
      {#if here.saved && (here.best || here.other)}
        <p class="caption">
          {#if here.best}
            Best: {here.best.level != null ? `Lv ${here.best.level}, ` : ""}{(here.best.party ?? 1) > 1
              ? `party of ${here.best.party}`
              : "solo"}, {shortDay(here.best.at)}.
          {:else}
            No {here.inParty ? "party" : "solo"} run here of 10 minutes yet.
          {/if}
          {#if here.other}
            {here.inParty ? "Solo" : "In a party"} you've averaged {formatCompact(here.other.expPerHour)} here.
          {/if}
          {formatDuration(here.saved.activeSeconds)} here over {here.saved.sessions}
          session{here.saved.sessions === 1 ? "" : "s"}.
        </p>
      {:else}
        <p class="caption">Your first time logged here. Your best shows once a run here passes 10 minutes.</p>
      {/if}
    </Block>
  {/if}

  {#if sessionMaps.length}
    <Block title="Maps this session" aside={sessionMaps.length > 1 ? mapsNote : undefined}>
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
      {#if session.deaths > 0 || session.mapHidden || dropped > 0 || session.mesosExcluded !== 0}
        <ul class="notes">
          {#if session.mapHidden}<li class="attention">Can't see the minimap's name, so new gains may go to the wrong map.</li>{/if}
          {#if session.deaths > 0}
            <li>
              {session.deaths} death{session.deaths === 1 ? "" : "s"}, {session.percentOnly
                ? `${session.percentLost.toFixed(2)}% of a level`
                : `${formatNumber(session.expLost)} EXP`} lost and left out of the rates{deathPlay ? `: ${deathPlay}` : ""}.
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
{/if}

{#if !glance && only !== "chart"}
  <!-- Stays in the page while another view shows: it works out the better-map line for This map. -->
  <div class="view" class:off={!onWhere}>
    <SpotList {level} {map} {session} bind:better bind:this={spotList} />
  </div>
{/if}

{#if tracking && session && headline && onChart && !glance}
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
        {#if session.events?.length}Green dots are level-ups, red ones deaths.{" "}{/if}<span
          class="info"
          use:tip={`Each point is the rate over the 5 minutes before it; the first 2½ minutes are left out.${metric === "mesos" ? " Mesos are spread evenly between your looks at them, so the line reaches only your last look." : ""}`}
          ><Info size={12} aria-label="How the graph is drawn" /></span
        >
      </p>
    {:else}
      <p class="caption">The graph starts after 5 minutes of play.</p>
    {/if}
  </Block>
{:else if only === "chart" && !glance}
  <!-- Now keeps the card's place before a session, saying what fills it. -->
  <Block title="This session">
    <p class="caption">Starts with your first EXP. The graph follows after 5 minutes of play.</p>
  </Block>
{/if}

{#if onHistory}
  {#if bests.length}
    <Block title="Your best maps">
      {#snippet aside()}<span>Best single run, 10 min or more</span>{/snippet}
      <table>
        <thead><tr><th>Map</th><th>EXP / hr</th><th>Mesos / hr</th><th>When</th></tr></thead>
        <tbody>
          {#each bests as m (m.map)}
            <tr>
              <td><Ref kind="map" name={m.map}>{m.name}</Ref> <SaveSpot label={m.map} name={m.name} /></td>
              <td class="tabular">{formatCompact(m.best!.expPerHour)}</td>
              <td class="tabular">{m.bestMesos ? formatCompact(m.bestMesos.mesosPerHour) : "—"}</td>
              <td class="tabular muted"
                >{m.best!.level != null ? `Lv ${m.best!.level} · ` : ""}{(m.best!.party ?? 1) > 1 ? `${partyLabel(m.best!.party)} · ` : ""}{shortDay(
                  m.best!.at,
                )}</td
              >
            </tr>
          {/each}
        </tbody>
      </table>
    </Block>
  {/if}
  {#if lastOne}
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
  {:else if !whole}
    <Block title="Past sessions">
      <p class="caption">
        Nothing saved yet. Sessions save when they end, and your best maps show here.
      </p>
      <div><Button onclick={openTrainingLog}>Training log</Button></div>
    </Block>
  {/if}
{/if}

{#if onSession && !only}<TimersCard />{/if}

<style>
  /* A block pads itself; the chips sit beside blocks, so they match it. */
  .views {
    display: flex;
    padding: 0 var(--space-3) var(--space-2);
  }

  .view {
    display: contents;
  }

  .view.off {
    display: none;
  }

  .state {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
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
    gap: var(--space-1);
    padding: var(--space-3);
    background: var(--surface-2);
    font-size: var(--text-md);
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
    gap: var(--space-0-5);
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
    font-weight: var(--weight-strong);
    line-height: 1;
  }

  .recent .num {
    font-size: var(--text-lg);
    font-weight: var(--weight-strong);
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
    gap: var(--space-1);
    font-size: var(--text-md);
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
    grid-auto-columns: minmax(0, 1fr);
    grid-auto-flow: column;
    padding: var(--space-2) 0;
    border-block: 1px solid var(--border);
  }

  /* Figures on the card itself, split by hairlines: no box inside the box. */
  .tiles :global(.stat) {
    padding: 0 var(--space-3);
    border: 0;
    border-left: 1px solid var(--border);
    border-radius: 0;
    background: none;
  }

  .tiles :global(.stat:first-child) {
    padding-left: 0;
    border-left: 0;
  }

  .info {
    display: inline-flex;
    color: var(--faint);
    cursor: help;
  }

  .reminder {
    margin: 0;
    padding: var(--space-1-5) var(--space-2);
    border-left: 2px solid var(--warn);
    background: var(--surface-2);
    font-size: var(--text-xs);
    color: var(--text);
  }


  .caption {
    margin: 0;
    font-size: var(--text-xs);
    color: var(--faint);
  }

  /* Another map should beat this one now: a quiet line, never an alert. */
  .nudge {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: var(--space-1) var(--space-2);
    margin: 0 0 var(--space-2);
    font-size: var(--text-md);
    color: var(--muted);
  }

  .nudge-actions {
    display: inline-flex;
    align-items: baseline;
    gap: var(--space-2);
  }

  .link {
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--faint);
    font: inherit;
    font-size: var(--text-xs);
    cursor: pointer;
  }

  .link:hover {
    color: var(--text);
  }

  .map-name {
    font-size: var(--text-md);
    font-weight: var(--weight-strong);
    color: var(--text);
  }

  .compare {
    display: grid;
    grid-template-columns: 96px minmax(0, 1fr) 44px;
    gap: var(--space-1-5) var(--space-2);
    align-items: center;
    font-size: var(--text-md);
  }

  .compare > :nth-child(3n) {
    text-align: right;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--text-md);
  }

  th {
    padding: 0 0 var(--space-1);
    font-size: var(--text-xs);
    font-weight: var(--weight-body);
    color: var(--faint);
    text-align: right;
  }

  td {
    padding: var(--space-1) 0;
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
    margin-right: var(--space-1-5);
    vertical-align: 1px;
    background: var(--accent);
  }

  .notes {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
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
    padding: var(--space-1-5) 0;
    border-top: 1px solid var(--border);
    font-size: var(--text-md);
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
