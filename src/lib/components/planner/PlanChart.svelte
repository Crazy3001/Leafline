<script lang="ts">
  // The plan as a chart: levels run across, each skill goal is a bar over the levels whose SP goes
  // into it, and the AP row shows where each level's AP goes. A line marks the character's level;
  // click any level (or use the arrow keys) to see it in the panel beside.
  import { TriangleAlert } from "@lucide/svelte";

  import Ref from "$lib/components/Ref.svelte";
  import { tip } from "$lib/tip.svelte";
  import Pic from "$lib/ui/Pic.svelte";
  import Segmented from "$lib/ui/Segmented.svelte";

  import { statLabel } from "$lib/builds.svelte";
  import { STAT_COLOURS, laneState, type ApRun, type GoalLane, type LaneState } from "$lib/planner";
  import type { BuildProblem, LevelStep } from "$lib/types";

  let {
    lanes,
    ap,
    steps,
    start,
    end,
    level,
    bands,
    problems,
    selected,
    onselect,
  }: {
    lanes: GoalLane[];
    ap: ApRun[];
    steps: LevelStep[];
    /** The plan's first and last level. */
    start: number;
    end: number;
    /** The character's level, read from the game. */
    level: number | null;
    /** The job each stretch of levels is in. */
    bands: { from: number; to: number; label: string; tier: number }[];
    problems: BuildProblem[];
    selected: number;
    onselect: (level: number) => void;
  } = $props();

  /** A long plan shows 30 levels around the character at first; the whole plan is a click away. */
  const WINDOW = 30;
  type Range = "ahead" | "whole";
  const RANGE_KEY = "leafline.planner.range";
  let range = $state<Range>(storedRange());
  const long = $derived(end - start + 1 > WINDOW + 6);

  function storedRange(): Range {
    try {
      return localStorage.getItem(RANGE_KEY) === "whole" ? "whole" : "ahead";
    } catch {
      return "ahead";
    }
  }

  function pickRange(next: Range) {
    range = next;
    try {
      localStorage.setItem(RANGE_KEY, next);
    } catch {
      // A convenience only.
    }
  }

  const clamp = (n: number, lo: number, hi: number) => Math.max(lo, Math.min(hi, n));
  /** The levels shown. The window follows the picked level when it leaves it. */
  const lo = $derived.by(() => {
    if (!long || range === "whole") return start;
    let from = clamp((level ?? start) - 2, start, end - WINDOW + 1);
    if (selected < from || selected > from + WINDOW - 1) from = clamp(selected - Math.floor(WINDOW / 2), start, end - WINDOW + 1);
    return from;
  });
  const hi = $derived(!long || range === "whole" ? end : lo + WINDOW - 1);
  const n = $derived(Math.max(1, hi - lo + 1));

  /** Left edge and width of a run of levels, as percentages of the track. */
  const left = (l: number) => ((clamp(l, lo, hi + 1) - lo) / n) * 100;
  const mid = (l: number) => ((l - lo + 0.5) / n) * 100;
  const width = (a: number, b: number) => left(b + 1) - left(a);
  const inView = (l: number) => l >= lo && l <= hi;

  const tickStep = $derived(n <= 16 ? 2 : n <= 40 ? 5 : 10);
  const ticks = $derived(Array.from({ length: n }, (_, i) => lo + i).filter((l) => l % tickStep === 0));

  let trackWidth = $state(600);
  let track = $state<HTMLElement | null>(null);
  let hover = $state<number | null>(null);

  function levelAt(x: number): number | null {
    if (!track) return null;
    const rect = track.getBoundingClientRect();
    const share = (x - rect.left) / rect.width;
    if (share < 0 || share > 1) return null;
    return clamp(lo + Math.floor(share * n), lo, hi);
  }

  function key(e: KeyboardEvent) {
    const by: Record<string, number> = { ArrowLeft: -1, ArrowRight: 1, ArrowDown: -1, ArrowUp: 1, PageDown: -5, PageUp: 5 };
    if (e.key in by) onselect(clamp(selected + by[e.key], start, end));
    else if (e.key === "Home") onselect(start);
    else if (e.key === "End") onselect(end);
    else return;
    e.preventDefault();
  }

  const STATE_TEXT: Record<LaneState, string> = { done: "Done", now: "Next level", later: "Later", short: "Short of the goal" };

  /** What a lane's bar says, and whether it fits inside the bar. */
  function barOf(lane: GoalLane) {
    const state = laneState(lane, level);
    const first = lane.spends[0]?.level ?? null;
    const last = lane.spends.at(-1)?.level ?? null;
    const got = lane.floor + lane.spends.reduce((sum, s) => sum + s.amount, 0);
    let text: string;
    if (lane.reached.kind === "never") text = `${got} of ${lane.target} by Lv ${end}`;
    else if (first != null && last != null && first !== last) text = `→ ${lane.target} · Lv ${first}–${last}`;
    else if (first != null) text = `→ ${lane.target} · Lv ${first}`;
    else text = `→ ${lane.target}`;
    const shown = first != null && last != null && last >= lo && first <= hi;
    const a = first == null ? 0 : Math.max(first, lo);
    const b = last == null ? 0 : Math.min(last, hi);
    const px = shown ? (width(a, b) / 100) * trackWidth : 0;
    const fits = px >= text.length * 6.2 + 14;
    const room = (1 - left(b + 1) / 100) * trackWidth;
    return { state, first, last, text, shown, a, b, fits, outsideLeft: !fits && room < text.length * 6.2 + 12 };
  }

  /** Where the done part of a bar ends: the character's level. */
  const doneTo = (a: number, b: number) => (level == null ? a - 1 : Math.min(b, level));

  const gearMarks = $derived(steps.filter((s) => s.gear.length && inView(s.level)));
  const advances = $derived(steps.filter((s) => s.advance && inView(s.level)));
  const marked = $derived(problems.filter((p) => p.level != null && inView(p.level)));
</script>

<div class="chart" style:--n={n}>
  <div class="bar-top">
    <div class="legend" aria-hidden="true">
      <span><i class="done"></i>Done</span>
      <span><i class="now"></i>Next level</span>
      <span><i class="later"></i>Later</span>
      <span><i class="short"></i>Short of the goal</span>
    </div>
    {#if long}
      <Segmented
        label="Levels shown"
        size="sm"
        options={[
          { id: "ahead" as Range, label: `${WINDOW} levels`, title: "The levels around you" },
          { id: "whole" as Range, label: "Whole plan", title: `Lv ${start} to ${end}` },
        ]}
        value={range}
        onchange={pickRange}
      />
    {/if}
  </div>

  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <div
    class="plot"
    role="slider"
    tabindex="0"
    aria-label="Level on the plan"
    aria-valuemin={start}
    aria-valuemax={end}
    aria-valuenow={selected}
    aria-valuetext="Level {selected}"
    onpointermove={(e) => (hover = levelAt(e.clientX))}
    onpointerleave={() => (hover = null)}
    onclick={(e) => {
      const at = levelAt(e.clientX);
      if (at != null && !(e.target as HTMLElement).closest("a, button")) onselect(at);
    }}
    onkeydown={key}
  >
    <!-- Levels and jobs -->
    <div class="row axis">
      <span class="label faint">Level</span>
      <div class="track" bind:this={track} bind:clientWidth={trackWidth}>
        {#each bands as band (band.tier)}
          {#if band.to >= lo && band.from <= hi}
            {@const share = width(Math.max(band.from, lo), Math.min(band.to, hi))}
            <span class="band tier-{band.tier}" style:left="{left(band.from)}%" style:width="{share}%" use:tip={band.label}
              >{(share / 100) * trackWidth > 60 ? band.label : ""}</span
            >
          {/if}
        {/each}
        {#each ticks as t (t)}
          <span class="tick tabular" style:left="{mid(t)}%">{t}</span>
        {/each}
        {#each marked as p (p.message)}
          <span class="problem" style:left="{mid(p.level!)}%" use:tip={p.message} aria-label={p.message}><TriangleAlert size={12} /></span>
        {/each}
        {#if hover != null}<span class="hover-at tabular" style:left="{mid(hover)}%">Lv {hover}</span>{/if}
      </div>
    </div>

    <div class="lanes">
      <!-- Guides behind the bars: the picked level, the hovered one, job changes, the character. -->
      <div class="guides" aria-hidden="true">
        {#if hover != null && hover !== selected}<span class="col hover" style:left="{left(hover)}%" style:width="{100 / n}%"></span>{/if}
        {#if inView(selected)}<span class="col picked" style:left="{left(selected)}%" style:width="{100 / n}%"></span>{/if}
        {#each advances as s (s.level)}<span class="advance" style:left="{left(s.level)}%"></span>{/each}
        {#if level != null && inView(level)}
          <span class="you" style:left="{mid(level)}%"><b>You · Lv {level}</b></span>
        {/if}
      </div>

      {#each lanes as lane (lane.index)}
        {@const bar = barOf(lane)}
        <div class="row lane">
          <span class="label"><Ref kind="skill" id={lane.skillId} name={lane.name} /></span>
          <div class="track">
            {#if bar.shown}
              {@const done = doneTo(bar.a, bar.b)}
              <span
                class="bar {bar.state}"
                style:left="{left(bar.a)}%"
                style:width="{width(bar.a, bar.b)}%"
                class:cut-left={bar.first != null && bar.first < lo}
                class:cut-right={bar.last != null && bar.last > hi}
                use:tip={`${lane.name} → ${lane.target}: ${STATE_TEXT[bar.state].toLowerCase()}. ${lane.spends.map((s) => `Lv ${s.level} +${s.amount}`).join(", ")}`}
              >
                {#if bar.state !== "done" && done >= bar.a}<span class="done-part" style:width="{(width(bar.a, done) / width(bar.a, bar.b)) * 100}%"></span>{/if}
                {#if bar.fits}<span class="in">{bar.text}</span>{/if}
              </span>
              {#if !bar.fits}
                <span
                  class="out {bar.state}"
                  class:before={bar.outsideLeft}
                  style:left={bar.outsideLeft ? null : `calc(${left(bar.b + 1)}% + 5px)`}
                  style:right={bar.outsideLeft ? `calc(${100 - left(bar.a)}% + 5px)` : null}>{bar.text}</span
                >
              {/if}
            {:else if bar.first == null}
              <span class="out note" style:left="4px"
                >{lane.reached.kind === "already" ? `→ ${lane.target} · had it at the start` : lane.reached.kind === "at" ? `→ ${lane.target} · an earlier goal covers it` : `gets no SP by Lv ${end}`}</span
              >
            {:else if bar.last != null && bar.last < lo}
              <span class="out note" style:left="4px">← {bar.text}</span>
            {:else}
              <span class="out note" style:right="4px">{bar.text} →</span>
            {/if}
          </div>
        </div>
      {:else}
        <div class="row lane"><span class="label faint">No skill goals</span><div class="track"></div></div>
      {/each}

      <div class="row lane rule"><span class="label"></span><div class="track"></div></div>

      <div class="row lane">
        <span class="label faint">Ability points</span>
        <div class="track">
          {#each ap as run (run.from)}
            {#if run.to >= lo && run.from <= hi}
              {@const a = Math.max(run.from, lo)}
              {@const b = Math.min(run.to, hi)}
              {@const px = (width(a, b) / 100) * trackWidth}
              <span
                class="bar ap"
                style:--stat={STAT_COLOURS[run.stat]}
                style:left="{left(a)}%"
                style:width="{width(a, b)}%"
                class:cut-left={run.from < lo}
                class:cut-right={run.to > hi}
                use:tip={`Lv ${run.from}${run.to > run.from ? `–${run.to}` : ""}: ${run.amount} AP into ${statLabel(run.stat)}, to ${run.total}`}
                ><span class="in">{px > 70 ? `${statLabel(run.stat)} → ${run.total}` : px > 28 ? statLabel(run.stat) : ""}</span></span
              >
            {/if}
          {/each}
        </div>
      </div>

      {#if gearMarks.length}
        <div class="row lane gear">
          <span class="label faint">New gear</span>
          <div class="track">
            {#each gearMarks as s (s.level)}
              <span class="mark" style:left="{mid(s.level)}%" use:tip={`Lv ${s.level}: ${s.gear.join(", ")}`} aria-label="Lv {s.level}: {s.gear.join(', ')}">
                <Pic kind="item" name={s.gear[0]} size={18}>
                  {#snippet fallback()}<span class="dot"></span>{/snippet}
                </Pic>
                {#if s.gear.length > 1}<b>{s.gear.length}</b>{/if}
              </span>
            {/each}
          </div>
        </div>
      {/if}
    </div>
  </div>

  <p class="key">
    Click a level, or use the arrow keys here, to see it beside the chart.
    {#if advances.length}The dashed line is a job advancement.{/if}
    {#if lo > start || hi < end}Showing Lv {lo} to {hi} of {start} to {end}.{/if}
  </p>
</div>

<style>
  .chart {
    --label: 132px;
    container: chart / inline-size;
  }

  @container chart (max-width: 520px) {
    .chart .row {
      --label: 92px;
    }
  }

  .bar-top {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 6px 12px;
    margin-bottom: 8px;
  }

  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 12px;
    color: var(--muted);
    font-size: var(--text-xs);
  }

  .legend span {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }

  .legend i {
    display: inline-block;
    width: 14px;
    height: 8px;
    border-radius: 3px;
  }

  .plot {
    position: relative;
    border-radius: var(--radius-sm);
    outline: none;
    cursor: pointer;
    user-select: none;
  }

  .plot:focus-visible {
    box-shadow: 0 0 0 2px var(--accent);
  }

  .row {
    display: grid;
    grid-template-columns: var(--label) minmax(0, 1fr);
    align-items: center;
  }

  .label {
    min-width: 0;
    padding-right: 10px;
    overflow: hidden;
    font-size: var(--text-sm);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .faint {
    color: var(--faint);
  }

  .track {
    position: relative;
    height: 28px;
  }

  .axis .track {
    height: 42px;
  }

  .band {
    position: absolute;
    top: 0;
    height: 18px;
    padding: 0 7px;
    overflow: hidden;
    border-radius: 6px;
    background: var(--surface-2);
    color: var(--muted);
    font-size: var(--text-xs);
    line-height: 18px;
    text-overflow: ellipsis;
    white-space: nowrap;
    box-shadow: inset 0 0 0 1px var(--border);
  }

  .band.tier-2 {
    background: color-mix(in srgb, var(--ref-skill) 14%, var(--surface));
    color: var(--ref-skill);
  }

  .tick,
  .hover-at {
    position: absolute;
    bottom: 2px;
    color: var(--faint);
    font-family: var(--font-display);
    font-size: var(--text-xs);
    transform: translateX(-50%);
  }

  .hover-at {
    padding: 0 5px;
    border-radius: 4px;
    background: var(--text);
    color: var(--bg);
  }

  .problem {
    position: absolute;
    top: 21px;
    display: grid;
    color: var(--warn);
    transform: translateX(-50%);
  }

  .lanes {
    position: relative;
  }

  .guides {
    position: absolute;
    top: 0;
    bottom: 0;
    left: var(--label);
    right: 0;
    pointer-events: none;
  }

  @container chart (max-width: 520px) {
    .guides {
      left: 92px;
    }
  }

  .col {
    position: absolute;
    top: 0;
    bottom: 0;
  }

  .col.hover {
    background: var(--hover);
  }

  .col.picked {
    background: var(--accent-soft);
    box-shadow:
      inset 1px 0 0 color-mix(in srgb, var(--accent) 45%, transparent),
      inset -1px 0 0 color-mix(in srgb, var(--accent) 45%, transparent);
  }

  .advance {
    position: absolute;
    top: 0;
    bottom: 0;
    border-left: 1px dashed color-mix(in srgb, var(--ref-skill) 70%, transparent);
  }

  .you {
    position: absolute;
    top: -6px;
    bottom: 0;
    width: 2px;
    margin-left: -1px;
    background: var(--accent);
  }

  .you b {
    position: absolute;
    top: -12px;
    left: 50%;
    padding: 1px 7px;
    border-radius: 5px;
    background: var(--accent);
    color: var(--accent-ink);
    font-size: 10.5px;
    font-weight: 600;
    white-space: nowrap;
    transform: translateX(-50%);
  }

  .lane + .lane {
    border-top: 1px solid color-mix(in srgb, var(--border) 55%, transparent);
  }

  .lane.rule .track {
    height: 6px;
  }

  .lane.rule + .lane {
    border-top: 1px solid var(--border);
  }

  .bar {
    position: absolute;
    top: 5px;
    display: flex;
    align-items: center;
    height: 18px;
    overflow: hidden;
    border-radius: 6px;
    font-family: var(--font-display);
    font-size: 10.5px;
    font-weight: 600;
    white-space: nowrap;
  }

  .bar.cut-left {
    border-top-left-radius: 0;
    border-bottom-left-radius: 0;
  }

  .bar.cut-right {
    border-top-right-radius: 0;
    border-bottom-right-radius: 0;
  }

  .in {
    position: relative;
    padding: 0 7px;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .done-part {
    position: absolute;
    top: 0;
    bottom: 0;
    left: 0;
    background: color-mix(in srgb, var(--good) 30%, var(--surface));
  }

  .bar.done,
  .legend .done {
    background: var(--good-soft);
    color: var(--good);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--good) 35%, transparent);
  }

  .bar.now,
  .legend .now {
    background: var(--accent);
    color: var(--accent-ink);
  }

  .bar.later,
  .legend .later {
    background: color-mix(in srgb, var(--ref-skill) 16%, var(--surface));
    color: var(--ref-skill);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--ref-skill) 35%, transparent);
  }

  .bar.short,
  .legend .short {
    background: var(--warn-soft);
    color: var(--warn);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--warn) 45%, transparent);
  }

  .bar.ap {
    background: color-mix(in srgb, var(--stat) 22%, var(--surface));
    color: var(--text);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--stat) 45%, transparent);
  }

  .out {
    position: absolute;
    top: 5px;
    height: 18px;
    color: var(--muted);
    font-family: var(--font-display);
    font-size: 10.5px;
    font-weight: 600;
    line-height: 18px;
    white-space: nowrap;
  }

  .out.done {
    color: var(--good);
  }

  .out.short {
    color: var(--warn);
  }

  .out.note {
    color: var(--faint);
    font-weight: 400;
  }

  .mark {
    position: absolute;
    top: 4px;
    display: inline-flex;
    align-items: center;
    gap: 1px;
    transform: translateX(-50%);
  }

  .mark b {
    color: var(--muted);
    font-size: 10px;
  }

  .dot {
    display: block;
    width: 9px;
    height: 9px;
    border-radius: 2px;
    background: var(--ref-item);
    transform: rotate(45deg);
  }

  .key {
    margin: 8px 0 0;
    color: var(--faint);
    font-size: var(--text-xs);
  }
</style>
