<script lang="ts">
  import { onMount, type Snippet } from "svelte";

  import { ChevronRight } from "@lucide/svelte";

  import Card from "./Card.svelte";
  import Gauge from "./Gauge.svelte";
  import { backend } from "$lib/api";
  import { openPanel } from "$lib/nav.svelte";
  import { formatAgo, formatCompact, formatDuration, formatNumber, formatPercent } from "$lib/format";
  import type { Gauge as GaugeReading, HudReading, KnownMesos, MesoReading, SessionSummary } from "$lib/types";

  let {
    hud,
    mesos,
    session = null,
    remember = false,
    plain = false,
    linkTrain = false,
    children,
  }: {
    hud: HudReading | null;
    mesos: MesoReading | null;
    session?: SessionSummary | null;
    /** Show the mesos remembered from earlier runs when none are on screen. Off for replays. */
    remember?: boolean;
    plain?: boolean;
    /** Link the session's rate to the Train tab, which has the rest. Off where there is no tab. */
    linkTrain?: boolean;
    /** More about the character under the rates, in the same card: the overlay puts the build plan here. */
    children?: Snippet;
  } = $props();

  /** The name read this run: the tracker's settled one, else the HUD's latest. */
  const readName = $derived(session?.name ?? hud?.name ?? null);
  /** The mesos remembered for that character, or with no name read, for the one seen last. */
  let known = $state<KnownMesos | null>(null);
  let now = $state(Date.now() / 1000);
  let asked = 0;

  async function recall(name: string | null) {
    const ask = ++asked;
    try {
      const found = await backend.knownMesos(name);
      if (ask === asked) known = found;
    } catch {
      if (ask === asked) known = null;
    }
  }

  $effect(() => {
    const name = readName;
    if (remember) recall(name);
    else known = null;
  });

  onMount(() => {
    const clock = setInterval(() => (now = Date.now() / 1000), 30_000);
    const stop = remember ? backend.onKnownMesos(() => recall(readName)) : null;
    return () => {
      clearInterval(clock);
      stop?.then((unlisten) => unlisten());
    };
  });

  /** A remembered character stands in only until a name is read this run. */
  const lastPlayed = $derived(readName == null ? known : null);
  const displayName = $derived(readName ?? lastPlayed?.name ?? null);

  const fraction = (g: GaugeReading | null | undefined) => (g ? g.current / g.max : null);
  const gaugeText = (g: GaugeReading | null | undefined) =>
    g ? `${formatNumber(g.current)} / ${formatNumber(g.max)}` : "not read";

  const expPercent = $derived(hud?.exp?.percent ?? hud?.expPercent ?? null);
  const percentOnly = $derived(session?.percentOnly ?? false);
  /** The session's rate in a line: EXP an hour, or percent at levels where the HUD shows only that. */
  const rateLine = $derived.by(() => {
    if (!session) return null;
    if (percentOnly) return session.percentPerHour != null ? `${session.percentPerHour.toFixed(1)}% / hr` : null;
    return session.expPerHour != null ? `${formatCompact(session.expPerHour)} EXP / hr` : null;
  });
</script>

<Card title="Character" {plain}>
  {#snippet aside()}
    {#if hud?.job}<span>{hud.job}</span>{/if}
  {/snippet}

  <div class="identity">
    <div class="level">
      <span class="lv">Lv</span>
      <span class="tabular">{hud?.level ?? session?.level ?? "—"}</span>
    </div>
    <div class="who">
      <div class="name">
        {displayName ?? "Unknown character"}
        {#if lastPlayed}<span class="seen" title="No character read yet: the one Leafline saw last">last played</span>{/if}
      </div>
      <div class="mesos tabular">
        <span class="coin" aria-hidden="true"></span>
        {#if mesos}
          {formatNumber(mesos.mesos)} mesos
        {:else if session?.mesos != null}
          {formatNumber(session.mesos)} mesos
          <span class="seen">(last seen {formatDuration(session.mesosAge)} ago)</span>
        {:else if known}
          <span title="Remembered from when your inventory last showed them">{formatNumber(known.mesos)} mesos</span>
          <span class="seen">(last seen {formatAgo(now - known.seenAt)})</span>
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

  {#if rateLine}
    {#if linkTrain}
      <button class="to-train" onclick={() => openPanel("train")}>
        <span>
          <span class="tabular strong">{rateLine}</span>
          {#if session?.secondsToLevel != null && hud?.level != null}<span class="muted"> · Lv {hud.level + 1} in {formatDuration(session.secondsToLevel)}</span>{/if}
        </span>
        <span class="muted go">Train <ChevronRight size={12} /></span>
      </button>
    {:else}
      <p class="rate-line tabular">
        {rateLine}{#if session?.secondsToLevel != null}<span class="muted"> · {formatDuration(session.secondsToLevel)} to level</span>{/if}
      </p>
    {/if}
  {/if}
  {@render children?.()}
</Card>

<style>
  .to-train {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    width: 100%;
    margin-top: var(--space-3);
    padding: var(--space-2) 10px;
    border: 0;
    background: var(--surface-2);
    color: var(--text);
    font-size: var(--text-sm);
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
    gap: 2px;
  }

  .strong {
    font-weight: 600;
  }

  .muted {
    color: var(--muted);
  }

  .rate-line {
    margin: var(--space-3) 0 0;
    font-size: var(--text-sm);
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
    font-size: var(--text-xl);
    font-weight: 700;
    line-height: 1;
    color: var(--text);
  }

  .lv {
    font-size: var(--text-xs);
    font-weight: 400;
    color: var(--muted);
  }

  .name {
    font-family: var(--font-display);
    font-size: var(--text-lg);
    font-weight: 600;
  }

  .mesos {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--muted);
    font-size: var(--text-sm);
  }

  .seen {
    color: var(--faint);
  }

  .name .seen {
    margin-left: 6px;
    font-family: var(--font-sans);
    font-size: var(--text-xs);
    font-weight: 400;
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
</style>
