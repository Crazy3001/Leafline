<script lang="ts">
  // The desk view's character card on Now: who you are, how far through the level and when the
  // next comes, and the session's two rates and clock. On Now this is the one place those numbers
  // are; the player bar steps aside for it.
  import { tip } from "$lib/tip.svelte";
  import { Clock3, Coins, TrendingUp, UserRound } from "@lucide/svelte";

  import SessionControls from "$lib/components/SessionControls.svelte";
  import { formatClockIn, formatCompact, formatDuration, formatNumber } from "$lib/format";
  import { live } from "$lib/readings.svelte";
  import { mesosCoverage } from "$lib/training";
  import LeafSprite from "$lib/ui/LeafSprite.svelte";
  import TitledCard from "$lib/ui/TitledCard.svelte";

  let { ongo }: { ongo: () => void } = $props();

  const hud = $derived(live.analysis?.hud ?? null);
  const session = $derived(live.session);
  const tracking = $derived(session != null && session.activeSeconds > 0);
  const percentOnly = $derived(session?.percentOnly ?? false);
  const level = $derived(hud?.level ?? session?.level ?? null);
  const percent = $derived(hud?.exp?.percent ?? hud?.expPercent ?? session?.percent ?? null);
  const name = $derived(hud?.name ?? session?.name ?? null);
  const job = $derived(hud?.job ?? session?.job ?? null);
  const toLevel = $derived(tracking && session && !session.paused ? session.secondsToLevel : null);
  const at = $derived(formatClockIn(toLevel));

  const big = (value: number | null | undefined) =>
    value == null ? "—" : Math.abs(value) >= 1e5 ? formatCompact(value) : formatNumber(Math.round(value));
  const rate = $derived.by(() => {
    if (!tracking || !session) return "—";
    if (percentOnly) return session.percentPerHour != null ? `${session.percentPerHour.toFixed(1)}%` : "—";
    return big(session.expPerHour);
  });
  const recent = $derived(percentOnly ? session?.recentPercentPerHour : session?.recentExpPerHour);
  const whole = $derived(percentOnly ? session?.percentPerHour : session?.expPerHour);
  const pace = $derived(recent != null && whole ? Math.round((recent / whole - 1) * 100) : null);
  const coverage = $derived(session ? mesosCoverage(session) : null);

  /** A session's length as a clock: `24:10`, `1:02:45`. */
  function clock(seconds: number): string {
    const total = Math.max(0, Math.floor(seconds));
    const h = Math.floor(total / 3600);
    const m = Math.floor((total % 3600) / 60);
    const s = String(total % 60).padStart(2, "0");
    return h ? `${h}:${String(m).padStart(2, "0")}:${s}` : `${m}:${s}`;
  }
</script>

<TitledCard title="Your character" icon={UserRound} tone="gold" more="Me" onmore={ongo}>
  <div class="who">
    <div class="avatar">
      <LeafSprite size={84} />
      {#if level != null}<span class="lv">Lv {level}</span>{/if}
    </div>
    <div class="info">
      <div class="name">{name ?? "Your character"}</div>
      <div class="job">{job ?? (level == null ? "Level not read yet" : "")}</div>
      <div class="exp">
        <span class="pct">{percent != null ? `${percent.toFixed(percent >= 100 ? 0 : 1)}%` : "—"}</span>
        {#if toLevel != null && level != null}
          <span class="eta" use:tip={at ? `About ${at} at this pace` : null}>Lv {level + 1} in <b>{formatDuration(toLevel)}</b></span>
        {/if}
      </div>
      <div class="track" aria-hidden="true"><span style:width="{Math.max(0, Math.min(100, percent ?? 0))}%"></span></div>
    </div>
  </div>

  <div class="tiles">
    <div class="tile">
      <span class="ic gold" aria-hidden="true"><TrendingUp size={18} /></span>
      <div>
        <div class="l">{percentOnly ? "Level / hr" : "EXP / hr"}</div>
        <div class="v">{rate}</div>
        {#if tracking}
          <div class="d" class:up={pace != null && pace > 0} class:down={pace != null && pace < 0}>
            {pace == null || session?.measuring ? "first estimate" : pace === 0 ? "steady" : `${pace > 0 ? "▲" : "▼"} ${Math.abs(pace)}% last 5 min`}
          </div>
        {/if}
      </div>
    </div>
    <div class="tile">
      <span class="ic orange" aria-hidden="true"><Coins size={18} /></span>
      <div>
        <div class="l">Mesos / hr</div>
        <div class="v ms" class:thin={coverage?.thin}>{tracking ? big(session?.mesosPerHour) : "—"}</div>
        {#if tracking}<div class="d plain">{coverage?.partial ? `counted ${coverage.label}` : session?.mesosGained ? `${session.mesosGained > 0 ? "+" : ""}${formatNumber(session.mesosGained)} so far` : ""}</div>{/if}
      </div>
    </div>
    <div class="tile">
      <span class="ic blue" aria-hidden="true"><Clock3 size={18} /></span>
      <div>
        <div class="l">Session{session?.held ? " · paused" : ""}</div>
        <div class="v" class:held={session?.held}>{session && tracking ? clock(session.activeSeconds) : "0:00"}</div>
        <div class="controls"><SessionControls /></div>
      </div>
    </div>
  </div>
</TitledCard>

<style>
  .who {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: var(--space-5);
    align-items: center;
  }

  .avatar {
    position: relative;
    display: grid;
    place-items: center;
    width: 112px;
    height: 112px;
    border-radius: var(--radius-lg);
    background: radial-gradient(circle at 50% 35%, color-mix(in srgb, var(--accent) 28%, var(--surface-2)), var(--surface-2) 70%);
  }

  .lv {
    position: absolute;
    bottom: -10px;
    left: 50%;
    padding: var(--space-0-5) var(--space-3);
    border-radius: var(--radius-pill);
    background: var(--exp);
    color: #1c1400;
    font-size: var(--text-xs);
    font-weight: var(--weight-strong);
    white-space: nowrap;
    transform: translateX(-50%);
  }

  .name {
    overflow: hidden;
    font-family: var(--font-display);
    font-size: var(--text-xl);
    font-weight: var(--weight-strong);
    line-height: 1.15;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .job {
    color: var(--muted);
  }

  .exp {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--space-3);
    margin-top: var(--space-2);
  }

  .pct {
    color: var(--exp-text);
    font-family: var(--font-display);
    font-size: var(--text-2xl);
    font-weight: var(--weight-strong);
    font-variant-numeric: tabular-nums;
  }

  .eta {
    color: var(--muted);
  }

  .eta b {
    color: var(--text);
  }

  .track {
    height: 10px;
    margin-top: var(--space-1-5);
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

  .tiles {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: var(--space-3);
    margin-top: var(--space-2);
  }

  .tile {
    display: flex;
    gap: var(--space-3);
    align-items: flex-start;
    min-width: 0;
    padding: var(--space-3);
    border-radius: var(--radius);
    background: var(--surface-2);
  }

  .ic {
    display: grid;
    flex: none;
    place-items: center;
    width: 34px;
    height: 34px;
    border-radius: var(--radius);
  }

  .ic.gold {
    background: color-mix(in srgb, var(--exp) 20%, transparent);
    color: var(--exp-text);
  }

  .ic.orange {
    background: color-mix(in srgb, var(--mesos) 18%, transparent);
    color: var(--mesos);
  }

  .ic.blue {
    background: color-mix(in srgb, var(--ref-item) 18%, transparent);
    color: var(--ref-item);
  }

  .l {
    color: var(--muted);
    font-size: var(--text-xs);
  }

  .v {
    font-family: var(--font-display);
    font-size: var(--text-2xl);
    font-weight: var(--weight-strong);
    font-variant-numeric: tabular-nums;
    line-height: 1.15;
    white-space: nowrap;
  }

  .v.ms {
    color: var(--mesos);
  }

  .v.thin {
    opacity: 0.6;
  }

  .v.held {
    color: var(--warn);
  }

  .d {
    color: var(--muted);
    font-size: var(--text-xs);
  }

  .d.up {
    color: var(--good);
  }

  .d.down {
    color: var(--warn);
  }

  .controls {
    margin-top: var(--space-1-5);
  }

  /* A narrow card: the three figures stack. */
  @container tcard (max-width: 560px) {
    .tiles {
      grid-template-columns: minmax(0, 1fr);
    }
  }
</style>
