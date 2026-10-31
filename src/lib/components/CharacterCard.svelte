<script lang="ts">
  import Card from "./Card.svelte";
  import Gauge from "./Gauge.svelte";
  import Trend from "./Trend.svelte";
  import { formatCompact, formatDuration, formatNumber, formatPercent } from "$lib/format";
  import type { Gauge as GaugeReading, HudReading, MesoReading, SessionSummary } from "$lib/types";

  let {
    hud,
    mesos,
    session = null,
  }: { hud: HudReading | null; mesos: MesoReading | null; session?: SessionSummary | null } = $props();

  const fraction = (g: GaugeReading | null | undefined) => (g ? g.current / g.max : null);
  const gaugeText = (g: GaugeReading | null | undefined) =>
    g ? `${formatNumber(g.current)} / ${formatNumber(g.max)}` : "not read";

  const expPercent = $derived(hud?.exp?.percent ?? hud?.expPercent ?? null);
  const percentOnly = $derived(session?.percentOnly ?? false);
  /** How the last five minutes compare with the whole session. */
  const pace = $derived.by(() => {
    const recent = percentOnly ? session?.recentPercentPerHour : session?.recentExpPerHour;
    const whole = percentOnly ? session?.percentPerHour : session?.expPerHour;
    if (recent == null || !whole) return null;
    return Math.round((recent / whole - 1) * 100);
  });
</script>

<Card title="Character">
  {#snippet aside()}
    {#if hud?.job}<span>{hud.job}</span>{/if}
  {/snippet}

  <div class="identity">
    <div class="level">
      <span class="lv">LV</span>
      <span class="tabular">{hud?.level ?? session?.level ?? "—"}</span>
    </div>
    <div class="who">
      <div class="name">{hud?.name ?? "Unknown character"}</div>
      <div class="mesos tabular">
        <span class="coin" aria-hidden="true"></span>
        {#if mesos}
          {formatNumber(mesos.mesos)} mesos
        {:else if session?.mesos != null}
          {formatNumber(session.mesos)} mesos
          <span class="seen">(last seen {formatDuration(session.mesosAge)} ago)</span>
        {:else}
          Mesos not visible
        {/if}
      </div>
    </div>
  </div>

  <div class="gauges">
    <Gauge label="HP" color="var(--hp)" fraction={fraction(hud?.hp)} value={gaugeText(hud?.hp)} />
    <Gauge label="MP" color="var(--mp)" fraction={fraction(hud?.mp)} value={gaugeText(hud?.mp)} />
    <Gauge
      label="EXP"
      color="var(--exp)"
      fraction={expPercent != null ? expPercent / 100 : null}
      value={hud?.exp ? `${formatNumber(hud.exp.points)} · ${formatPercent(hud.exp.percent)}` : formatPercent(expPercent)}
    />
  </div>

  <dl class="rates" class:live={session != null}>
    {#if percentOnly}
      <div title="Percent of a level per hour of active time. The HUD shows no EXP amount at this level.">
        <dt>% / hr</dt>
        <dd class="tabular">{session?.percentPerHour?.toFixed(1) ?? "—"}%</dd>
      </div>
    {:else}
      <div title="EXP gained per hour of active time">
        <dt>EXP / hr</dt>
        <dd class="tabular">{formatCompact(session?.expPerHour)}</dd>
        {#if session?.percentPerHour != null}<dd class="sub tabular">{session.percentPerHour.toFixed(1)}%</dd>{/if}
      </div>
    {/if}
    <div title="Mesos gained per hour of active time, leaving out shops, storage and trades">
      <dt>Mesos / hr</dt>
      <dd class="tabular">{formatCompact(session?.mesosPerHour)}</dd>
    </div>
    <div title="At the current EXP rate">
      <dt>To level</dt>
      <dd class="tabular">{formatDuration(session?.secondsToLevel)}</dd>
    </div>
  </dl>
  {#if session?.recentExpPerHour != null || session?.recentPercentPerHour != null}
    <div class="recent" title="Over the last five minutes of active time, against the whole session">
      <span class="recent-label">Last 5 min</span>
      <span class="tabular">
        {percentOnly ? `${session.recentPercentPerHour?.toFixed(1)}%` : formatCompact(session.recentExpPerHour)}
      </span>
      {#if pace != null}<span class="pace tabular" class:up={pace > 0} class:down={pace < 0}>
          {pace > 0 ? "+" : ""}{pace}%
        </span>{/if}
      <span class="tabular muted">{formatCompact(session.recentMesosPerHour)} mesos</span>
      <span class="trend"><Trend points={session.trend} /></span>
    </div>
  {/if}
  {#if !session}
    <p class="hint">Rates appear once a session is tracked.</p>
  {:else if session.expPerHour == null}
    <p class="hint">Rates need at least 30 s of EXP gains with the HUD in view.</p>
  {/if}
</Card>

<style>
  .recent {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 8px;
    font-size: 11.5px;
  }

  .recent-label,
  .muted {
    color: var(--faint);
    white-space: nowrap;
  }

  .pace {
    color: var(--muted);
  }

  .pace.up {
    color: var(--good);
  }

  .pace.down {
    color: var(--warn);
  }

  .trend {
    flex: 1;
    min-width: 40px;
  }

  .identity {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 12px;
  }

  .level {
    display: grid;
    place-items: center;
    width: 48px;
    height: 48px;
    border-radius: var(--radius-sm);
    background: var(--accent-soft);
    border: 1px solid color-mix(in srgb, var(--accent) 45%, transparent);
    font-family: var(--font-display);
    font-size: 20px;
    font-weight: 700;
    line-height: 1;
    color: var(--accent-strong);
  }

  .lv {
    font-size: 9px;
    color: var(--accent);
    opacity: 0.8;
  }

  .name {
    font-family: var(--font-display);
    font-size: 16px;
    font-weight: 600;
  }

  .mesos {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--muted);
    font-size: 12px;
  }

  .seen {
    color: var(--faint);
  }

  .coin {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: var(--accent);
  }

  .gauges {
    display: grid;
    gap: 8px;
  }

  .rates {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 6px;
    margin: 12px 0 0;
  }

  .rates div {
    padding: 6px 8px;
    border-radius: var(--radius-sm);
    background: rgba(255, 255, 255, 0.03);
  }

  dt {
    font-size: 10.5px;
    color: var(--faint);
  }

  dd {
    margin: 0;
    font-weight: 600;
    color: var(--muted);
  }

  .live dd {
    color: var(--text);
  }

  dd.sub {
    display: inline;
    margin-left: 5px;
    font-size: 11px;
    font-weight: 400;
    color: var(--muted);
  }

  .live dt + dd {
    display: inline;
  }

  .hint {
    margin: 8px 0 0;
    color: var(--faint);
    font-size: 11.5px;
  }
</style>
