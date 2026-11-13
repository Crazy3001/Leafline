<script lang="ts">
  // How a quest goes, step by step with directions, and what it pays and leads to. For a quest
  // being tracked, the steps start with its objectives, since it has already been taken.
  import Ref from "$lib/components/Ref.svelte";
  import ObjectiveHint from "./ObjectiveHint.svelte";
  import PlaceRoute from "./PlaceRoute.svelte";
  import { backend } from "$lib/api";
  import { formatMesos, formatNumber } from "$lib/format";
  import { showItemNamed } from "$lib/items.svelte";
  import { showMonsterNamed } from "$lib/monsters.svelte";
  import { showNpcNamed } from "$lib/places.svelte";
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
      <p class="step">Talk to <Ref kind="npc" name={info.giver} /></p>
      <PlaceRoute maps={info.giverMaps} {from} npc={info.giver} />
    </li>
  {/if}
  {#each sheet.objectives as objective (objective.guide.target)}
    {@const finishedStep = done(objective.guide.target)}
    <li class:done={finishedStep}>
      <p class="step">
        {objective.guide.kind === "monster" ? "Defeat" : "Bring"}
        {objective.count}
        {#if objective.guide.kind === "monster"}
          <Ref kind="mob" name={objective.guide.target} />
        {:else}
          <Ref kind="item" name={objective.guide.item ?? objective.guide.target}>{objective.guide.target}</Ref>
        {/if}
      </p>
      {#if !finishedStep}<ObjectiveHint guide={objective.guide} {from} />{/if}
    </li>
  {/each}
  <li class:next={finished}>
    {#if info.turnIn}
      <p class="step">Turn in to <Ref kind="npc" name={info.turnIn} /></p>
      <PlaceRoute maps={info.turnInMaps} {from} npc={info.turnIn} />
    {:else if info.giver}
      <p class="step">Turn it in</p>
      <p class="unsure">Nothing says to whom yet. It came from <Ref kind="npc" name={info.giver} />.</p>
      <PlaceRoute maps={info.giverMaps} {from} npc={info.giver} />
    {:else}
      <p class="step">Turn it in</p>
    {/if}
  </li>
</ol>

{#if sheet.row.exp || sheet.row.mesos || sheet.rewards.length}
  <h4>Pays</h4>
  <dl class="pay">
    {#if sheet.row.exp}<div><dt>EXP</dt><dd class="tabular exp">{formatNumber(sheet.row.exp)}</dd></div>{/if}
    {#if sheet.row.mesos}<div><dt>Mesos</dt><dd class="tabular mesos">{formatMesos(sheet.row.mesos)}</dd></div>{/if}
  </dl>
  {#if sheet.rewards.length}
    <ul class="links">
      {#each sheet.rewards as item (item.id)}
        <li>
          <Ref kind="item" id={item.id} name={item.name} />
          {#if item.count > 1}<span class="tabular">×{item.count}</span>{/if}
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
      <li><Ref kind="quest" id={quest.id} name={quest.name} /></li>
    {/each}
  </ul>
{/if}
{#if sheet.next}
  <h4>Leads to</h4>
  <ul class="links">
    <li><Ref kind="quest" id={sheet.next!.id} name={sheet.next.name} /></li>
  </ul>
{/if}

<style>
  .story {
    margin: 6px 0 0;
    color: var(--muted);
    font-size: var(--text-sm);
  }

  h4 {
    margin: 10px 0 4px;
    color: var(--muted);
    font-family: var(--font-display);
    font-size: var(--text-sm);
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
    font-size: var(--text-xs);
    font-variant-numeric: tabular-nums;
  }

  .step {
    margin: 0;
    font-size: var(--text-sm);
    font-weight: 600;
  }

  .done .step,
  .done .step :global(.ref) {
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
    font-size: var(--text-xs);
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
    font-size: var(--text-xs);
  }

  dd {
    margin: 0;
    font-weight: 600;
  }

  dd.exp {
    color: var(--exp);
  }

  dd.mesos {
    color: var(--mesos);
  }

  .links {
    margin: 0;
    padding: 0;
    list-style: none;
    color: var(--muted);
    font-size: var(--text-sm);
  }

  .links :global(.ref) {
    padding: 2px 0;
    font-weight: 600;
  }

  .maybe {
    margin-left: 4px;
    color: var(--faint);
    font-size: var(--text-xs);
  }
</style>
