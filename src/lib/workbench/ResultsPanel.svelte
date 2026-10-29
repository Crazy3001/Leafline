<script lang="ts">
  import CharacterCard from "$lib/components/CharacterCard.svelte";
  import ItemCard from "$lib/components/ItemCard.svelte";
  import QuestCard from "$lib/components/QuestCard.svelte";
  import { formatMs } from "$lib/format";
  import type { Analysis, Calibration, HudValues, Rect } from "$lib/types";
  import HudCalibration from "./HudCalibration.svelte";

  let {
    analysis,
    highlight = $bindable(null),
    calibrate,
  }: {
    analysis: Analysis;
    highlight?: Rect | null;
    calibrate: (values: HudValues) => Promise<Calibration>;
  } = $props();

  let tab = $state<"readings" | "text">("readings");
  let teaching = $state(false);
  const total = $derived(analysis.timings.reduce((sum, t) => sum + t.ms, 0));
</script>

<div class="panel">
  <div class="tabs" role="tablist">
    <button role="tab" aria-selected={tab === "readings"} onclick={() => (tab = "readings")}>Readings</button>
    <button role="tab" aria-selected={tab === "text"} onclick={() => (tab = "text")}>
      Raw OCR <span class="count">{analysis.overview.length}</span>
    </button>
  </div>

  <div class="body">
    {#if tab === "readings"}
      <CharacterCard hud={analysis.hud} mesos={analysis.mesos} />
      {#if analysis.hud}
        {#if analysis.hud.needsCalibration || teaching}
          <HudCalibration hud={analysis.hud} {calibrate} />
        {:else}
          <button class="link" onclick={() => (teaching = true)}>Teach the HUD again from this screenshot</button>
        {/if}
      {/if}
      {#each analysis.tooltips as item, i}
        <ItemCard {item} title={analysis.tooltips.length > 1 ? `Item ${i + 1}` : "Item"} />
      {:else}
        <p class="none">No item tooltip found.</p>
      {/each}
      {#if analysis.quest}
        <QuestCard quest={analysis.quest} />
      {:else}
        <p class="none">No Quest Helper found.</p>
      {/if}

      <section class="timings">
        <div class="eyebrow">Pipeline · {formatMs(total)}</div>
        {#each analysis.timings as timing}
          <div class="timing">
            <span>{timing.stage}</span>
            <span class="bar"><span style:width="{(timing.ms / total) * 100}%"></span></span>
            <span class="tabular">{formatMs(timing.ms)}</span>
          </div>
        {/each}
      </section>
    {:else}
      <p class="note">Text from the whole-frame pass, used to find panels. Hover a line to see where it is.</p>
      <ul class="lines">
        {#each analysis.overview as line}
          <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
          <li onmouseenter={() => (highlight = line.rect)} onmouseleave={() => (highlight = null)}>
            <span class="text">{line.text}</span>
            <span class="pos tabular">{Math.round(line.rect.x)}, {Math.round(line.rect.y)}</span>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</div>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }

  .tabs {
    display: flex;
    gap: 2px;
    padding: 10px 12px 0;
    border-bottom: 1px solid var(--border);
  }

  .tabs button {
    padding: 8px 10px;
    border: 0;
    border-bottom: 2px solid transparent;
    background: none;
    color: var(--muted);
    font-weight: 500;
    cursor: pointer;
  }

  .tabs button[aria-selected="true"] {
    color: var(--text);
    border-bottom-color: var(--accent);
  }

  .count {
    margin-left: 4px;
    padding: 0 5px;
    border-radius: 999px;
    background: var(--surface-3);
    font-size: 10.5px;
    color: var(--muted);
  }

  .body {
    flex: 1;
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    align-content: start;
    gap: 10px;
    padding: 12px;
    overflow-y: auto;
  }

  .link {
    justify-self: start;
    padding: 0 2px;
    border: 0;
    background: none;
    color: var(--faint);
    font-size: 12px;
    text-decoration: underline;
    text-underline-offset: 3px;
    cursor: pointer;
  }

  .link:hover {
    color: var(--muted);
  }

  .none,
  .note {
    margin: 0;
    padding: 0 2px;
    color: var(--faint);
    font-size: 12px;
  }

  .timings {
    display: grid;
    gap: 5px;
    padding: 10px 2px 0;
  }

  .timing {
    display: grid;
    grid-template-columns: 110px 1fr 60px;
    align-items: center;
    gap: 8px;
    color: var(--muted);
    font-size: 12px;
  }

  .timing .tabular {
    text-align: right;
  }

  .bar {
    height: 4px;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.05);
    overflow: hidden;
  }

  .bar span {
    display: block;
    height: 100%;
    background: var(--faint);
  }

  .lines {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .lines li {
    display: flex;
    justify-content: space-between;
    gap: 10px;
    padding: 5px 8px;
    border-radius: var(--radius-sm);
    font-family: var(--font-mono);
    font-size: 11.5px;
  }

  .lines li:hover {
    background: var(--surface-3);
  }

  .text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .pos {
    flex: none;
    color: var(--faint);
  }
</style>
