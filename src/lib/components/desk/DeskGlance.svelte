<script lang="ts">
  // The desk view's Now strip: the session's numbers large enough to read from your chair. EXP an
  // hour with its recent line, time to the next level, mesos and kills an hour, and the clock.
  import { tip } from "$lib/tip.svelte";

  import SessionControls from "$lib/components/SessionControls.svelte";
  import { formatClockIn, formatCompact, formatDuration, formatNumber } from "$lib/format";
  import { live } from "$lib/readings.svelte";
  import ExpBar from "$lib/ui/ExpBar.svelte";
  import Sparkline from "$lib/ui/Sparkline.svelte";

  const session = $derived(live.session);
  const tracking = $derived(session != null && session.activeSeconds > 0);
  const percentOnly = $derived(session?.percentOnly ?? false);
  const level = $derived(live.analysis?.hud?.level ?? session?.level ?? null);
  const percent = $derived(live.analysis?.hud?.exp?.percent ?? live.analysis?.hud?.expPercent ?? session?.percent ?? null);

  const big = (value: number | null | undefined) =>
    value == null ? "—" : Math.abs(value) >= 1e5 ? formatCompact(value) : formatNumber(Math.round(value));

  const rate = $derived.by(() => {
    if (!session) return "—";
    if (percentOnly) return session.percentPerHour != null ? `${session.percentPerHour.toFixed(1)}%` : "—";
    return big(session.expPerHour);
  });
  const recent = $derived(percentOnly ? session?.recentPercentPerHour : session?.recentExpPerHour);
  const whole = $derived(percentOnly ? session?.percentPerHour : session?.expPerHour);
  /** How the last five minutes compare with the session, in percent. */
  const pace = $derived(recent != null && whole ? Math.round((recent / whole - 1) * 100) : null);
  const recentText = $derived(recent == null ? null : percentOnly ? `${recent.toFixed(1)}%` : big(recent));
  const clockAt = $derived(formatClockIn(session?.secondsToLevel));

  /** A session's length as a clock: `24:10`, `1:02:45`. */
  function clock(seconds: number): string {
    const total = Math.max(0, Math.floor(seconds));
    const h = Math.floor(total / 3600);
    const m = Math.floor((total % 3600) / 60);
    const s = String(total % 60).padStart(2, "0");
    return h ? `${h}:${String(m).padStart(2, "0")}:${s}` : `${m}:${s}`;
  }
</script>

<section class="glance card" aria-label="This session at a glance">
  <div class="tile wide">
    <span class="label">{percentOnly ? "Level per hour" : "EXP / hr"} · this session{session?.measuring ? " · first estimate" : ""}</span>
    <div class="hero-row">
      <span class="num hero exp" class:early={session?.measuring}>{rate}</span>
      {#if tracking && session?.trend.length}
        <span class="spark"><Sparkline values={session.trend} color="var(--exp-text)" height={40} label="EXP an hour over the last minutes" /></span>
      {/if}
    </div>
    {#if recentText}
      <span class="sub" class:up={pace != null && pace > 0} class:down={pace != null && pace < 0}>
        {recentText} in the last 5 min{#if pace != null && pace !== 0}, {pace > 0 ? "▲" : "▼"} {Math.abs(pace)}% against the session{/if}
      </span>
    {:else}
      <span class="sub">{tracking ? "The last 5 minutes show after a few minutes" : "Starts with your first EXP"}</span>
    {/if}
  </div>

  <div class="tile wide">
    <span class="label">{level != null ? `Level ${level} → ${level + 1}` : "Level"}</span>
    <div class="hero-row">
      {#if tracking && session?.secondsToLevel != null}
        <span class="num hero" use:tip={clockAt ? `About ${clockAt}` : ""}>{formatDuration(session.secondsToLevel)}</span>
        {#if clockAt}<span class="sub at">about {clockAt}</span>{/if}
      {:else}
        <span class="num hero quiet">{percent != null ? `${percent.toFixed(1)}%` : "—"}</span>
      {/if}
    </div>
    {#if percent != null}
      <ExpBar {percent} points={session?.exp?.points ?? null} needed={session?.expNeeded ?? null} />
    {/if}
  </div>

  <div class="tile">
    <span class="label">Mesos / hr</span>
    <span class="num mid mesos">{tracking ? big(session?.mesosPerHour) : "—"}</span>
    <span class="sub">{tracking && session?.mesosGained ? `${session.mesosGained > 0 ? "+" : ""}${formatNumber(session.mesosGained)} so far` : " "}</span>
  </div>

  <div class="tile">
    <span class="label">Kills / hr</span>
    <span class="num mid">{tracking && session?.killsPerHour != null ? formatCompact(session.killsPerHour) : "—"}</span>
    <span class="sub">{tracking && session?.kills ? `${formatNumber(session.kills)} on matched maps` : " "}</span>
  </div>

  <div class="tile">
    <span class="label">Session{session?.held ? " · paused" : session?.paused && tracking ? " · idle" : ""}</span>
    <span class="num mid" class:held={session?.held}>{session && tracking ? clock(session.activeSeconds) : "0:00"}</span>
    <div class="controls"><SessionControls /></div>
  </div>
</section>

<style>
  .glance {
    display: grid;
    grid-template-columns: minmax(0, 1.5fr) minmax(0, 1.3fr) repeat(3, minmax(0, 1fr));
    align-items: stretch;
    padding: 16px 6px;
  }

  .tile {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
    padding: 2px 18px;
    border-left: 1px solid var(--border);
  }

  .tile:first-child {
    border-left: 0;
  }

  .label {
    color: var(--faint);
    font-size: var(--text-xs);
    font-weight: 500;
  }

  .hero-row {
    display: flex;
    align-items: flex-end;
    gap: 12px;
    min-width: 0;
  }

  .num {
    font-family: var(--font-display);
    font-variant-numeric: tabular-nums;
    letter-spacing: -0.01em;
  }

  .hero {
    font-size: 42px;
    font-weight: 600;
    line-height: 1.05;
    white-space: nowrap;
  }

  .hero.early {
    opacity: 0.7;
  }

  .hero.quiet {
    color: var(--muted);
  }

  .exp {
    color: var(--exp-text);
  }

  .spark {
    flex: 1;
    min-width: 60px;
    max-width: 170px;
    margin-bottom: 4px;
  }

  .mid {
    font-size: 28px;
    font-weight: 600;
    line-height: 1.15;
    white-space: nowrap;
  }

  .mesos {
    color: var(--mesos);
  }

  .held {
    color: var(--warn);
  }

  .sub {
    color: var(--muted);
    font-size: var(--text-sm);
  }

  .sub.at {
    margin-bottom: 6px;
  }

  .sub.up {
    color: var(--good);
  }

  .sub.down {
    color: var(--warn);
  }

  .controls {
    margin-top: 2px;
  }

  /* A narrower desk window: the big two on a row, the rest under them. */
  @container desk-main (max-width: 900px) {
    .glance {
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 14px 0;
    }

    .tile:nth-child(3) {
      border-left: 0;
    }

    .tile:nth-child(5) {
      border-left: 0;
    }
  }
</style>
