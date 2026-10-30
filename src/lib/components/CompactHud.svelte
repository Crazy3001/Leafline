<script lang="ts">
  // The compact HUD: the session's main numbers in a small window that fits what it shows, in the
  // shape chosen in Settings. Everything else stays in the full overlay.
  import { Check, Maximize2, Pause, Play } from "@lucide/svelte";
  import { onMount } from "svelte";

  import { backend } from "$lib/api";
  import Logo from "$lib/components/Logo.svelte";
  import { formatCompact, formatDuration } from "$lib/format";
  import { live } from "$lib/readings.svelte";
  import { display } from "$lib/settings.svelte";

  /** Width in screen units at Normal size; the window grows with the Size setting. */
  const WIDTHS = { card: 248, wide: 400, bar: 460 };
  /** More counters than this scroll instead of growing the window. */
  const MAX_HEIGHT = 320;

  let frame = $state<HTMLElement>();

  const session = $derived(live.session);
  const shape = $derived(display.compactShape);
  const reading = $derived(live.liveStatus.state === "reading" ? live.liveStatus : null);
  const percent = $derived(
    live.analysis?.hud?.exp?.percent ?? live.analysis?.hud?.expPercent ?? session?.percent ?? null,
  );
  const percentOnly = $derived(session?.percentOnly ?? false);
  const stats = $derived([
    {
      label: "Last 5 min",
      value: percentOnly
        ? `${session?.recentPercentPerHour?.toFixed(1) ?? "—"}%`
        : formatCompact(session?.recentExpPerHour),
    },
    { label: "Mesos / hr", value: formatCompact(session?.mesosPerHour) },
    { label: "To level", value: formatDuration(session?.secondsToLevel) },
  ]);
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
    const size = [WIDTHS[shape] * display.scale, Math.min(MAX_HEIGHT, Math.ceil(frame.scrollHeight)) * display.scale];
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
    void shape;
    fit();
  });
</script>

{#snippet tiles()}
  {#each stats as stat (stat.label)}
    <span class="stat" title={stat.label} data-tauri-drag-region>
      <span class="stat-label" data-tauri-drag-region>{stat.label}</span>
      <span class="stat-value tabular" data-tauri-drag-region>{stat.value}</span>
    </span>
  {/each}
{/snippet}

<div class="hud {shape}" bind:this={frame}>
  <div class="level-track" title="Toward the next level">
    <span class="level-fill" style:width="{Math.min(100, percent ?? 0)}%"></span>
  </div>
  <div class="bar" data-tauri-drag-region>
    <span class="logo" data-tauri-drag-region><Logo size={13} /></span>
    <span class="rate" data-tauri-drag-region>
      {#if session?.expPerHour != null}
        <span class="value tabular" data-tauri-drag-region>{formatCompact(session.expPerHour)}</span>
        <span class="unit" data-tauri-drag-region>EXP / hr</span>
        {#if session.percentPerHour != null && shape === "card"}
          <span class="sub tabular" data-tauri-drag-region>{session.percentPerHour.toFixed(1)}%</span>
        {/if}
        {#if session.paused}<span class="sub" data-tauri-drag-region>· paused</span>{/if}
      {:else if percentOnly && session?.percentPerHour != null}
        <span class="value tabular" data-tauri-drag-region>{session.percentPerHour.toFixed(1)}%</span>
        <span class="unit" data-tauri-drag-region>/ hr</span>
        {#if session.paused}<span class="sub" data-tauri-drag-region>· paused</span>{/if}
      {:else}
        <span class="unit" data-tauri-drag-region>{session ? "Measuring EXP / hr…" : "No EXP readings yet"}</span>
      {/if}
    </span>
    {#if shape === "bar"}{@render tiles()}{/if}
    {#if reading}
      <button
        class="expand"
        title="{reading.held ? 'Resume' : 'Pause'} tracking (Ctrl+Alt+K)"
        aria-label={reading.held ? "Resume tracking" : "Pause tracking"}
        onclick={() => backend.setLiveHeld(!reading.held)}
      >
        {#if reading.held}<Play size={12} />{:else}<Pause size={12} />{/if}
      </button>
    {/if}
    <button
      class="expand"
      title="Full overlay (Ctrl+Alt+H)"
      aria-label="Full overlay"
      onclick={() => backend.setCompact(false)}
    >
      <Maximize2 size={12} />
    </button>
  </div>

  {#if shape !== "bar"}<div class="stats" data-tauri-drag-region>{@render tiles()}</div>{/if}

  {#if shape !== "card"}
    <!-- Wide and Bar are for the numbers alone. -->
  {:else if objectives.length}
    <ul class="counters">
      {#each objectives as objective, i (i)}
        {@const done = objective.current >= objective.required}
        <li class:done title={objective.quest}>
          <span class="target">
            {#if done}<Check size={10} />{/if}
            {objective.target}
          </span>
          <span class="count tabular">{objective.current}/{objective.required}</span>
          <span class="track"
            ><span class="fill" style:width="{Math.min(1, objective.current / objective.required) * 100}%"></span></span
          >
        </li>
      {/each}
    </ul>
  {:else}
    <p class="empty">{live.analysis?.quest ? "No counters read." : "Quest Helper not in view."}</p>
  {/if}
</div>

<style>
  .hud {
    max-height: 100vh;
    overflow-y: auto;
  }

  .level-track {
    height: 2px;
    background: rgba(255, 255, 255, 0.06);
  }

  .level-fill {
    display: block;
    height: 100%;
    background: var(--exp);
  }

  .stats {
    display: flex;
    gap: 12px;
    padding: 2px 10px 7px;
    user-select: none;
    cursor: grab;
  }

  .stat {
    display: grid;
    flex: 1;
    min-width: 0;
    white-space: nowrap;
  }

  .stat-label {
    color: var(--faint);
    font-size: 10px;
  }

  .stat-value {
    font-size: 12px;
    font-weight: 600;
  }

  .hud.bar .stat {
    flex: none;
    display: inline-flex;
    align-items: baseline;
    gap: 4px;
  }

  .hud.bar .stat-label {
    display: none;
  }

  .bar {
    display: flex;
    align-items: center;
    gap: 7px;
    height: 30px;
    padding: 0 4px 0 9px;
    user-select: none;
    cursor: grab;
  }

  .logo {
    display: inline-grid;
    flex: none;
  }

  .rate {
    display: inline-flex;
    flex: 1;
    align-items: baseline;
    gap: 5px;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
  }

  .value {
    font-size: 14px;
    font-weight: 600;
    color: var(--exp);
  }

  .unit,
  .sub {
    color: var(--muted);
    font-size: 11px;
  }

  .sub {
    color: var(--faint);
  }

  .expand {
    display: grid;
    flex: none;
    place-items: center;
    width: 22px;
    height: 22px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--faint);
    cursor: pointer;
  }

  .expand:hover {
    background: rgba(255, 255, 255, 0.07);
    color: var(--text);
  }

  .counters {
    display: grid;
    gap: 5px;
    margin: 0;
    padding: 4px 10px 9px;
    border-top: 1px solid var(--border);
    list-style: none;
  }

  li {
    display: grid;
    grid-template-columns: 1fr auto;
    align-items: baseline;
    column-gap: 8px;
    row-gap: 2px;
    font-size: 11.5px;
  }

  .target {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .count {
    color: var(--muted);
  }

  .track {
    grid-column: 1 / -1;
    height: 2px;
    border-radius: var(--radius-sm);
    background: rgba(255, 255, 255, 0.06);
    overflow: hidden;
  }

  .fill {
    display: block;
    height: 100%;
    background: var(--accent);
  }

  .done .target,
  .done .count {
    color: var(--good);
  }

  .done .fill {
    background: var(--good);
  }

  .empty {
    margin: 0;
    padding: 6px 10px 9px;
    border-top: 1px solid var(--border);
    color: var(--faint);
    font-size: 11.5px;
  }
</style>
