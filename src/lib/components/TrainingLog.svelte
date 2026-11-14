<script lang="ts">
  // The training log: every saved session with its graph, rates per map by level and party, the
  // player's records per map by level, party size and job, and progress over days. Its own window opens from the Train tab, and the desk view can show it
  // in a column (embedded). Everything here stays on this PC.
  import { clipped, tip } from "$lib/tip.svelte";
  import Ref from "$lib/components/Ref.svelte";
  import { Download, PenLine, Share2 } from "@lucide/svelte";
  import { onMount, untrack } from "svelte";

  import DropLogView from "./DropLogView.svelte";
  import Logo from "./Logo.svelte";
  import MapNote from "./MapNote.svelte";
  import MapRecords from "./MapRecords.svelte";
  import PotionCosts from "./PotionCosts.svelte";
  import { netMesosPerHour } from "$lib/potions.svelte";
  import ShareDialog from "./ShareDialog.svelte";
  import { backend } from "$lib/api";
  import { dropsBetween, watchDrops } from "$lib/drops.svelte";
  import { showItemNamed } from "$lib/items.svelte";
  import { watchMapData, mapData } from "$lib/mapdata.svelte";
  import { showMonsterNamed } from "$lib/monsters.svelte";
  import { showMapNamed } from "$lib/places.svelte";
  import { formatCompact, formatDuration, formatNumber, partyLabel, splitMapLabel } from "$lib/format";
  import { characters as activeCharacter } from "$lib/characters.svelte";
  import { history, watchHistory } from "$lib/history.svelte";
  import { live, watchLive } from "$lib/readings.svelte";
  import { watchSettings } from "$lib/settings.svelte";
  import {
    chartPoints,
    clock,
    days,
    eventMarkers,
    longDay,
    mapBands,
    mapBars,
    levelBands,
    bandRange,
    mapStats,
    pointDetail,
    rateFormat,
    shortDay,
    showUnplaced,
    type LineFilter,
    type Metric,
  } from "$lib/training";
  import type { SavedSession, SessionTimeline } from "$lib/types";
  import BarList from "$lib/ui/BarList.svelte";
  import Button from "$lib/ui/Button.svelte";
  import Columns from "$lib/ui/Columns.svelte";
  import LineChart from "$lib/ui/LineChart.svelte";
  import Meter from "$lib/ui/Meter.svelte";
  import Segmented from "$lib/ui/Segmented.svelte";
  import Stat from "$lib/ui/Stat.svelte";

  /** In a desk view column: no title of its own, and the views in a row above. */
  let { embedded = false }: { embedded?: boolean } = $props();

  type View = "sessions" | "maps" | "records" | "progress" | "drops";
  type Range = "7" | "30" | "all";

  const VIEW_KEY = "leafline.training.view";

  onMount(() => {
    watchHistory();
    watchDrops();
    watchMapData();
    watchLive();
    watchSettings();
  });

  let view = $state<View>(stored(VIEW_KEY, "sessions") as View);
  let range = $state<Range>("30");
  let character = $state<string>("");
  /** Once the player has picked a character (or All), their pick stands. */
  let characterChosen = false;
  let metric = $state<Metric>("exp");
  let selectedId = $state<number | null>(null);
  let timeline = $state<SessionTimeline | null>(null);
  let note = $state("");
  let noteTimer: ReturnType<typeof setTimeout> | undefined;
  let confirmDelete = $state(false);
  let sharing = $state(false);
  let problem = $state<string | null>(null);
  let exported = $state<string | null>(null);

  // Fixing a misread map name, from a session's map table.
  let renaming = $state<string | null>(null);
  let newName = $state("");

  function startRename(map: string) {
    renaming = map;
    newName = splitMapLabel(map).name;
  }

  async function finishRename(map: string) {
    const { street, name } = splitMapLabel(map);
    const next = newName.trim();
    renaming = null;
    if (!next || next === name) return;
    try {
      await backend.renameMap(map, street ? `${street} : ${next}` : next);
    } catch (e) {
      problem = String(e);
    }
  }

  // Maps view.
  let band = $state<string>("any");
  let party = $state<"solo" | "party" | "any">("any");
  let pickedMap = $state<string | null>(null);

  // Progress view.
  let dayMetric = $state<"exp" | "time" | "mesos">("exp");

  const all = $derived(history.value?.sessions ?? []);
  const characters = $derived([...new Set(all.map((s) => s.character).filter((c): c is string => !!c))]);
  // With more than one character, open on the one played last: a blend of two characters' levels
  // and maps reads as nonsense, so All is there to ask for.
  $effect(() => {
    if (!characterChosen && characters.length > 1) character = activeCharacter.viewing ?? (activeCharacter.all ? "" : (all[0]?.character ?? ""));
  });
  const since = $derived(range === "all" ? null : Date.now() / 1000 - Number(range) * 86400);
  const sessions = $derived(
    all.filter(
      (s) => (!character || s.character === character) && (since == null || (s.startedAt ?? s.savedAt) >= since),
    ),
  );
  const totalSeconds = $derived(sessions.reduce((a, s) => a + s.activeSeconds, 0));
  const selected = $derived(sessions.find((s) => s.id === selectedId) ?? sessions[0] ?? null);
  /** Drops logged while the session shown was running. */
  const sessionDrops = $derived.by(() => {
    const s = selected;
    if (!s) return [];
    const start = s.startedAt ?? s.savedAt - s.elapsedSeconds;
    return dropsBetween(start, start + s.elapsedSeconds + 60, s.character);
  });
  const sessionDropRows = $derived.by(() => {
    const rows = new Map<string, { item: string; monster: string; count: number }>();
    for (const d of sessionDrops) {
      const key = `${d.item}|${d.monster}`.toLowerCase();
      const row = rows.get(key) ?? { item: d.item, monster: d.monster, count: 0 };
      row.count += 1;
      rows.set(key, row);
    }
    return [...rows.values()].sort((x, y) => y.count - x.count);
  });

  // Load the graph for the session shown.
  $effect(() => {
    const id = selected?.id;
    timeline = null;
    confirmDelete = false;
    if (id == null) return;
    untrack(() => (note = selected?.note ?? ""));
    backend
      .sessionTimeline(id)
      .then((t) => {
        if (selected?.id === id) timeline = t;
      })
      .catch(() => (timeline = { points: [], events: [] }));
  });

  function stored(key: string, fallback: string) {
    try {
      return localStorage.getItem(key) ?? fallback;
    } catch {
      return fallback;
    }
  }

  function pickView(next: View) {
    view = next;
    try {
      localStorage.setItem(VIEW_KEY, next);
    } catch {
      // A convenience only.
    }
  }

  const dur = (seconds: number | null | undefined) => formatDuration(seconds);
  const mainMap = (s: SavedSession) => (s.maps.length ? splitMapLabel(s.maps[0].map).name : "No map read");
  const title = (s: SavedSession) => s.label || mainMap(s);
  const levelText = (s: SavedSession) => {
    if (s.level == null) return null;
    const start = s.level - s.levelUps;
    return s.levelUps ? `Lv ${start} → ${s.level}` : `Lv ${s.level}`;
  };
  const percentOnly = (s: SavedSession) => s.expPerHour == null && s.percentPerHour != null;
  const expRate = (s: SavedSession) =>
    percentOnly(s) ? `${s.percentPerHour?.toFixed(1)}%` : formatCompact(s.expPerHour);

  function saveNote(value: string) {
    note = value;
    const id = selected?.id;
    if (id == null) return;
    clearTimeout(noteTimer);
    noteTimer = setTimeout(() => backend.setSessionNote(id, value).catch((e) => (problem = String(e))), 500);
  }

  async function remove() {
    if (!selected) return;
    if (!confirmDelete) {
      confirmDelete = true;
      return;
    }
    try {
      await backend.deleteSession(selected.id);
      selectedId = null;
    } catch (e) {
      problem = String(e);
    }
  }

  async function exportCsv() {
    exported = null;
    try {
      const rows = await backend.exportCsv(view === "drops" ? "drops" : "sessions");
      if (rows != null) exported = `Saved ${rows} row${rows === 1 ? "" : "s"}.`;
    } catch (e) {
      problem = String(e);
    }
  }

  // Maps view: level bands that hold some play, then the stats under the filters.
  const bands = $derived(levelBands(sessions));
  const filter = $derived<LineFilter>({
    levels: bandRange(band),
    party,
  });
  const stats = $derived(mapStats(sessions, filter));
  const bars = $derived(mapBars(stats, metric));
  const picked = $derived(stats.find((m) => m.map === pickedMap) ?? stats[0] ?? null);

  // The picked map's id in the map data, which notes are kept by. Sessions name maps as read.
  let pickedId = $state<number | null>(null);
  $effect(() => {
    const label = picked?.map ?? null;
    void mapData.status?.ready;
    pickedId = null;
    if (!label || !mapData.status?.ready) return;
    const { street, name } = splitMapLabel(label);
    backend
      .findMap(name, street, null)
      .then((match) => {
        if (picked?.map === label) pickedId = match?.card.map.id ?? null;
      })
      .catch(() => {});
  });

  // Progress view.
  const dayRows = $derived(days(sessions));
  const columns = $derived(
    dayRows.map((d) => ({
      label: shortDay(d.start),
      value: dayMetric === "exp" ? d.expGained : dayMetric === "mesos" ? d.mesosGained : d.activeSeconds,
      display: dayMetric === "time" ? dur(d.activeSeconds) : formatCompact(dayMetric === "exp" ? d.expGained : d.mesosGained),
      detail: `${longDay(d.start)}: ${dur(d.activeSeconds)} played in ${d.sessions} session${d.sessions === 1 ? "" : "s"}, ${formatCompact(d.expGained)} EXP, ${formatCompact(d.mesosGained)} mesos`,
    })),
  );
  const totals = $derived.by(() => {
    const exp = sessions.reduce((a, s) => a + s.expGained, 0);
    const mesos = sessions.reduce((a, s) => a + s.mesosGained, 0);
    const levels = sessions.map((s) => s.level).filter((l): l is number => l != null);
    const ups = sessions.reduce((a, s) => a + s.levelUps, 0);
    const first = sessions.length ? sessions[sessions.length - 1] : null;
    const start = first?.level != null ? first.level - first.levelUps : null;
    return {
      exp,
      mesos,
      ups,
      from: start,
      to: levels.length ? Math.max(...levels) : null,
      days: dayRows.length,
      expPerHour: totalSeconds > 0 ? (exp * 3600) / totalSeconds : null,
      mesosPerHour: totalSeconds > 0 ? (mesos * 3600) / totalSeconds : null,
    };
  });
  /** Active time at each level, from the sessions' lines: how long each level took. */
  const perLevel = $derived.by(() => {
    const by = new Map<number, { seconds: number; maps: Map<string, number> }>();
    for (const s of sessions)
      for (const l of s.lines) {
        if (l.level == null) continue;
        const row = by.get(l.level) ?? { seconds: 0, maps: new Map() };
        row.seconds += l.activeSeconds;
        if (l.map) row.maps.set(l.map, (row.maps.get(l.map) ?? 0) + l.activeSeconds);
        by.set(l.level, row);
      }
    const top = Math.max(...totals.to != null ? [totals.to] : [0]);
    return [...by]
      .sort((a, b) => a[0] - b[0])
      .map(([level, row]) => ({
        id: String(level),
        label: `Lv ${level} → ${level + 1}`,
        sub: [...row.maps]
          .sort((a, b) => b[1] - a[1])
          .slice(0, 2)
          .map(([m]) => splitMapLabel(m).name)
          .join(", "),
        value: row.seconds,
        display: dur(row.seconds) + (level === top ? " so far" : ""),
        faded: level === top,
      }));
  });
  const current = $derived(live.session && live.session.activeSeconds > 0 ? live.session : null);
  const paceSeconds = $derived(totals.days > 0 ? totalSeconds / totals.days : null);
</script>

{#if sharing && selected}
  <ShareDialog session={selected} {timeline} drops={sessionDrops} onclose={() => (sharing = false)} />
{/if}

<div class="page" class:embedded>
  <header>
    {#if !embedded}<span class="brand"><Logo size={18} /><h1 class="num">Training log</h1></span>{/if}
    {#if characters.length > 1}
      <label class="field">
        Character
        <select bind:value={character} onchange={() => (characterChosen = true)}>
          <option value="">All characters</option>
          {#each characters as name (name)}<option value={name}>{name}</option>{/each}
        </select>
      </label>
    {:else if characters.length === 1}
      <span class="who">{characters[0]}</span>
    {/if}
    <Segmented
      label="Time range"
      value={range}
      options={[
        { id: "7", label: "7 days" },
        { id: "30", label: "30 days" },
        { id: "all", label: "All" },
      ]}
      onchange={(r) => (range = r)}
    />
    <span class="grow"></span>
    {#if exported}<span class="quiet">{exported}</span>{/if}
    <Button onclick={exportCsv}><Download size={13} />Export CSV</Button>
  </header>

  <div class="body">
    <nav aria-label="Training log">
      {#each [["sessions", "Sessions"], ["maps", "Maps"], ["records", "Records"], ["progress", "Progress"], ["drops", "Drops"]] as [id, name] (id)}
        <button class="nav" aria-current={view === id ? "page" : undefined} onclick={() => pickView(id as View)}>{name}</button>
      {/each}
    </nav>

    <main>
      {#if problem}<p class="problem" role="alert">{problem}</p>{/if}

      {#if view === "drops"}
        <DropLogView {character} {since} />
      {:else if !history.value}
        <p class="quiet">Loading…</p>
      {:else if !sessions.length}
        <div class="empty">
          <strong>No sessions {range === "all" ? "yet" : `in the last ${range} days`}</strong>
          <span>Sessions save themselves while live reading tracks your play. They stay on this PC.</span>
        </div>
      {:else if view === "sessions"}
        <div class="split">
          <section class="list panel">
            <div class="list-head">
              <h2>Sessions</h2>
              <span class="quiet tabular">{sessions.length} · {dur(totalSeconds)}</span>
            </div>
            <div class="srow head"><span>Date</span><span>Main map</span><span class="r">Played</span><span class="r">EXP / hr</span></div>
            {#each sessions as s (s.id)}
              <button class="srow" aria-pressed={s.id === selected?.id} onclick={() => (selectedId = s.id)}>
                <span class="tabular muted">{shortDay(s.startedAt ?? s.savedAt)}</span>
                <span class="what">
                  <span use:clipped class="ellipsis">{title(s)}</span>
                  {#if levelText(s)}<span class="small">{levelText(s)}</span>{/if}
                </span>
                <span class="tabular r">{dur(s.activeSeconds)}</span>
                <span class="tabular r strong">{expRate(s)}</span>
              </button>
            {/each}
          </section>

          {#if selected}
            {@const s = selected}
            <section class="detail">
              <div class="detail-head">
                <div>
                  <h2 class="num big">{longDay(s.startedAt ?? s.savedAt)} · {title(s)}</h2>
                  <p class="muted tabular">
                    {[
                      s.startedAt ? `Started ${clock(s.startedAt)}` : null,
                      levelText(s),
                      s.job,
                      s.character,
                      `${dur(s.activeSeconds)} active of ${dur(s.elapsedSeconds)}`,
                    ]
                      .filter(Boolean)
                      .join(" · ")}
                  </p>
                </div>
                <div class="actions">
                  <Button onclick={() => (sharing = true)}><Share2 size={13} />Share</Button>
                  <Button variant="danger" onclick={remove}>{confirmDelete ? "Delete for good?" : "Delete"}</Button>
                </div>
              </div>

              <div class="tiles">
                <Stat size="lg" label={percentOnly(s) ? "% / hr" : "EXP / hr"} value={expRate(s)} />
                <Stat
                  size="lg"
                  label="Mesos / hr"
                  value={formatCompact(s.mesosPerHour)}
                  sub={s.potionCost > 0 ? `${formatCompact(netMesosPerHour(s.mesosPerHour, s.potionCost, s.activeSeconds))} after potions` : null}
                />
                <Stat size="lg" label="Played" value={dur(s.activeSeconds)} />
                <Stat size="lg" label="EXP gained" value={formatNumber(s.expGained)} />
                <Stat size="lg" label="Levels" value={String(s.levelUps)} />
                <Stat size="lg" label="Deaths" value={String(s.deaths)} />
              </div>

              <div class="panel">
                <PotionCosts startedAt={s.startedAt} activeSeconds={s.activeSeconds} mesosPerHour={s.mesosPerHour} character={s.character} />
              </div>

              <div class="panel chart">
                <div class="list-head">
                  <h3>Rate over the session</h3>
                  <Segmented
                    size="sm"
                    label="Graph shows"
                    value={metric}
                    options={[
                      { id: "exp", label: percentOnly(s) ? "% / hr" : "EXP / hr" },
                      { id: "mesos", label: "Mesos / hr" },
                    ]}
                    onchange={(m) => (metric = m)}
                  />
                </div>
                {#if timeline && timeline.points.length > 1}
                  {@const t = timeline}
                  <LineChart
                    points={chartPoints(t.points, metric, percentOnly(s))}
                    xMax={Math.max(s.activeSeconds, t.points[t.points.length - 1].at)}
                    color={metric === "exp" ? "var(--exp)" : "var(--accent)"}
                    format={rateFormat(metric, percentOnly(s))}
                    bands={mapBands(t.points)}
                    markers={eventMarkers(t.events)}
                    detail={(i) => pointDetail(t.points[i])}
                    label="{metric === 'exp' ? 'EXP' : 'Mesos'} per hour over the session"
                    height={210}
                  />
                  <p class="caption">
                    <span class="key"><span class="dot good"></span>Level up</span>
                    <span class="key"><span class="dot bad"></span>Death</span>
                    Shaded stretches are maps. Each point is the rate over the 5 minutes before it; the first 2½ minutes are left out.
                    {#if metric === "mesos"}Mesos are spread evenly between your looks at them.{/if}
                  </p>
                {:else if timeline}
                  <p class="quiet">No graph for this session: it was saved before Leafline kept timelines, or was too short.</p>
                {:else}
                  <p class="quiet">Loading the graph…</p>
                {/if}
              </div>

              <div class="split">
                <div class="grow2">
                  <h3>By map</h3>
                  <table>
                    <thead><tr><th>Map</th><th>Level</th><th>Party</th><th>Time</th><th>EXP / hr</th><th>Mesos / hr</th></tr></thead>
                    <tbody>
                      {#each s.lines.filter((l) => showUnplaced(l, s.activeSeconds)) as l, i (i)}
                        <tr>
                          <td>
                            {#if l.map && renaming === l.map}
                              {@const map = l.map}
                              <!-- svelte-ignore a11y_autofocus -->
                              <input
                                class="rename"
                                aria-label="Map name"
                                bind:value={newName}
                                autofocus
                                onblur={() => finishRename(map)}
                                onkeydown={(e) => {
                                  if (e.key === "Enter") e.currentTarget.blur();
                                  if (e.key === "Escape") renaming = null;
                                }}
                              />
                            {:else}
                              {#if l.map}
                                {@const map = l.map}
                                <Ref kind="map" name={map}>{splitMapLabel(map).name}</Ref>
                              {:else}Not placed on a map{/if}
                              {#if l.map}
                                {@const map = l.map}
                                <button class="pen" use:tip={"Rename"} aria-label="Rename map" onclick={() => startRename(map)}
                                  ><PenLine size={11} /></button
                                >
                              {/if}
                            {/if}
                          </td>
                          <td>{l.level ?? "—"}</td>
                          <td>{partyLabel(l.party) ?? "Solo"}</td>
                          <td>{dur(l.activeSeconds)}</td>
                          <td>{l.activeSeconds > 0 ? (l.expGained > 0 ? formatCompact((l.expGained * 3600) / l.activeSeconds) : l.percentGained > 0 ? `${((l.percentGained * 3600) / l.activeSeconds).toFixed(1)}%` : "—") : "—"}</td>
                          <td>{l.mesosSeconds > 0 ? formatCompact((l.mesosGained * 3600) / l.mesosSeconds) : "—"}</td>
                        </tr>
                      {/each}
                    </tbody>
                  </table>
                </div>
                <div class="grow1">
                  {#if sessionDropRows.length}
                    <h3>Drops you logged</h3>
                    <ul class="drops">
                      {#each sessionDropRows as d (d.item + d.monster)}
                        <li>
                          <span><Ref kind="item" name={d.item} /></span>
                          <span class="muted"><Ref kind="mob" name={d.monster} />{d.count > 1 ? ` ×${d.count}` : ""}</span>
                        </li>
                      {/each}
                    </ul>
                  {/if}
                  <label for="note"><h3>Note</h3></label>
                  <textarea
                    id="note"
                    rows="4"
                    placeholder="What you tried, where you stood, who you partied with"
                    value={note}
                    oninput={(e) => saveNote(e.currentTarget.value)}
                  ></textarea>
                </div>
              </div>
            </section>
          {/if}
        </div>
      {:else if view === "records"}
        <MapRecords {sessions} />
      {:else if view === "maps"}
        <div class="filters">
          <span class="field">
            Your level
            <Segmented
              label="Your level then"
              value={band}
              options={[...bands, { id: "any", label: "Any" }]}
              onchange={(b) => (band = b)}
            />
          </span>
          <span class="field">
            Party
            <Segmented
              label="Party"
              value={party}
              options={[
                { id: "solo", label: "Solo" },
                { id: "party", label: "In a party" },
                { id: "any", label: "Either" },
              ]}
              onchange={(p) => (party = p)}
            />
          </span>
        </div>

        {#if !stats.length}
          <p class="quiet">No play matches these filters.</p>
        {:else}
          <div class="split">
            <section class="panel grow3">
              <div class="list-head">
                <h2>Your {metric === "exp" ? "EXP" : "mesos"} per hour by map</h2>
                <Segmented
                  size="sm"
                  label="Bars show"
                  value={metric}
                  options={[
                    { id: "exp", label: "EXP / hr" },
                    { id: "mesos", label: "Mesos / hr" },
                  ]}
                  onchange={(m) => (metric = m)}
                />
              </div>
              <BarList
                rows={bars}
                color={metric === "exp" ? "var(--exp)" : "var(--accent)"}
                selected={picked?.map ?? null}
                onselect={(id) => (pickedMap = id)}
                label="Maps by rate"
              />
              <p class="caption">Faded bars rest on under 30 minutes of play, so treat them as a rough guide.</p>
            </section>

            {#if picked}
              <section class="grow2 picked">
                <div>
                  <h2 class="num big">{picked.name}</h2>
                  {#if picked.best}
                    <p class="muted">
                      Best session {longDay(picked.best.at)}, {formatCompact(picked.best.expPerHour)} EXP / hr{picked.best.level != null
                        ? ` at Lv ${picked.best.level}`
                        : ""}
                    </p>
                  {/if}
                </div>
                <div class="tiles two">
                  <Stat size="lg" label="EXP / hr" value={formatCompact(picked.expPerHour)} />
                  <Stat size="lg" label="Mesos / hr" value={formatCompact(picked.mesosPerHour)} />
                  <Stat size="lg" label="Played" value={dur(picked.activeSeconds)} />
                  <Stat size="lg" label="Sessions" value={String(picked.sessions)} />
                </div>
                {#if pickedId != null}<MapNote mapId={pickedId} name={picked.name} flush />{/if}
                <div>
                  <h3>By your level</h3>
                  <table>
                    <thead><tr><th>Level</th><th>Played</th><th>EXP / hr</th><th>Mesos / hr</th></tr></thead>
                    <tbody>
                      {#each picked.byLevel as row (row.level)}
                        <tr>
                          <td>{row.level != null ? `Lv ${row.level}` : "Unknown"}</td>
                          <td>{dur(row.activeSeconds)}</td>
                          <td>{formatCompact(row.expPerHour)}</td>
                          <td>{formatCompact(row.mesosPerHour)}</td>
                        </tr>
                      {/each}
                    </tbody>
                  </table>
                </div>
              </section>
            {/if}
          </div>

          <section class="panel">
            <h2>Every map you trained on</h2>
            <div class="scroll">
              <table class="wide">
                <thead><tr><th>Map</th><th>Sessions</th><th>Played</th><th>EXP / hr</th><th>Mesos / hr</th><th>Best session</th></tr></thead>
                <tbody>
                  {#each stats as m (m.map)}
                    <tr>
                      <td><Ref kind="map" name={m.map}>{m.name}</Ref></td>
                      <td>{m.sessions}</td>
                      <td>{dur(m.activeSeconds)}</td>
                      <td>{formatCompact(m.expPerHour)}</td>
                      <td>{formatCompact(m.mesosPerHour)}</td>
                      <td>{m.best ? `${shortDay(m.best.at)} · ${formatCompact(m.best.expPerHour)}` : "—"}</td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
          </section>
        {/if}
      {:else}
        <div class="tiles four">
          <Stat size="lg" label="Played" value={dur(totalSeconds)} sub="{totals.days} day{totals.days === 1 ? '' : 's'}{paceSeconds ? `, ${dur(paceSeconds)} a day` : ''}" />
          <Stat
            size="lg"
            label="Levels"
            value={totals.from != null && totals.to != null && totals.to !== totals.from ? `${totals.from} → ${totals.to}` : String(totals.ups)}
            sub="{totals.ups} level-up{totals.ups === 1 ? '' : 's'}"
          />
          <Stat size="lg" label="EXP gained" value={formatNumber(totals.exp)} sub={totals.expPerHour ? `${formatCompact(totals.expPerHour)} an hour` : null} />
          <Stat size="lg" label="Mesos gained" value={formatNumber(totals.mesos)} sub={totals.mesosPerHour ? `${formatCompact(totals.mesosPerHour)} an hour` : null} />
        </div>

        <div class="split">
          <section class="panel grow3">
            <div class="list-head">
              <h2>{dayMetric === "exp" ? "EXP per day" : dayMetric === "mesos" ? "Mesos per day" : "Time played per day"}</h2>
              <Segmented
                size="sm"
                label="Columns show"
                value={dayMetric}
                options={[
                  { id: "exp", label: "EXP" },
                  { id: "time", label: "Time played" },
                  { id: "mesos", label: "Mesos" },
                ]}
                onchange={(m) => (dayMetric = m)}
              />
            </div>
            <Columns
              {columns}
              color={dayMetric === "exp" ? "var(--exp)" : dayMetric === "mesos" ? "var(--accent)" : "var(--muted)"}
              format={(v) => (dayMetric === "time" ? dur(v) : formatCompact(v))}
              label="Play per day"
            />
          </section>

          {#if current && current.level != null && current.percent != null}
            <section class="panel grow2">
              <h2>Next level</h2>
              <div class="figure">
                <span class="num huge">Lv {current.level + 1}</span>
                {#if current.secondsToLevel != null}<span class="muted">in about {dur(current.secondsToLevel)} of play</span>{/if}
              </div>
              <Meter value={current.percent / 100} label="EXP towards the next level" />
              <p class="muted tabular">{current.percent.toFixed(2)}% of Lv {current.level}</p>
              {#if current.secondsToLevel != null && paceSeconds}
                <p class="muted">
                  At your pace of {dur(paceSeconds)} a day, that is about {Math.max(1, Math.ceil(current.secondsToLevel / paceSeconds))}
                  day{Math.ceil(current.secondsToLevel / paceSeconds) === 1 ? "" : "s"} away. Based on this session's rate.
                </p>
              {/if}
            </section>
          {/if}
        </div>

        {#if perLevel.length}
          <section class="panel">
            <h2>Time at each level</h2>
            <BarList rows={perLevel} label="Active time at each level" />
            <p class="caption">Active play only. Levels you started or finished outside these sessions show only part of their time.</p>
          </section>
        {/if}
      {/if}
    </main>
  </div>
</div>

<style>
  .drops {
    margin: 0 0 12px;
    padding: 0;
    list-style: none;
    font-size: var(--text-sm);
  }

  .drops li {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    padding: 3px 0;
    border-top: 1px solid var(--border);
  }

  .page {
    display: flex;
    flex-direction: column;
    min-height: 100vh;
  }

  /* In a desk view column, which scrolls and names it already. */
  .embedded {
    min-height: 0;
  }

  .embedded header {
    padding: 0 0 var(--space-3);
  }

  .embedded .body {
    flex-direction: column;
    align-items: stretch;
    gap: var(--space-4);
    padding: var(--space-3) 0 0;
  }

  .embedded nav {
    flex: none;
    flex-direction: row;
    flex-wrap: wrap;
    max-width: none;
  }

  .embedded main {
    flex: none;
  }

  header {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-5);
    border-bottom: 1px solid var(--border);
  }

  .brand {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    margin-right: var(--space-3);
  }

  h1 {
    margin: 0;
    font-size: var(--text-lg);
    font-weight: 600;
  }

  h2,
  h3 {
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--text-md);
    font-weight: 400;
    color: var(--muted);
  }

  h2.big {
    font-size: var(--text-xl);
    font-weight: 600;
    color: var(--text);
  }

  .field {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-sm);
    color: var(--muted);
  }

  select {
    height: var(--control-h);
    padding: 0 6px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
  }

  .who {
    font-size: var(--text-sm);
    color: var(--muted);
  }

  .grow {
    flex: 1;
  }

  .body {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-5);
    padding: var(--space-4) var(--space-5);
    align-items: flex-start;
  }

  nav {
    display: flex;
    flex: 1 1 140px;
    flex-direction: column;
    gap: 2px;
    max-width: 180px;
  }

  .nav {
    height: 34px;
    padding: 0 var(--space-3);
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--muted);
    font-size: var(--text-md);
    text-align: left;
    cursor: pointer;
  }

  .nav:hover {
    background: var(--surface-2);
    color: var(--text);
  }

  .nav[aria-current="page"] {
    background: var(--surface-3);
    color: var(--text);
    font-weight: 600;
  }

  main {
    display: flex;
    flex: 999 1 560px;
    flex-direction: column;
    gap: var(--space-5);
    min-width: 0;
  }

  .split {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-5);
    align-items: flex-start;
  }

  .panel {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
  }

  .list {
    flex: 1 1 340px;
    min-width: 0;
    gap: 0;
    padding: 0;
  }

  .list-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
  }

  .list > .list-head {
    padding: var(--space-3);
  }

  .srow {
    display: grid;
    grid-template-columns: 64px minmax(0, 1fr) 64px 60px;
    gap: var(--space-2);
    align-items: center;
    width: 100%;
    padding: 9px var(--space-3);
    border: 0;
    border-top: 1px solid var(--border);
    background: transparent;
    color: var(--text);
    font-size: var(--text-sm);
    text-align: left;
    cursor: pointer;
  }

  .srow:hover {
    background: var(--surface-2);
  }

  .srow[aria-pressed="true"] {
    background: var(--surface-3);
  }

  .srow.head {
    padding-top: 0;
    padding-bottom: 6px;
    border-top: 0;
    font-size: var(--text-xs);
    color: var(--faint);
    cursor: default;
  }

  .srow.head:hover {
    background: transparent;
  }

  .what {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .ellipsis {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .small {
    font-size: var(--text-xs);
    color: var(--faint);
  }

  .r {
    text-align: right;
  }

  .strong {
    font-weight: 600;
  }

  .muted {
    color: var(--muted);
  }

  .detail {
    display: flex;
    flex: 2 1 520px;
    flex-direction: column;
    gap: var(--space-4);
    min-width: 0;
  }

  .detail-head {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--space-3);
  }

  .detail-head p {
    margin: 4px 0 0;
    font-size: var(--text-sm);
  }

  .actions {
    display: flex;
    gap: var(--space-2);
  }

  .tiles {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(110px, 1fr));
    gap: var(--space-2);
  }

  .tiles.two {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .tiles.four {
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
  }

  .caption {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-4);
    margin: 0;
    font-size: var(--text-xs);
    color: var(--faint);
  }

  .key {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--muted);
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }

  .dot.good {
    background: var(--good);
  }

  .dot.bad {
    background: var(--bad);
  }

  .grow1 {
    display: flex;
    flex: 1 1 220px;
    flex-direction: column;
    gap: var(--space-2);
    min-width: 0;
  }

  .grow2 {
    display: flex;
    flex: 2 1 320px;
    flex-direction: column;
    gap: var(--space-2);
    min-width: 0;
  }

  .grow3 {
    flex: 3 1 420px;
    min-width: 0;
  }

  .picked {
    gap: var(--space-3);
  }

  .picked p {
    margin: 2px 0 0;
    font-size: var(--text-sm);
  }

  textarea {
    padding: var(--space-2);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
    resize: vertical;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--text-sm);
    font-variant-numeric: tabular-nums;
  }

  table.wide {
    min-width: 600px;
  }

  .scroll {
    overflow-x: auto;
  }

  th {
    padding: 0 0 6px;
    font-size: var(--text-xs);
    font-weight: 400;
    color: var(--faint);
    text-align: right;
  }

  td {
    padding: 7px 0;
    border-top: 1px solid var(--border);
    text-align: right;
  }

  th:first-child,
  td:first-child {
    text-align: left;
  }

  .pen {
    display: inline-grid;
    place-items: center;
    width: 20px;
    height: 20px;
    margin-left: 4px;
    padding: 0;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--faint);
    vertical-align: middle;
    cursor: pointer;
    opacity: 0;
  }

  tr:hover .pen,
  .pen:focus-visible {
    opacity: 1;
  }

  .pen:hover {
    color: var(--text);
  }

  .rename {
    width: 100%;
    height: 24px;
    padding: 0 6px;
    border: 1px solid var(--accent);
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--text);
    font: inherit;
  }

  .filters {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-4);
  }

  .figure {
    display: flex;
    align-items: baseline;
    gap: 10px;
  }

  .huge {
    font-size: var(--text-hero);
    font-weight: 600;
  }

  .panel p {
    margin: 0;
    font-size: var(--text-sm);
  }

  .empty {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: var(--space-5);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    color: var(--muted);
  }

  .empty strong {
    color: var(--text);
    font-size: var(--text-md);
  }

  .quiet {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--faint);
  }

  .problem {
    margin: 0;
    padding: var(--space-2) var(--space-3);
    background: var(--bad-soft);
    color: var(--bad);
    font-size: var(--text-sm);
  }
</style>
