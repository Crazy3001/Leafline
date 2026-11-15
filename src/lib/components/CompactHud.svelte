<script lang="ts">
  // The compact HUD: the numbers worth a glance while playing, in a small window that fits what it
  // shows. One layout: the level and an EXP bar like the game's, EXP per hour with how the last
  // five minutes compare, when the level comes, then mesos per hour and the session, then the quest
  // counters under each quest's name. Everything else stays in the full overlay.
  import { tip } from "$lib/tip.svelte";
  import { Check, Maximize2, Pause, Play } from "@lucide/svelte";
  import { onMount } from "svelte";

  import { backend } from "$lib/api";
  import Pic from "$lib/ui/Pic.svelte";
  import Logo from "$lib/components/Logo.svelte";
  import NextStep from "$lib/components/NextStep.svelte";
  import { formatClockIn, formatCompact, formatDuration, objectiveProgress } from "$lib/format";
  import ExpBar from "$lib/ui/ExpBar.svelte";
  import { live } from "$lib/readings.svelte";
  import { display, keyHint } from "$lib/settings.svelte";
  import { mesosCoverage } from "$lib/training";

  /** Width in screen units at Normal size; the window grows with the text size. */
  const WIDTH = 260;
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

  /** The clock time the level comes at this pace, for the To level tile's tooltip. */
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
  <div class="level-row" data-tauri-drag-region>
    <span class="lv tabular" data-tauri-drag-region>{level != null ? `Lv ${level}` : "Lv –"}</span>
    <span class="exp-bar" data-tauri-drag-region><ExpBar percent={percent ?? 0} small /></span>
  </div>

  <div class="top" data-tauri-drag-region>
    <span class="logo" data-tauri-drag-region><Logo size={13} /></span>
    <span class="rate" data-tauri-drag-region>
      {#if rate}
        <span class="value num" data-tauri-drag-region>{rate.value}</span>
        <span class="unit" data-tauri-drag-region>{rate.unit}</span>
        {#if session?.held || session?.paused}<span class="paused" data-tauri-drag-region>Paused</span>{:else if session?.measuring}<span
            class="paused"
            data-tauri-drag-region>Measuring</span
          >{:else if pace != null && pace !== 0}<span
            class="pace chip"
            class:up={pace > 0}
            class:down={pace < 0}
            use:tip={"The last five minutes against the whole session"}
            data-tauri-drag-region>{pace > 0 ? "▲" : "▼"}{Math.abs(pace)}%</span
          >{/if}
      {:else}
        <span class="unit" data-tauri-drag-region>{session && session.activeSeconds > 0 ? "Measuring…" : "Starts with your first EXP"}</span>
      {/if}
    </span>
    {#if reading}
      <button
        class="icon"
        use:tip={`${reading.held ? 'Resume' : 'Pause'} tracking${keyHint('PauseTracking')}`}
        aria-label={reading.held ? "Resume tracking" : "Pause tracking"}
        onclick={() => backend.setLiveHeld(!reading.held)}
      >
        {#if reading.held}<Play size={12} />{:else}<Pause size={12} />{/if}
      </button>
    {/if}
    <button class="icon" use:tip={`Full overlay${keyHint('ToggleCompact')}`} aria-label="Full overlay" onclick={() => backend.setCompact(false)}>
      <Maximize2 size={12} />
    </button>
  </div>

  {#if session?.secondsToLevel != null}
    <p class="level-up tabular" data-tauri-drag-region>
      Level up in <b>{formatDuration(session.secondsToLevel)}</b>{#if levelAt}<span class="at"> · about {levelAt}</span>{/if}
    </p>
  {/if}

  <div class="tiles" data-tauri-drag-region>
    <span class="tile" data-tauri-drag-region>
      <span class="label" data-tauri-drag-region>Last 5 min</span>
      <span class="tile-value num" data-tauri-drag-region>{recent}</span>
    </span>
    <span
      class="tile"
      data-tauri-drag-region
    >
      <span class="label" data-tauri-drag-region>Mesos / hr</span>
      <span class="tile-value num" class:faint={coverage?.partial && coverage.thin} data-tauri-drag-region
        >{formatCompact(session?.mesosPerHour)}</span
      >
    </span>
    <span class="tile" data-tauri-drag-region>
      <span class="label" data-tauri-drag-region>Session</span>
      <span class="tile-value num" data-tauri-drag-region>{session?.activeSeconds ? formatDuration(session.activeSeconds) : "—"}</span>
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
              <span class="count tabular" class:faint={objective.unread}
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
    max-height: 100vh;
    overflow-y: auto;
    user-select: none;
  }

  .level-row {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 8px 0 10px;
    cursor: grab;
  }

  .lv {
    flex: none;
    padding: 1px 5px;
    border-radius: 3px;
    background: linear-gradient(#ffb547, #e8891c);
    color: #2a1600;
    font-size: 10px;
    font-weight: 800;
    letter-spacing: 0.02em;
  }

  .exp-bar {
    flex: 1;
    min-width: 0;
  }

  .top {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: 4px 4px 0 10px;
    cursor: grab;
  }

  .level-up {
    margin: 0;
    padding: 0 10px 2px;
    color: var(--muted);
    font-size: var(--text-xs);
    cursor: grab;
  }

  .level-up b {
    color: var(--text);
    font-weight: 600;
  }

  .level-up .at {
    color: var(--faint);
  }

  .logo {
    display: inline-grid;
    flex: none;
  }

  .rate {
    display: flex;
    flex: 1;
    align-items: baseline;
    gap: 6px;
    min-width: 0;
    white-space: nowrap;
  }

  .value {
    font-size: var(--text-xl);
    font-weight: 600;
    line-height: 1.1;
  }

  .unit {
    font-size: var(--text-xs);
    color: var(--muted);
  }

  .paused {
    font-size: var(--text-xs);
    color: var(--warn);
  }

  .icon {
    display: grid;
    flex: none;
    place-items: center;
    width: 24px;
    height: 24px;
    padding: 0;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--muted);
    cursor: pointer;
  }

  .icon:hover {
    background: rgba(255, 255, 255, 0.07);
    color: var(--text);
  }

  .tiles {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 4px;
    padding: 4px var(--space-2) var(--space-2);
    cursor: grab;
  }

  .tile {
    display: flex;
    flex-direction: column;
    min-width: 0;
    padding: 4px 6px;
    border-radius: var(--radius-sm);
    background: var(--surface-2);
  }

  .label {
    font-size: 10px;
    color: var(--muted);
    white-space: nowrap;
  }

  .tile-value {
    font-size: var(--text-sm);
    font-weight: 600;
    white-space: nowrap;
  }

  .tile-value.faint {
    color: var(--faint);
  }

  .pace {
    margin-left: 3px;
    font-family: var(--font-sans);
    font-size: 10px;
    font-weight: 400;
    color: var(--muted);
  }

  .pace.up {
    color: var(--good);
  }

  .pace.down {
    color: var(--warn);
  }

  .pace.chip {
    margin-left: 0;
    padding: 0 5px;
    border-radius: var(--radius-pill);
    background: rgba(255, 255, 255, 0.06);
    font-size: 10px;
    font-variant-numeric: tabular-nums;
  }

  .quests {
    padding: 4px 10px 8px;
    border-top: 1px solid var(--border);
  }

  .quest-title {
    margin: 4px 0 2px;
    overflow: hidden;
    color: var(--faint);
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.03em;
    text-overflow: ellipsis;
    text-transform: uppercase;
    white-space: nowrap;
  }

  .counters {
    display: grid;
    gap: 5px;
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
  }

  .track {
    grid-column: 1 / -1;
    height: 3px;
    overflow: hidden;
    border-radius: var(--radius-pill);
    background: var(--surface-3);
  }

  .fill {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: var(--accent);
  }

  .done .target,
  .done .count {
    color: var(--good);
  }

  .done .fill {
    background: var(--good);
  }
</style>
