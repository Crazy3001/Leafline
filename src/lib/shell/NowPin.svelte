<script lang="ts">
  // The Now tab pinned into a window of its own, to glance at mid-fight: where and how long, the
  // EXP rate big with its line, the other rates in a row, and the tracked quest's counters. The
  // pinned window's header names it and opens the tab for the rest.
  import { formatCompact, formatDuration, formatNumber, objectiveProgress, partyLabel } from "$lib/format";
  import { mesosCoverage } from "$lib/training";
  import type { Analysis, SessionSummary } from "$lib/types";
  import Sparkline from "$lib/ui/Sparkline.svelte";

  let { analysis, session }: { analysis: Analysis | null; session: SessionSummary | null } = $props();

  const percentOnly = $derived(session?.percentOnly ?? false);
  const tracking = $derived(session != null && session.frames > 0);
  const where = $derived([partyLabel(session?.party), session?.map].filter(Boolean).join(" · "));
  const level = $derived(analysis?.hud?.level ?? session?.level ?? null);

  /** A session's length as a clock: `24:10`, `1:02:45`. */
  function clock(seconds: number): string {
    const total = Math.max(0, Math.floor(seconds));
    const h = Math.floor(total / 3600);
    const m = Math.floor((total % 3600) / 60);
    const s = String(total % 60).padStart(2, "0");
    return h ? `${h}:${String(m).padStart(2, "0")}:${s}` : `${m}:${s}`;
  }

  const big = (value: number | null | undefined) =>
    value == null ? "—" : Math.abs(value) >= 1e6 ? formatCompact(value) : formatNumber(Math.round(value));

  const expRate = $derived.by(() => {
    if (!session) return "—";
    if (percentOnly) return session.percentPerHour != null ? `${session.percentPerHour.toFixed(1)}%` : "—";
    return big(session.expPerHour);
  });
  const coverage = $derived(session ? mesosCoverage(session) : null);

  const quests = $derived((analysis?.quest?.quests ?? []).filter((q) => q.objectives.length));
  const quest = $derived(quests[0] ?? null);
</script>

<div class="pin">
  <div class="head">
    <span class="where">{where || "This session"}</span>
    {#if session}<span class="num clock" class:paused={session.paused}>{session.paused ? "Paused · " : ""}{clock(session.activeSeconds)}</span>{/if}
  </div>

  {#if tracking && session}
    <div class="hero">
      <div>
        <div class="num rate" class:early={session.measuring}>{expRate}</div>
        <div class="label">{percentOnly ? "Level per hour" : "EXP per hour"}{session.measuring ? " · first estimate" : ""}</div>
      </div>
      <div class="line"><Sparkline values={session.trend} color="var(--exp-text)" height={36} /></div>
    </div>
    <div class="row">
      <span><b class="num mesos" class:thin={coverage?.thin}>{big(session.mesosPerHour)}</b> <span class="label">mesos/hr</span></span>
      {#if session.killsPerHour != null}<span><b class="num">{formatCompact(session.killsPerHour)}</b> <span class="label">kills/hr</span></span>{/if}
      {#if session.secondsToLevel != null && level != null}
        <span class="end"><b class="num">{formatDuration(session.secondsToLevel)}</b> <span class="label">to Lv {level + 1}</span></span>
      {/if}
    </div>
  {:else}
    <p class="quiet">Rates start once the EXP bar has moved.</p>
  {/if}

  {#if quest}
    <div class="quest">
      <div class="head q">
        <span class="where strong">{quest.title}</span>
        {#if quests.length > 1}<span class="label">1 of {quests.length}</span>{/if}
      </div>
      {#each quest.objectives as o (o.target)}
        {@const done = !o.unread && o.current >= o.required}
        <div class="objective">
          <span class="target">{o.target}</span>
          {#if !o.unread}<span class="num count" class:done>{o.current} <span class="of">/ {o.required}</span></span>{/if}
        </div>
        <div class="bar"><span class:done style:width="{o.unread ? 0 : objectiveProgress(o) * 100}%"></span></div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .pin {
    display: grid;
    gap: var(--space-2);
    font-size: var(--text-md);
  }

  .head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--space-2);
    color: var(--muted);
    font-size: var(--text-xs);
    font-weight: var(--weight-strong);
  }

  .where {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .strong {
    color: var(--text);
    font-size: var(--text-md);
    font-weight: var(--weight-strong);
  }

  .clock {
    flex: none;
    color: var(--muted);
  }

  .clock.paused {
    color: var(--warn);
  }

  .hero {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: var(--space-3);
  }

  .rate {
    font-size: var(--text-hero);
    font-weight: var(--weight-strong);
    line-height: 32px;
    white-space: nowrap;
  }

  .rate.early {
    opacity: 0.7;
  }

  .line {
    flex: 0 1 130px;
    min-width: 60px;
  }

  .label {
    color: var(--faint);
    font-size: var(--text-xs);
    font-weight: var(--weight-body);
  }

  .row {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1) var(--space-3);
  }

  .row b {
    font-weight: var(--weight-strong);
  }

  .mesos {
    color: var(--mesos);
  }

  .mesos.thin {
    opacity: 0.55;
  }

  .end {
    margin-left: auto;
  }

  .quiet {
    margin: 0;
    color: var(--faint);
  }

  .quest {
    display: grid;
    padding-top: var(--space-2);
    border-top: 1px solid var(--border);
  }

  .head.q {
    margin-bottom: var(--space-1-5);
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
    height: 5px;
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
  }

  .bar span.done {
    background: var(--good);
  }
</style>
