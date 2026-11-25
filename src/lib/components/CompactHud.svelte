<script lang="ts">
  // The compact HUD: one strip of the numbers worth a glance while playing, in a small window that
  // fits what it shows. The level with an EXP bar like the game's, then EXP per hour with how the
  // last five minutes compare, mesos per hour, when the level comes and how long the session has
  // run. Under it, when there is one, the route's next step, then the quest counters. Everything
  // else stays in the full overlay.
  import { tip } from "$lib/tip.svelte";
  import { Check, Clock, Maximize2, Pause, Play } from "@lucide/svelte";
  import { onMount } from "svelte";

  import { backend } from "$lib/api";
  import { isWeb } from "$lib/platform";
  import Pic from "$lib/ui/Pic.svelte";
  import NextStep from "$lib/components/NextStep.svelte";
  import { formatClockIn, formatCompact, formatDuration, objectiveProgress } from "$lib/format";
  import ExpBar from "$lib/ui/ExpBar.svelte";
  import { live } from "$lib/readings.svelte";
  import { display, keyHint } from "$lib/settings.svelte";
  import { mesosCoverage } from "$lib/training";

  /** Width in screen units at Normal size; the window grows with the text size. */
  const WIDTH = 680;
  /** More counters than this scroll instead of growing the window. */
  const MAX_HEIGHT = 320;

  let frame = $state<HTMLElement>();

  const session = $derived(live.session);
  const reading = $derived(live.liveStatus.state === "reading" ? live.liveStatus : null);
  const percent = $derived(
    live.analysis?.hud?.exp?.percent ?? live.analysis?.hud?.expPercent ?? session?.percent ?? null,
  );
  const percentOnly = $derived(session?.percentOnly ?? false);
  const coverage = $derived(session ? mesosCoverage(session) : null);

  /** The clock time the level comes at this pace, for the time to level's tooltip. */
  const levelAt = $derived(session?.secondsToLevel != null && !session.paused ? formatClockIn(session.secondsToLevel) : "");

  const rate = $derived.by(() => {
    if (!session) return null;
    if (percentOnly) return session.percentPerHour != null ? { value: `${session.percentPerHour.toFixed(1)}%`, unit: "/ hr" } : null;
    return session.expPerHour != null ? { value: formatCompact(session.expPerHour), unit: "EXP / hr" } : null;
  });
  /** The last five minutes against the whole session, in percent. */
  const pace = $derived.by(() => {
    const recent = percentOnly ? session?.recentPercentPerHour : session?.recentExpPerHour;
    const whole = percentOnly ? session?.percentPerHour : session?.expPerHour;
    if (recent == null || !whole) return null;
    return Math.round((recent / whole - 1) * 100);
  });
  const recent = $derived(
    percentOnly
      ? session?.recentPercentPerHour != null
        ? `${session.recentPercentPerHour.toFixed(1)}%`
        : "—"
      : formatCompact(session?.recentExpPerHour),
  );
  const quests = $derived((live.analysis?.quest?.quests ?? []).filter((q) => q.objectives.length));
  const level = $derived(session?.level ?? live.analysis?.hud?.level ?? null);

  let asked = "";

  /**
   * Sizes the window to the content. The window is sized in screen units, the page in zoomed ones.
   * Only a change in what the content needs asks again: the window being resized from outside
   * (going back to the full overlay) must not be undone.
   */
  function fit() {
    if (!frame) return;
    const size = [WIDTH * display.scale, Math.min(MAX_HEIGHT, Math.ceil(frame.scrollHeight)) * display.scale];
    if (size.join() === asked) return;
    asked = size.join();
    backend.fitWindow(size[0], size[1]);
  }

  onMount(() => {
    if (!frame) return;
    const observer = new ResizeObserver(fit);
    observer.observe(frame);
    return () => observer.disconnect();
  });

  $effect(() => {
    void display.scale;
    fit();
  });
</script>

<div class="hud" bind:this={frame}>
  <div class="strip" data-tauri-drag-region>
    <span class="lv num" data-tauri-drag-region>{level != null ? `Lv ${level}` : "Lv –"}</span>
    <span class="exp-bar" data-tauri-drag-region><ExpBar percent={percent ?? 0} small /></span>
    <span class="sep" aria-hidden="true"></span>

    {#if rate}
      <span class="stat" data-tauri-drag-region>
        <b class="num" data-tauri-drag-region>{rate.value}</b>
        <span class="unit" data-tauri-drag-region>{rate.unit}</span>
        {#if session?.held || session?.paused}<span class="state" data-tauri-drag-region>Paused</span>{:else if session?.measuring}<span
            class="state"
            data-tauri-drag-region>Measuring</span
          >{:else if pace != null && pace !== 0}<span
            class="pace"
            class:up={pace > 0}
            class:down={pace < 0}
            use:tip={`Last 5 min: ${recent}${percentOnly ? " / hr" : " EXP / hr"}, against the whole session`}
            data-tauri-drag-region>{pace > 0 ? "▲" : "▼"}{Math.abs(pace)}%</span
          >{/if}
      </span>
      <span class="stat" data-tauri-drag-region>
        <b class="num mesos" class:faint={coverage?.partial && coverage.thin} data-tauri-drag-region>{formatCompact(session?.mesosPerHour)}</b>
        <span class="unit" data-tauri-drag-region>mesos / hr</span>
      </span>
      {#if session?.secondsToLevel != null}
        <span class="stat" use:tip={levelAt ? `About ${levelAt}` : null} data-tauri-drag-region>
          <b class="num" data-tauri-drag-region>{formatDuration(session.secondsToLevel)}</b>
          <span class="unit" data-tauri-drag-region>to Lv {level != null ? level + 1 : "up"}</span>
        </span>
      {/if}
      <span class="stat clock" use:tip={"This session's playing time"} data-tauri-drag-region>
        <Clock size={11} />
        <b class="num" data-tauri-drag-region>{session?.activeSeconds ? formatDuration(session.activeSeconds) : "—"}</b>
      </span>
    {:else}
      <span class="unit wait" data-tauri-drag-region>{session && session.activeSeconds > 0 ? "Measuring…" : "Starts with your first EXP"}</span>
    {/if}

    <span class="buttons">
      {#if reading}
        <button
          class="icon"
          use:tip={`${reading.held ? "Resume" : "Pause"} tracking${keyHint("PauseTracking")}`}
          aria-label={reading.held ? "Resume tracking" : "Pause tracking"}
          onclick={() => backend.setLiveHeld(!reading.held)}
        >
          {#if reading.held}<Play size={12} />{:else}<Pause size={12} />{/if}
        </button>
      {/if}
      {#if !isWeb}<button class="icon" use:tip={`Full overlay${keyHint("ToggleCompact")}`} aria-label="Full overlay" onclick={() => backend.setCompact(false)}>
        <Maximize2 size={12} />
      </button>{/if}
    </span>
  </div>

  <NextStep variant="hud" />

  {#if quests.length}
    <div class="quests">
      {#each quests as quest, q (q)}
        <p class="quest-title" title={quest.title}>{quest.title}</p>
        <ul class="counters">
          {#each quest.objectives as objective, i (i)}
            <!-- A line the data says is there but no reading has counted shows no count, not a 0. -->
            {@const progress = objective.unread ? 0 : objectiveProgress(objective)}
            {@const done = progress >= 1}
            <li class:done>
              <span class="target">
                {#if done}<Check size={10} />{/if}
                <!-- The counter names an item or a monster; whichever has a picture shows it. -->
                <Pic kind="item" name={objective.target} size={16} /><Pic kind="mob" name={objective.target} size={16} />
                {objective.target}
              </span>
              <span class="count num" class:faint={objective.unread}
                >{objective.unread ? "–" : objective.current}/{objective.required}</span
              >
              <span class="track"><span class="fill" style:width="{progress * 100}%"></span></span>
            </li>
          {/each}
        </ul>
      {/each}
    </div>
  {/if}
</div>

<style>
  .hud {
    max-height: calc(100vh / var(--page-zoom, 1));
    overflow-y: auto;
    user-select: none;
  }

  /* One row, wrapping only when the text size leaves no room. */
  .strip {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 12px;
    min-height: 44px;
    padding: 6px 6px 6px 14px;
    cursor: grab;
  }

  .lv {
    flex: none;
    padding: 2px 7px;
    border: var(--line-w) solid var(--control-line);
    border-radius: var(--radius-pill);
    background: var(--accent);
    color: var(--accent-ink);
    font-size: var(--text-xs);
    font-weight: var(--weight-strong);
  }

  :global(:root:not([data-theme^="sprout"])) .lv {
    border-color: transparent;
  }

  /* The bar prints its own percentage, as the game's does. */
  .exp-bar {
    flex: 0 0 120px;
    margin-left: -6px;
  }

  .sep {
    flex: none;
    width: 1px;
    height: 18px;
    background: var(--border-strong);
  }

  .stat {
    display: inline-flex;
    flex: none;
    align-items: baseline;
    gap: 5px;
    white-space: nowrap;
  }

  .stat b {
    font-size: var(--text-md);
    font-weight: var(--weight-strong);
  }

  .mesos {
    color: var(--mesos);
  }

  .clock {
    align-items: center;
    color: var(--faint);
  }

  .clock b {
    color: var(--text);
  }

  .faint {
    opacity: 0.55;
  }

  .unit {
    color: var(--faint);
    font-size: var(--text-xs);
  }

  .wait {
    flex: 1;
  }

  .state {
    color: var(--warn);
    font-size: var(--text-xs);
  }

  .pace {
    padding: 0 5px;
    border-radius: var(--radius-pill);
    background: var(--hover);
    color: var(--muted);
    font-size: var(--text-2xs);
    font-variant-numeric: tabular-nums;
  }

  .pace.up {
    color: var(--good);
  }

  .pace.down {
    color: var(--warn);
  }

  .buttons {
    display: inline-flex;
    flex: none;
    margin-left: auto;
  }

  .icon {
    display: grid;
    flex: none;
    place-items: center;
    width: 26px;
    height: 26px;
    padding: 0;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--muted);
    cursor: pointer;
  }

  .icon:hover {
    background: var(--hover);
    color: var(--text);
  }

  .quests {
    padding: 4px 14px 10px;
    border-top: 1px solid var(--border);
  }

  .quest-title {
    margin: 4px 0 3px;
    overflow: hidden;
    color: var(--faint);
    font-size: var(--text-2xs);
    font-weight: var(--weight-strong);
    letter-spacing: 0.03em;
    text-overflow: ellipsis;
    text-transform: uppercase;
    white-space: nowrap;
  }

  /* Two counters a row: the strip is wide and short. */
  .counters {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 6px 18px;
    margin: 0;
    padding: 0;
    list-style: none;
    font-size: var(--text-xs);
  }

  .counters li {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 2px 8px;
  }

  .target {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .count {
    color: var(--muted);
  }

  .count.faint {
    color: var(--faint);
    opacity: 1;
  }

  .track {
    grid-column: 1 / -1;
    height: 3px;
    overflow: hidden;
    border-radius: var(--radius-pill);
    background: var(--track);
  }

  .fill {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: var(--ref-item);
  }

  .done .target,
  .done .count {
    color: var(--good);
  }

  .done .fill {
    background: var(--good);
  }
</style>
