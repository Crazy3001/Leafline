<script lang="ts">
  // How a quest goes, step by step with directions, and what it pays and leads to. For a quest
  // being tracked, the steps start with its objectives, since it has already been taken.
  import ObjectiveHint from "./ObjectiveHint.svelte";
  import PlaceRoute from "./PlaceRoute.svelte";
  import { backend } from "$lib/api";
  import { formatMesos, formatNumber } from "$lib/format";
  import type { MapRef, Objective, QuestSheet } from "$lib/types";

  let {
    sheet,
    from,
    tracked = null,
  }: {
    sheet: QuestSheet;
    /** The map the character is on, when known. */
    from: MapRef | null;
    /** What the Quest Helper counts, for a quest being tracked. */
    tracked?: Objective[] | null;
  } = $props();

  const info = $derived(sheet.info);

  function done(target: string) {
    const counted = tracked?.find((o) => o.target.toLowerCase() === target.toLowerCase());
    return counted != null && counted.current >= counted.required;
  }

  const finished = $derived(sheet.objectives.length > 0 && sheet.objectives.every((o) => done(o.guide.target)));
</script>

{#each sheet.story as line (line)}<p class="story">{line}</p>{/each}

<h4>Steps</h4>
<ol class="steps">
  {#if !tracked && info.giver}
    <li>
      <p class="step">Talk to {info.giver}</p>
      <PlaceRoute maps={info.giverMaps} {from} />
    </li>
  {/if}
  {#each sheet.objectives as objective (objective.guide.target)}
    {@const finishedStep = done(objective.guide.target)}
    <li class:done={finishedStep}>
      <p class="step">
        {objective.guide.kind === "monster" ? "Defeat" : "Bring"}
        {objective.count}
        {objective.guide.target}
      </p>
      {#if !finishedStep}<ObjectiveHint guide={objective.guide} {from} />{/if}
    </li>
  {/each}
  <li class:next={finished}>
    {#if info.turnIn}
      <p class="step">Turn in to {info.turnIn}</p>
      <PlaceRoute maps={info.turnInMaps} {from} />
    {:else if info.giver}
      <p class="step">Turn it in</p>
      <p class="unsure">The data doesn't say to whom. It came from {info.giver}.</p>
      <PlaceRoute maps={info.giverMaps} {from} />
    {:else}
      <p class="step">Turn it in</p>
    {/if}
  </li>
</ol>

{#if sheet.row.exp || sheet.row.mesos || sheet.rewards.length}
  <h4>Pays</h4>
  <dl class="pay">
    {#if sheet.row.exp}<div><dt>EXP</dt><dd class="tabular">{formatNumber(sheet.row.exp)}</dd></div>{/if}
    {#if sheet.row.mesos}<div><dt>Mesos</dt><dd class="tabular">{formatMesos(sheet.row.mesos)}</dd></div>{/if}
  </dl>
  {#if sheet.rewards.length}
    <ul class="links">
      {#each sheet.rewards as item (item.id)}
        <li>
          <button title="Show it in the Lookup tab" onclick={() => backend.showItem(item.id, null)}>{item.name}</button>
          {#if item.count > 1}<span class="tabular">× {item.count}</span>{/if}
          {#if !item.sure}<span class="maybe">possible</span>{/if}
        </li>
      {/each}
    </ul>
  {/if}
{/if}

{#if sheet.after.length}
  <h4>Comes after</h4>
  <ul class="links">
    {#each sheet.after as quest (quest.id)}
      <li><button onclick={() => backend.showQuest(quest.id)}>{quest.name}</button></li>
    {/each}
  </ul>
{/if}
{#if sheet.next}
  <h4>Leads to</h4>
  <ul class="links">
    <li><button onclick={() => backend.showQuest(sheet.next!.id)}>{sheet.next.name}</button></li>
  </ul>
{/if}

<style>
  .story {
    margin: 6px 0 0;
    color: var(--muted);
    font-size: 12px;
  }

  h4 {
    margin: 10px 0 4px;
    color: var(--muted);
    font-family: var(--font-display);
    font-size: 12px;
    font-weight: 400;
  }

  .steps {
    display: grid;
    gap: 6px;
    margin: 0;
    padding: 0;
    list-style: none;
    counter-reset: step;
  }

  .steps li {
    position: relative;
    padding-left: 18px;
    counter-increment: step;
  }

  .steps li::before {
    content: counter(step);
    position: absolute;
    left: 0;
    color: var(--faint);
    font-size: 11px;
    font-variant-numeric: tabular-nums;
  }

  .step {
    margin: 0;
    font-size: 12px;
    font-weight: 600;
  }

  .done .step {
    color: var(--faint);
    font-weight: 500;
    text-decoration: line-through;
  }

  .next .step {
    color: var(--good);
  }

  .unsure {
    margin: 1px 0 0;
    color: var(--faint);
    font-size: 11px;
  }

  .pay {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(90px, 1fr));
    gap: 6px;
    margin: 0 0 6px;
  }

  .pay div {
    padding: 4px 7px;
    border-radius: var(--radius-sm);
    background: rgba(255, 255, 255, 0.03);
  }

  dt {
    color: var(--faint);
    font-size: 10.5px;
  }

  dd {
    margin: 0;
    font-weight: 600;
  }

  .links {
    margin: 0;
    padding: 0;
    list-style: none;
    color: var(--muted);
    font-size: 12px;
  }

  .links button {
    padding: 2px 0;
    border: 0;
    background: transparent;
    font-size: 12px;
    font-weight: 600;
    text-align: left;
    cursor: pointer;
  }

  .links button:hover,
  .links button:focus-visible {
    color: var(--accent-strong);
    text-decoration: underline;
    outline: none;
  }

  .maybe {
    margin-left: 4px;
    color: var(--faint);
    font-size: 10.5px;
  }
</style>
