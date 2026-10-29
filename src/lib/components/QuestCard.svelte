<script lang="ts">
  import Card from "./Card.svelte";
  import type { QuestReading } from "$lib/types";

  let { quest, plain = false }: { quest: QuestReading; plain?: boolean } = $props();
</script>

<Card title="Quests" {plain}>
  {#snippet aside()}
    {#if quest.page && !plain}<span class="tabular">{quest.page[0]} tracked</span>{/if}
  {/snippet}

  {#each quest.quests as { title, objectives }}
    <div class="quest" class:dense={plain}>
      <h3 class="title">{title || "Untitled quest"}</h3>
      {#if objectives.length}
        <ul>
          {#each objectives as objective}
            {@const done = objective.current >= objective.required}
            <li class:done>
              <div class="row">
                <span>{objective.target}</span>
                <span class="count tabular">{objective.current} / {objective.required}</span>
              </div>
              <div class="track">
                <div class="fill" style:width="{(objective.current / objective.required) * 100}%"></div>
              </div>
            </li>
          {/each}
        </ul>
      {:else}
        <p class="empty">Counters could not be read.</p>
      {/if}
    </div>
  {:else}
    <p class="empty">The Quest Helper was found, but its text could not be read.</p>
  {/each}
</Card>

<style>
  .quest + .quest {
    margin-top: 12px;
    padding-top: 10px;
    border-top: 1px solid var(--border);
  }

  .title {
    margin: 0 0 8px;
    font-family: var(--font-display);
    font-size: 14px;
    font-weight: 600;
  }

  ul {
    display: grid;
    gap: 8px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .row {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 4px;
  }

  .count {
    flex: none;
    color: var(--muted);
  }

  .done .count {
    color: var(--good);
  }

  .track {
    height: 4px;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.06);
    overflow: hidden;
  }

  .fill {
    height: 100%;
    background: var(--accent);
  }

  .done .fill {
    background: var(--good);
  }

  /* Pinned: the same layout, packed tighter. */
  .dense .title {
    margin-bottom: 4px;
    font-size: 12.5px;
  }

  .dense ul {
    gap: 5px;
  }

  .dense .row {
    margin-bottom: 2px;
    font-size: 12px;
  }

  .dense .track {
    height: 3px;
  }

  .quest.dense + .quest.dense {
    margin-top: 8px;
    padding-top: 7px;
  }

  .empty {
    margin: 0;
    color: var(--faint);
    font-size: 12px;
  }
</style>
