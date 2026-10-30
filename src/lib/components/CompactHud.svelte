<script lang="ts">
  // The compact HUD: EXP per hour and the Quest Helper's counters, in a small window that fits
  // what it shows. Everything else stays in the full overlay.
  import { Check, Maximize2 } from "@lucide/svelte";
  import { onMount } from "svelte";

  import { backend } from "$lib/api";
  import Logo from "$lib/components/Logo.svelte";
  import { formatCompact } from "$lib/format";
  import { live } from "$lib/readings.svelte";
  import { display } from "$lib/settings.svelte";

  /** Width in screen units at Normal size; the window grows with the Size setting. */
  const WIDTH = 236;
  /** More counters than this scroll instead of growing the window. */
  const MAX_HEIGHT = 320;

  let frame = $state<HTMLElement>();

  const session = $derived(live.session);
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
  <div class="bar" data-tauri-drag-region>
    <span class="logo" data-tauri-drag-region><Logo size={13} /></span>
    <span class="rate" data-tauri-drag-region>
      {#if session?.expPerHour != null}
        <span class="value tabular" data-tauri-drag-region>{formatCompact(session.expPerHour)}</span>
        <span class="unit" data-tauri-drag-region>EXP / hr</span>
        {#if session.percentPerHour != null}
          <span class="sub tabular" data-tauri-drag-region>{session.percentPerHour.toFixed(1)}%</span>
        {/if}
        {#if session.paused}<span class="sub" data-tauri-drag-region>· paused</span>{/if}
      {:else}
        <span class="unit" data-tauri-drag-region>{session ? "Measuring EXP / hr…" : "No EXP readings yet"}</span>
      {/if}
    </span>
    <button
      class="expand"
      title="Full overlay (Ctrl+Alt+H)"
      aria-label="Full overlay"
      onclick={() => backend.setCompact(false)}
    >
      <Maximize2 size={12} />
    </button>
  </div>

  {#if objectives.length}
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
    border-radius: 999px;
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
