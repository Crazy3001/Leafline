<script lang="ts">
  // The character at a glance, for a pinned window: level, EXP and the session's rates.
  import { formatCompact, formatDuration, formatPercent } from "$lib/format";
  import type { HudReading, SessionSummary } from "$lib/types";

  let { hud, session }: { hud: HudReading | null; session: SessionSummary | null } = $props();

  const percent = $derived(hud?.exp?.percent ?? hud?.expPercent ?? null);
</script>

<div class="strip">
  {#if !hud}<p class="paused">The HUD isn't in view, so level and EXP aren't read.</p>{/if}
  <div class="who">
    <span class="level tabular"><span class="lv">LV</span>{hud?.level ?? session?.level ?? "—"}</span>
    <span class="name">{hud?.name ?? "Unknown character"}</span>
    {#if hud?.job}<span class="job">{hud.job}</span>{/if}
  </div>

  <div class="exp" title="EXP toward the next level">
    <div class="track"><div class="fill" style:width="{Math.min(100, percent ?? 0)}%"></div></div>
    <span class="pct tabular">{formatPercent(percent)}</span>
  </div>

  <dl class="rates">
    {#if session?.percentOnly}
      <div title="Percent of a level per hour of active time">
        <dt>% / hr</dt>
        <dd class="tabular">{session.percentPerHour?.toFixed(1) ?? "—"}%</dd>
      </div>
    {:else}
      <div title="EXP gained per hour of active time; the last five minutes in brackets">
        <dt>EXP / hr</dt>
        <dd class="tabular">{formatCompact(session?.expPerHour)}</dd>
      </div>
    {/if}
    <div title="At the current EXP rate">
      <dt>To level</dt>
      <dd class="tabular">{formatDuration(session?.secondsToLevel)}</dd>
    </div>
    <div title="Mesos gained per hour of active time">
      <dt>Mesos / hr</dt>
      <dd class="tabular">{formatCompact(session?.mesosPerHour)}</dd>
    </div>
  </dl>
  {#if session?.held}
    <p class="paused">Paused</p>
  {:else if session?.paused}
    <p class="paused">Paused: waiting for the next EXP gain</p>
  {/if}
</div>

<style>
  .strip {
    display: grid;
    gap: 8px;
  }

  .who {
    display: flex;
    align-items: baseline;
    gap: 8px;
    min-width: 0;
  }

  .level {
    display: inline-flex;
    align-items: baseline;
    gap: 3px;
    padding: 1px 7px;
    border-radius: var(--radius-sm);
    background: var(--accent-soft);
    color: var(--accent-strong);
    font-family: var(--font-display);
    font-size: 14px;
    font-weight: 700;
  }

  .lv {
    font-size: 8.5px;
    opacity: 0.8;
  }

  .name {
    overflow: hidden;
    font-family: var(--font-display);
    font-size: 14px;
    font-weight: 600;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .job {
    margin-left: auto;
    color: var(--faint);
    font-size: 11px;
    white-space: nowrap;
  }

  .exp {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .track {
    flex: 1;
    height: 6px;
    border-radius: var(--radius-sm);
    background: rgba(255, 255, 255, 0.07);
    overflow: hidden;
  }

  .fill {
    height: 100%;
    border-radius: inherit;
    background: var(--exp);
  }

  .pct {
    color: var(--exp);
    font-size: 12px;
    font-weight: 600;
  }

  .rates {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 4px;
    margin: 0;
  }

  .rates div {
    padding: 4px 7px;
    border-radius: var(--radius-sm);
    background: rgba(255, 255, 255, 0.04);
  }

  dt {
    font-size: 10px;
    color: var(--faint);
  }

  dd {
    margin: 0;
    font-size: 13px;
    font-weight: 600;
  }

  .paused {
    margin: 0;
    color: var(--faint);
    font-size: 10.5px;
  }
</style>
