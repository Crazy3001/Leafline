<script lang="ts">
  import { can } from "$lib/platform";
  import type { Snippet } from "svelte";

  import { ChevronRight } from "@lucide/svelte";

  import Card from "./Card.svelte";
  import Gauge from "./Gauge.svelte";
  import { openPanel } from "$lib/nav.svelte";
  import { ago, lastKnown, profile, watchProfile } from "$lib/profile.svelte";
  import { formatCompact, formatDuration, formatNumber, formatPercent, splitMapLabel } from "$lib/format";
  import type { Gauge as GaugeReading, HudReading, MesoReading, SessionSummary } from "$lib/types";

  let {
    hud,
    mesos,
    session = null,
    remember = false,
    plain = false,
    linkTrain = false,
    trim = false,
    bare = false,
    children,
  }: {
    hud: HudReading | null;
    mesos: MesoReading | null;
    session?: SessionSummary | null;
    /** Show what was read of the character on earlier runs (faded, dated) where nothing is read
     *  now, and follow the character picked in the character picker. Off for replays. */
    remember?: boolean;
    plain?: boolean;
    /** Link the session's rate to the Train tab, which has the rest. Off where there is no tab. */
    linkTrain?: boolean;
    /** Under the main window's header, which already shows the level, name, job, EXP and time to
     *  level: only the mesos, HP and MP, and what follows as a card of its own. */
    trim?: boolean;
    /** No card around the content, which brings its own (the desk view's wide build). */
    bare?: boolean;
    /** More about the character under the rates, in the same card: the overlay puts the build plan here. */
    children?: Snippet;
  } = $props();

  watchProfile();

  /** The readings are this character's: no other one picked, or a replay. */
  const onScreen = $derived(!remember || profile.live);
  const read = $derived(onScreen ? hud : null);
  const now = $derived(onScreen ? session : null);
  /** What was read of them on earlier runs. Never used for the rates below. */
  const known = $derived(remember ? profile.known : null);

  /** The name read this run: the tracker's settled one, else the HUD's latest. */
  const readName = $derived(now?.name ?? read?.name ?? null);
  const displayName = $derived(readName ?? (remember ? profile.name : null) ?? known?.name ?? null);
  const level = $derived(lastKnown(read?.level ?? now?.level, known?.level, known?.seenAt, onScreen));
  const job = $derived(lastKnown(read?.job ?? now?.job, known?.job, known?.seenAt, onScreen));
  const expPercent = $derived(
    lastKnown(read?.exp?.percent ?? read?.expPercent ?? now?.percent, known?.expPercent, known?.seenAt, onScreen),
  );
  const coins = $derived(lastKnown(mesos?.mesos ?? now?.mesos, known?.mesos, known?.mesosSeenAt, onScreen));
  /** When the level and the rest were read, when they weren't read now. */
  const seenAt = $derived(level.at ?? expPercent.at);
  /** The map they were last on, while they aren't being read. */
  const lastMap = $derived(seenAt != null && known?.map ? splitMapLabel(known.map).name : null);

  const fraction = (g: GaugeReading | null | undefined) => (g ? g.current / g.max : null);
  const gaugeText = (g: GaugeReading | null | undefined) =>
    g ? `${formatNumber(g.current)} / ${formatNumber(g.max)}` : "not read";

  const percentOnly = $derived(now?.percentOnly ?? false);
  /** The session's rate in a line: EXP an hour, or percent at levels where the HUD shows only that. */
  const rateLine = $derived.by(() => {
    if (!now) return null;
    if (percentOnly) return now.percentPerHour != null ? `${now.percentPerHour.toFixed(1)}% / hr` : null;
    return now.expPerHour != null ? `${formatCompact(now.expPerHour)} EXP / hr` : null;
  });
  const expText = $derived.by(() => {
    if (expPercent.at != null) return `${formatPercent(expPercent.value)} · ${ago(expPercent.at)}`;
    return read?.exp ? `${formatNumber(read.exp.points)} · ${formatPercent(read.exp.percent)}` : formatPercent(expPercent.value);
  });
</script>

{#snippet purse()}
  <div class="mesos tabular">
    <span class="coin" aria-hidden="true"></span>
    {#if mesos && onScreen}
      {formatNumber(mesos.mesos)} mesos
    {:else if now?.mesos != null}
      {formatNumber(now.mesos)} mesos
      <span class="seen">({formatDuration(now.mesosAge)} ago)</span>
    {:else if coins.value != null && coins.at != null}
      <span class="stale">{formatNumber(coins.value)} mesos</span>
      <span class="seen">(as of {ago(coins.at)})</span>
    {:else}
      Mesos not visible
    {/if}
  </div>
{/snippet}

{#if trim}
  <!--
    The header above has who you are and the level; the game shows HP and MP. Here: the mesos you
    carry (read from the HUD, which the website doesn't, or remembered from an earlier read), then
    the build and stats.
  -->
  {#if children || can.liveReading || coins.at != null}<section class="more" class:card={!bare} class:bare>
    {#if can.liveReading || coins.at != null}<div class="purse-line">{@render purse()}</div>{/if}
    {#if children}{@render children()}{/if}
  </section>{/if}
{:else}
<Card title="Character" {plain}>
  {#snippet aside()}
    {#if job.value}<span class:stale={job.at != null}>{job.value}</span>{/if}
  {/snippet}

  <div class="identity">
    <div class="level" class:stale={level.at != null}>
      <span class="lv">Lv</span>
      <span class="tabular">{level.value ?? "—"}</span>
    </div>
    <div class="who">
      <div class="name">
        {displayName ?? "Unknown character"}
        {#if seenAt != null}<span class="seen">last seen {ago(seenAt)}</span>{/if}
      </div>
      {@render purse()}
      {#if lastMap}<div class="last-map">Last on <span class="stale">{lastMap}</span></div>{/if}
    </div>
  </div>

  <div class="gauges">
    <Gauge label="HP" color="var(--hp)" fraction={fraction(read?.hp)} value={gaugeText(read?.hp)} />
    <Gauge label="MP" color="var(--mp)" fraction={fraction(read?.mp)} value={gaugeText(read?.mp)} />
    <div class:stale={expPercent.at != null}>
      <Gauge
        label="EXP"
        color="var(--exp)"
        fraction={expPercent.value != null ? expPercent.value / 100 : null}
        value={expText}
      />
    </div>
  </div>

  {#if rateLine}
    {#if linkTrain}
      <button class="to-train" onclick={() => openPanel("train")}>
        <span>
          <span class="tabular strong">{rateLine}</span>
          {#if now?.secondsToLevel != null && read?.level != null}<span class="muted">{" · "}Lv {read.level + 1} in {formatDuration(now.secondsToLevel)}</span>{/if}
        </span>
        <span class="muted go">Train <ChevronRight size={12} /></span>
      </button>
    {:else}
      <p class="rate-line tabular">
        {rateLine}{#if now?.secondsToLevel != null}<span class="muted">{" · "}{formatDuration(now.secondsToLevel)} to level</span>{/if}
      </p>
    {/if}
  {/if}
  {@render children?.()}
</Card>
{/if}

<style>
  .to-train {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    width: 100%;
    margin-top: var(--space-3);
    padding: var(--space-2) var(--space-3);
    border: 0;
    background: var(--surface-2);
    color: var(--text);
    font-size: var(--text-md);
    text-align: left;
    cursor: pointer;
  }

  .to-train:hover {
    background: var(--surface-3);
  }

  .to-train:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }

  .go {
    display: inline-flex;
    flex: none;
    align-items: center;
    gap: var(--space-0-5);
  }

  .strong {
    font-weight: var(--weight-strong);
  }

  .muted {
    color: var(--muted);
  }

  .rate-line {
    margin: var(--space-3) 0 0;
    font-size: var(--text-md);
  }

  .identity {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    margin-bottom: var(--space-3);
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
    font-size: var(--text-xl);
    font-weight: var(--weight-strong);
    line-height: 1;
    color: var(--text);
  }

  .lv {
    font-size: var(--text-xs);
    font-weight: var(--weight-body);
    color: var(--muted);
  }

  .name {
    font-family: var(--font-display);
    font-size: var(--text-lg);
    font-weight: var(--weight-strong);
  }

  .mesos {
    display: flex;
    align-items: center;
    gap: var(--space-1-5);
    color: var(--muted);
    font-size: var(--text-md);
  }

  .seen {
    color: var(--faint);
  }

  /* Remembered from an earlier read, not read now. */
  .stale {
    opacity: 0.6;
  }

  .last-map {
    margin-top: 2px;
    color: var(--muted);
    font-size: var(--text-xs);
  }

  .name .seen {
    margin-left: var(--space-1-5);
    font-family: var(--font-sans);
    font-size: var(--text-xs);
    font-weight: var(--weight-body);
  }

  .coin {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: var(--accent);
  }

  .gauges {
    display: grid;
    gap: var(--space-2);
  }

  /* Under the window's header: the frame is the shared .card. */
  .more {
    margin: 0 var(--space-3) var(--space-2);
    padding: var(--space-3) var(--space-4) var(--space-3);
  }

  .more.bare {
    margin: 0;
    padding: 0;
  }

  .purse-line {
    margin-bottom: var(--space-3);
  }
</style>
