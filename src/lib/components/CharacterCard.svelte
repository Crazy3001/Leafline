<script lang="ts">
  import Card from "./Card.svelte";
  import Gauge from "./Gauge.svelte";
  import { formatNumber, formatPercent } from "$lib/format";
  import type { Gauge as GaugeReading, HudReading, MesoReading } from "$lib/types";

  let { hud, mesos }: { hud: HudReading | null; mesos: MesoReading | null } = $props();

  const fraction = (g: GaugeReading | null | undefined) => (g ? g.current / g.max : null);
  const gaugeText = (g: GaugeReading | null | undefined) =>
    g ? `${formatNumber(g.current)} / ${formatNumber(g.max)}` : "not read";
</script>

<Card title="Character">
  {#snippet aside()}
    {#if hud?.job}<span>{hud.job}</span>{/if}
  {/snippet}

  <div class="identity">
    <div class="level">
      <span class="lv">LV</span>
      <span class="tabular">{hud?.level ?? "—"}</span>
    </div>
    <div class="who">
      <div class="name">{hud?.name ?? "Unknown character"}</div>
      <div class="mesos tabular">
        <span class="coin" aria-hidden="true"></span>
        {mesos ? `${formatNumber(mesos.mesos)} mesos` : "Mesos not visible"}
      </div>
    </div>
  </div>

  <div class="gauges">
    <Gauge label="HP" color="var(--hp)" fraction={fraction(hud?.hp)} value={gaugeText(hud?.hp)} />
    <Gauge label="MP" color="var(--mp)" fraction={fraction(hud?.mp)} value={gaugeText(hud?.mp)} />
    <Gauge
      label="EXP"
      color="var(--exp)"
      fraction={hud?.exp ? hud.exp.percent / 100 : null}
      value={hud?.exp ? `${formatNumber(hud.exp.points)} · ${formatPercent(hud.exp.percent)}` : "not read"}
    />
  </div>

  {#if hud?.needsCalibration}
    <p class="calibrate">Teach Leafline your HUD in the workbench to read HP, MP and EXP.</p>
  {/if}

  <dl class="rates">
    <div><dt>EXP / hr</dt><dd>—</dd></div>
    <div><dt>Mesos / hr</dt><dd>—</dd></div>
    <div><dt>To level</dt><dd>—</dd></div>
  </dl>
</Card>

<style>
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
    border-radius: 12px;
    background: linear-gradient(160deg, rgba(243, 177, 63, 0.22), rgba(243, 177, 63, 0.06));
    border: 1px solid rgba(243, 177, 63, 0.3);
    font-family: var(--font-display);
    font-size: 20px;
    font-weight: 700;
    line-height: 1;
    color: var(--accent-strong);
  }

  .lv {
    font-size: 9px;
    letter-spacing: 0.12em;
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

  .coin {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: radial-gradient(circle at 35% 35%, #ffe08a, #d79a1e);
  }

  .gauges {
    display: grid;
    gap: 8px;
  }

  .calibrate {
    margin: 10px 0 0;
    padding: 7px 9px;
    border-radius: var(--radius-sm);
    background: var(--accent-soft);
    color: var(--accent-strong);
    font-size: 12px;
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
</style>
