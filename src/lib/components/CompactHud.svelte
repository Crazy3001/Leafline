<script lang="ts">
  // The compact HUD: the numbers worth a glance while playing, in a small window that fits what it
  // shows. One layout: EXP per hour, the last five minutes, mesos per hour, time to level, then
  // the quest counters. Everything else stays in the full overlay.
  import { Check, Maximize2, Pause, Play } from "@lucide/svelte";
  import { onMount } from "svelte";

  import { backend } from "$lib/api";
  import Pic from "$lib/ui/Pic.svelte";
  import Logo from "$lib/components/Logo.svelte";
  import NextStep from "$lib/components/NextStep.svelte";
  import { formatCompact, formatDuration, objectiveProgress } from "$lib/format";
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
  const objectives = $derived(
    (live.analysis?.quest?.quests ?? []).flatMap((q) => q.objectives.map((o) => ({ ...o, quest: q.title }))),
  );

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
  <div class="level-track" title={percent != null ? `${percent.toFixed(2)}% toward the next level` : "Toward the next level"}>
    <span class="level-fill" style:width="{Math.min(100, percent ?? 0)}%"></span>
  </div>

  <div class="top" data-tauri-drag-region>
    <span class="logo" data-tauri-drag-region><Logo size={13} /></span>
    <span class="rate" data-tauri-drag-region>
      {#if rate}
        <span class="value num" data-tauri-drag-region>{rate.value}</span>
        <span class="unit" data-tauri-drag-region>{rate.unit}</span>
        {#if session?.held || session?.paused}<span class="paused" data-tauri-drag-region>Paused</span>{/if}
      {:else}
        <span class="unit" data-tauri-drag-region>{session && session.activeSeconds > 0 ? "Measuring…" : "Starts with your first EXP"}</span>
      {/if}
    </span>
    {#if reading}
      <button
        class="icon"
        title="{reading.held ? 'Resume' : 'Pause'} tracking{keyHint('PauseTracking')}"
        aria-label={reading.held ? "Resume tracking" : "Pause tracking"}
        onclick={() => backend.setLiveHeld(!reading.held)}
      >
        {#if reading.held}<Play size={12} />{:else}<Pause size={12} />{/if}
      </button>
    {/if}
    <button class="icon" title="Full overlay{keyHint('ToggleCompact')}" aria-label="Full overlay" onclick={() => backend.setCompact(false)}>
      <Maximize2 size={12} />
    </button>
  </div>

  <div class="tiles" data-tauri-drag-region>
    <span class="tile" data-tauri-drag-region>
      <span class="label" data-tauri-drag-region>Last 5 min</span>
      <span class="tile-value num" data-tauri-drag-region>
        {recent}{#if pace != null && pace !== 0}<span class="pace" class:up={pace > 0} class:down={pace < 0}
            >{pace > 0 ? "▲" : "▼"}{Math.abs(pace)}%</span
          >{/if}
      </span>
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
      <span class="label" data-tauri-drag-region>To level</span>
      <span class="tile-value num" data-tauri-drag-region>{formatDuration(session?.secondsToLevel)}</span>
    </span>
  </div>

  <NextStep variant="hud" />

  {#if objectives.length}
    <ul class="counters">
      {#each objectives as objective, i (i)}
        <!-- A line the data says is there but no reading has counted shows no count, not a 0. -->
        {@const progress = objective.unread ? 0 : objectiveProgress(objective)}
        {@const done = progress >= 1}
        <li class:done title={objective.quest}>
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
  {/if}
</div>

<style>
  .hud {
    max-height: 100vh;
    overflow-y: auto;
    user-select: none;
  }

  .level-track {
    height: 3px;
    background: var(--surface-3);
  }

  .level-fill {
    display: block;
    height: 100%;
    background: var(--exp);
  }

  .top {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: 6px 4px 2px 10px;
    cursor: grab;
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

  .counters {
    display: grid;
    gap: 5px;
    margin: 0;
    padding: 6px 10px 8px;
    border-top: 1px solid var(--border);
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
    height: 2px;
    background: var(--surface-3);
  }

  .fill {
    display: block;
    height: 100%;
    background: var(--muted);
  }

  .done .target,
  .done .count {
    color: var(--good);
  }

  .done .fill {
    background: var(--good);
  }
</style>
