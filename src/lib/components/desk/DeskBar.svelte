<script lang="ts">
  // The desk view's player bar, over every view: who you are and whether Leafline is reading (on
  // the website, the Share button on the top row says that), how far through the level and when
  // the next one comes, the two rates and the session's clock. These numbers live here and nowhere
  // else on the desk, Now included, so the views under it don't repeat them.
  import { tip } from "$lib/tip.svelte";

  import Ref from "$lib/components/Ref.svelte";
  import SessionControls from "$lib/components/SessionControls.svelte";
  import { formatClockIn, formatCompact, formatDuration, formatNumber, partyLabel } from "$lib/format";
  import { ago, shownCharacter, watchProfile } from "$lib/profile.svelte";
  import { can } from "$lib/platform";
  import { display } from "$lib/settings.svelte";
  import { live } from "$lib/readings.svelte";
  import { place, regionHue, watchRegion } from "$lib/region.svelte";
  import { mesosCoverage } from "$lib/training";
  import LevelRing from "$lib/ui/LevelRing.svelte";
  import LookPicture from "$lib/ui/LookPicture.svelte";
  import { characters } from "$lib/characters.svelte";
  import { LOOK_TIP, lookOf, watchLooks } from "$lib/looks.svelte";
  import Sparkline from "$lib/ui/Sparkline.svelte";

  watchRegion();
  const hue = $derived(regionHue(place.region));

  watchProfile();
  watchLooks();

  // Read now when this character is on screen; else what was read of them last, faded and dated.
  // The session and its rates are only ever the one on screen's.
  const who = $derived(shownCharacter());
  const session = $derived(who.onScreen ? live.session : null);
  const tracking = $derived(session != null && session.activeSeconds > 0);
  const percentOnly = $derived(session?.percentOnly ?? false);
  const level = $derived(who.level.value);
  const percent = $derived(who.percent.value);
  const name = $derived(who.name);
  /** Their look from the game's Character Info window, kept from an earlier read. */
  const look = $derived(characters.all ? null : lookOf(name));
  const lookTip = $derived(
    look ? `${name}'s look, as of ${ago(look.readAt)}` : can.liveReading && display.live.characterLook !== false ? LOOK_TIP : null,
  );
  const job = $derived(who.job.value);
  const points = $derived(who.points);
  /** The map's own name, without its street, which the Map view gives; or the one they were last on. */
  const mapName = $derived(
    (who.onScreen ? (live.analysis?.map?.name ?? session?.map?.split(" : ").pop()) : null) ?? who.lastMap,
  );
  const party = $derived(partyLabel(session?.party));
  const reading = $derived(live.liveStatus.state === "reading");

  const status = $derived.by(() => {
    const s = live.liveStatus;
    if (s.state === "reading") return { tone: "good", text: "Reading", tip: `Reading ${s.window}` };
    if (s.state === "waiting") return { tone: "warn", text: "Waiting", tip: s.reason };
    if (s.state === "failed") return { tone: "bad", text: "Stopped", tip: `Live reading stopped: ${s.message}` };
    return {
      tone: "quiet",
      text: "Not reading",
      tip: can.liveReading ? "Not reading the game. Turn live reading on in the overlay" : "Not reading the game",
    };
  });

  const big = (value: number | null | undefined) =>
    value == null ? "—" : Math.abs(value) >= 1e5 ? formatCompact(value) : formatNumber(Math.round(value));

  const rate = $derived.by(() => {
    if (!tracking || !session) return "—";
    if (percentOnly) return session.percentPerHour != null ? `${session.percentPerHour.toFixed(1)}%` : "—";
    return big(session.expPerHour);
  });
  const recent = $derived(percentOnly ? session?.recentPercentPerHour : session?.recentExpPerHour);
  const whole = $derived(percentOnly ? session?.percentPerHour : session?.expPerHour);
  /** How the last five minutes compare with the session, in percent. */
  const pace = $derived(recent != null && whole ? Math.round((recent / whole - 1) * 100) : null);
  const coverage = $derived(session ? mesosCoverage(session) : null);
  const toLevel = $derived(tracking && session && !session.paused ? session.secondsToLevel : null);
  const clockAt = $derived(formatClockIn(toLevel));

  /** A session's length as a clock: `24:10`, `1:02:45`. */
  function clock(seconds: number): string {
    const total = Math.max(0, Math.floor(seconds));
    const h = Math.floor(total / 3600);
    const m = Math.floor((total % 3600) / 60);
    const s = String(total % 60).padStart(2, "0");
    return h ? `${h}:${String(m).padStart(2, "0")}:${s}` : `${m}:${s}`;
  }
</script>

<section class="bar" class:washed={hue != null} style:--region-hue={hue} aria-label="Your character and this session">
  <div class="cell ident">
    {#snippet pictured(inner: number)}{#if look}<LookPicture {look} size={inner} crop="bust" />{/if}{/snippet}
    <span class="ring" class:stale={who.level.at != null} use:tip={lookTip}
      ><LevelRing {level} {percent} size={46} picture={look ? pictured : undefined}><span class="num lv-none">?</span></LevelRing></span
    >
    <div class="who">
      <div class="name-row">
        <b class="name">{name ?? "Your character"}</b>
        <!-- On the website, the Share button on the top row says this, so it isn't said twice. -->
        {#if !can.readWindow}<span class="status {status.tone}" use:tip={status.tip}><span class="dot" aria-hidden="true"></span>{status.text}</span>{/if}
      </div>
      <div class="sub" class:stale={who.seenAt != null}>
        {#if level != null}Lv {level}{job ? ` ${job}` : ""}{:else}Level not read yet{/if}
      </div>
      <!-- The map on a line of its own, whole: it wraps rather than being cut off. -->
      {#if mapName}
        <div class="where" class:stale={who.seenAt != null}>
          <Ref kind="map" name={mapName} />{#if who.onScreen && place.region}<span class="party">{` · ${place.region}`}</span>{/if}{#if party}<span class="party">{` · ${party}`}</span>{/if}
        </div>
      {/if}
      {#if who.seenAt != null}<div class="seen">last seen {ago(who.seenAt)}</div>{/if}
    </div>
  </div>

  <div class="cell level">
    <div class="level-row" class:stale={who.percent.at != null}>
      <span class="num pct" class:dim={!reading}>{percent != null ? `${percent.toFixed(percent >= 100 ? 0 : 1)}%` : "—"}</span>
      {#if points != null}
        <span class="points num">{formatNumber(points)}{session?.expNeeded ? ` / ${formatNumber(session.expNeeded)}` : ""}</span>
      {/if}
    </div>
    <div class="track" aria-hidden="true"><span class:dim={!reading} style:width="{Math.max(0, Math.min(100, percent ?? 0))}%"></span></div>
    {#if toLevel != null && level != null}
      <span class="when" use:tip={clockAt ? `About ${clockAt} at this pace` : null}>
        Lv {level + 1} in <b>{formatDuration(toLevel)}</b>{#if clockAt}<span class="at">{" · "}about {clockAt}</span>{/if}
      </span>
    {/if}
  </div>

  <div class="cell">
    <span class="label">{percentOnly ? "Level / hr" : "EXP / hr"}{session?.measuring ? " · first estimate" : ""}</span>
    <div class="rate-row">
      <span class="num fig exp" class:early={session?.measuring}>{rate}</span>
      {#if tracking && session?.trend.length}
        <span class="spark"><Sparkline values={session.trend} color="var(--exp-text)" height={28} label="EXP an hour over the last minutes" /></span>
      {/if}
    </div>
    <span class="sub" class:up={pace != null && pace > 0} class:down={pace != null && pace < 0}>
      {#if !tracking}
        Starts with your first EXP
      {:else if pace == null || pace === 0}
        {pace === 0 ? "Steady over the last 5 min" : "Whole session"}
      {:else}
        {pace > 0 ? "▲" : "▼"} {Math.abs(pace)}% in the last 5 min
      {/if}
    </span>
  </div>

  <div class="cell">
    <span class="label">Mesos / hr</span>
    <span class="num fig mesos" class:thin={coverage?.thin}>{tracking ? big(session?.mesosPerHour) : "—"}</span>
    <span class="sub">
      {#if tracking && coverage?.partial}Counted {coverage.label}{:else if tracking && session?.mesosGained}{session.mesosGained > 0 ? "+" : ""}{formatNumber(session.mesosGained)} so far{:else}&nbsp;{/if}
    </span>
  </div>

  <div class="cell">
    <span class="label">Session{session?.held ? " · paused" : session?.paused && tracking ? " · idle" : ""}</span>
    <span class="num fig" class:held={session?.held}>{session && tracking ? clock(session.activeSeconds) : "0:00"}</span>
    <SessionControls compact />
  </div>
</section>

<style>
  /* Remembered from an earlier read, not read now. */
  .stale {
    opacity: 0.6;
  }

  .ring {
    display: block;
    flex: none;
  }

  .seen {
    color: var(--muted);
    font-size: var(--text-xs);
  }

  .bar {
    position: relative;
    isolation: isolate;
    display: grid;
    grid-template-columns: minmax(0, 1.5fr) minmax(0, 1.4fr) minmax(0, 1.2fr) minmax(0, 1fr) minmax(0, 1fr);
    align-items: center;
    padding: var(--space-3) var(--space-1);
    border-bottom: 1px solid var(--border);
    background: var(--surface);
  }

  /*
   * The region you're in, as a soft glow from the bar's left end. Its hue comes from the region's
   * name; it fades to the next when you travel.
   */
  .bar::before {
    content: "";
    position: absolute;
    inset: 0;
    z-index: -1;
    background: radial-gradient(70% 260% at 0% 0%, hsl(var(--region-hue) 62% 52% / var(--wash, 0.2)), transparent 72%);
    opacity: 0;
    transition:
      opacity 0.8s ease,
      --region-hue 1.2s ease;
    pointer-events: none;
  }

  .bar.washed::before {
    opacity: 1;
  }

  :global([data-theme$="light"]) .bar::before {
    --wash: 0.16;
  }

  /* The ring is drawn for the dark header; here it sits on the page's own surface. */
  .ident {
    --header-text: var(--text);
    --header-faint: var(--faint);
    --header-track: var(--track);
  }

  .cell {
    display: flex;
    flex-direction: column;
    gap: var(--space-0-5);
    min-width: 0;
    padding: var(--space-0-5) var(--space-4);
    border-left: 1px solid var(--border);
  }

  .cell:first-child {
    border-left: 0;
  }

  .ident {
    flex-direction: row;
    align-items: center;
    gap: var(--space-3);
  }

  .who {
    min-width: 0;
  }

  .name-row {
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
  }

  .name {
    min-width: 0;
    overflow: hidden;
    font-family: var(--font-display);
    font-size: var(--text-lg);
    font-weight: var(--weight-strong);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .sub {
    overflow: hidden;
    color: var(--muted);
    font-size: var(--text-xs);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .where {
    font-size: var(--text-xs);
    line-height: 1.3;
    overflow-wrap: anywhere;
  }

  .party {
    color: var(--muted);
  }

  .label {
    color: var(--faint);
    font-size: var(--text-xs);
  }

  .num {
    font-family: var(--font-display);
    font-variant-numeric: tabular-nums;
    letter-spacing: -0.01em;
  }

  .lv-none {
    color: var(--faint);
    font-weight: var(--weight-strong);
  }

  .status {
    display: inline-flex;
    flex: none;
    align-items: center;
    gap: var(--space-1);
    color: var(--faint);
    font-size: var(--text-xs);
  }

  .status.good {
    color: var(--good);
  }

  .status.warn {
    color: var(--warn);
  }

  .status.bad {
    color: var(--bad);
  }

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: currentColor;
  }

  .status.good .dot {
    animation: pulse 1.6s ease-in-out infinite;
  }

  @keyframes pulse {
    50% {
      opacity: 0.35;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .status.good .dot {
      animation: none;
    }
  }

  .level-row {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--space-2);
    min-width: 0;
  }

  .pct {
    color: var(--exp-text);
    font-size: var(--text-hero);
    font-weight: var(--weight-strong);
    line-height: 1.1;
  }

  .pct.dim {
    color: var(--muted);
  }

  .points {
    overflow: hidden;
    color: var(--faint);
    font-size: var(--text-xs);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .when {
    overflow: hidden;
    color: var(--muted);
    font-size: var(--text-xs);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .when b {
    color: var(--text);
    font-weight: var(--weight-strong);
  }

  .at {
    color: var(--faint);
  }

  .track {
    height: 6px;
    margin-top: var(--space-1);
    border-radius: var(--radius-pill);
    background: var(--track);
    overflow: hidden;
  }

  .track span {
    display: block;
    height: 100%;
    border-radius: var(--radius-pill);
    background: var(--exp);
    transition: width 0.6s var(--ease);
  }

  .track span.dim {
    background: var(--faint);
  }

  .rate-row {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    min-width: 0;
  }

  .fig {
    font-size: var(--text-hero);
    font-weight: var(--weight-strong);
    line-height: 1.1;
    white-space: nowrap;
  }

  .fig.early {
    opacity: 0.7;
  }

  .exp {
    color: var(--exp-text);
  }

  .mesos {
    color: var(--mesos);
  }

  .mesos.thin {
    opacity: 0.55;
  }

  .held {
    color: var(--warn);
  }

  .spark {
    flex: 1;
    min-width: 48px;
    max-width: 130px;
  }

  .sub.up {
    color: var(--good);
  }

  .sub.down {
    color: var(--warn);
  }

  /* A narrower desk window: who you are and the level on one row, the three figures under them. */
  @container desk-main (max-width: 1100px) {
    .bar {
      grid-template-columns: repeat(6, minmax(0, 1fr));
      gap: var(--space-3) 0;
    }

    .ident,
    .level {
      grid-column: span 3;
    }

    .cell:not(.ident):not(.level) {
      grid-column: span 2;
    }

    .cell:nth-child(3) {
      border-left: 0;
    }
  }

  @container desk-main (max-width: 620px) {
    .ident,
    .level,
    .cell:not(.ident):not(.level) {
      grid-column: 1 / -1;
      border-left: 0;
    }
  }
</style>
