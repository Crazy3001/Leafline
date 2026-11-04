<script lang="ts">
  // The build plan's part of the Character tab: what the plan says at the character's level. It
  // moves on with the level, and catches up with what was really spent whenever the stat and
  // skill windows are read. The level itself is shown once, in the character's header above.
  import { ExternalLink, Pencil, RefreshCw } from "@lucide/svelte";
  import { untrack } from "svelte";

  import AdvancementChecklist from "./AdvancementChecklist.svelte";
  import GearPicks from "./GearPicks.svelte";
  import MapDataPanel from "./map/MapDataPanel.svelte";
  import Section from "./map/Section.svelte";
  import { backend } from "$lib/api";
  import { STATS, buildState, statLabel, watchBuilds } from "$lib/builds.svelte";
  import { OSMS_CREDIT, OSMS_URL, mapData, watchMapData } from "$lib/mapdata.svelte";
  import { display, keyHint } from "$lib/settings.svelte";
  import type { BuildPlan, LevelStep } from "$lib/types";

  let {
    level,
    character = null,
  }: {
    /** Read from the HUD or the session. */
    level: number | null;
    character?: string | null;
  } = $props();

  /** How many later levels with something to do are shown. */
  const COMING = 4;

  watchMapData();
  watchBuilds();

  const ready = $derived(mapData.status?.ready ?? false);

  let plan = $state<BuildPlan | null>(null);
  let loaded = $state(false);
  let error = $state<string | null>(null);

  const key = $derived(JSON.stringify([character, buildState.version, display.buildRules, ready]));

  // Follows the key alone: every live reading brings new objects with the same content.
  $effect(() => {
    const asked = key;
    if (!ready) {
      plan = null;
      return;
    }
    untrack(() => backend.activeBuild(character))
      .then((result) => {
        if (asked !== key) return;
        plan = result;
        error = null;
        loaded = true;
      })
      .catch((e) => {
        error = String(e);
        loaded = true;
      });
  });

  const steps = $derived(plan?.expansion.steps ?? []);
  const at = $derived(level ?? plan?.build.start.level ?? 0);
  const step = $derived(steps.findLast((s) => s.level <= at) ?? steps[0] ?? null);
  const hasWork = (s: LevelStep) => s.ap.length > 0 || s.sp.length > 0 || s.advance != null || s.gear.length > 0;
  const coming = $derived(steps.filter((s) => s.level > at && hasWork(s)).slice(0, COMING));
  const advance = $derived(steps.find((s) => s.level > at && s.advance));
  /** The advancement the checklist is for: the next one, or this level's until the next level. */
  const nextAdvance = $derived(steps.find((s) => s.level >= at && s.advance));
  const warnings = $derived(plan?.expansion.problems.filter((p) => p.severity === "warning") ?? []);
  const past = $derived(level != null && steps.length > 0 && level > steps[steps.length - 1].level);
</script>

<section class="build" aria-label="Build plan">
  <header class="head">
    <span class="title">Build</span>
    {#if plan && step}<span class="plan" title={plan.build.name}>{plan.build.name}</span>{/if}
    {#if plan && step}
      <span class="tools">
        <button
          class="link"
          title="Live reading follows your Character Stat and Skill windows while they're open. With it off, open them and read them once here{keyHint('ReadCharacter')}"
          onclick={() => backend.readCharacter()}><RefreshCw size={12} />Read stats</button
        >
        <button class="link" title="Edit this build" onclick={() => backend.showBuild()}><Pencil size={12} />Edit</button>
      </span>
    {/if}
  </header>

  {#if !ready}
    <p class="lead">Plan your AP and SP level by level. Builds use the skills and equipment in the map data.</p>
    <MapDataPanel compact />
  {:else if error}
    <p class="lead">{error}</p>
  {:else if loaded && !plan}
    <p class="lead">
      No build yet. Choose a job and write your goals in order, from a guide or your own idea, and Leafline lays them out
      level by level.
    </p>
    <button class="action" onclick={() => backend.showBuild()}><Pencil size={13} />Plan a build</button>
  {:else if plan && step}
    <div class="now">
      <span class="at tabular">Lv {level ?? step.level}</span>
      <div class="todo">
        {#if past}
          <p class="quiet">Past the end of this plan (Lv {step.level}).</p>
        {:else if !hasWork(step)}
          <p class="quiet">Nothing to spend at this level.</p>
        {/if}
        {#if !past}
          {#if step.advance}<p class="advance">{step.advance}</p>{/if}
          {#each step.sp as gain (gain.skillId)}
            <p>
              <span class="tag">{gain.to - gain.from} SP</span>{gain.name}
              <span class="tabular dim">{gain.from} → {gain.to}</span>
            </p>
          {/each}
          {#each step.ap as gain (gain.stat)}
            <p><span class="tag">{gain.amount} AP</span>{statLabel(gain.stat)} <span class="tabular dim">to {gain.total}</span></p>
          {/each}
          {#each step.gear as name (name)}
            <p><span class="tag">Gear</span>{name}</p>
          {/each}
        {/if}
      </div>
    </div>

    <dl class="stats" title="Base stats the plan has at this level. Compare them with your Character Stat window.">
      {#each STATS as stat (stat.id)}
        <div>
          <dt>{stat.label}</dt>
          <dd class="tabular">{step.stats[stat.id]}</dd>
        </div>
      {/each}
    </dl>

    {#if advance}
      <p class="next">
        Next job advancement: <strong>{advance.advance}</strong> at Lv {advance.level}
      </p>
    {/if}
    {#if nextAdvance?.advance && plan.build.plan.job}
      <Section title="Job advancement checklist">
        <AdvancementChecklist
          job={plan.build.plan.job}
          tier={nextAdvance.advance.startsWith("1st") ? 1 : 2}
          at={nextAdvance.level}
          {level}
          {character}
        />
      </Section>
    {/if}

    {#if coming.length}
      <Section title="Coming up" count={coming.length}>
        <ul class="coming">
          {#each coming as s (s.level)}
            <li>
              <span class="when tabular">Lv {s.level}</span>
              <span class="what">
                {#if s.advance}<span class="advance">{s.advance}</span>{/if}
                {#each s.sp as gain (gain.skillId)}<span>{gain.name} {gain.to}</span>{/each}
                {#each s.ap as gain (gain.stat)}<span class="tabular">{statLabel(gain.stat)} {gain.total}</span>{/each}
                {#each s.gear as name (name)}<span>{name}</span>{/each}
              </span>
            </li>
          {/each}
        </ul>
      </Section>
    {/if}

    {#if plan.expansion.gear.length}
      <Section title="Gear goals" count={plan.expansion.gear.length}>
        <ul class="goals">
          {#each plan.expansion.gear as check (check.item.id)}
            <li>
              <button title="Show it in the Lookup tab" onclick={() => backend.showItem(check.item.id, null)}
                >{check.item.name}</button
              >
              <span class="tabular" class:warn={!check.usable || check.level == null}>
                {#if !check.usable}Wrong job{:else if check.level != null}Lv {check.level}{:else}Out of reach{/if}
              </span>
            </li>
          {/each}
        </ul>
      </Section>
    {/if}
    <Section title="Gear to look for" open={false}>
      <GearPicks build={plan.build.id} level={at} />
    </Section>

    {#if warnings.length}
      <h4 class="warn-title">{warnings.length === 1 ? "1 thing to fix" : `${warnings.length} things to fix`}</h4>
      <ul class="warnings">
        {#each warnings.slice(0, 3) as problem (problem.message)}<li>{problem.message}</li>{/each}
      </ul>
      {#if warnings.length > 3}<button class="link" onclick={() => backend.showBuild()}>See all in the editor</button>{/if}
    {/if}

    <footer>
      {#if plan.build.guide}
        <button class="credit" onclick={() => backend.openGuide(plan!.build.guide)}><ExternalLink size={11} />Open guide</button>
      {/if}
      <button class="credit" onclick={() => backend.openLink(OSMS_URL)}>{OSMS_CREDIT}</button>
    </footer>
  {/if}
</section>

<style>
  .build {
    margin-top: 12px;
    padding-top: 10px;
    border-top: 1px solid var(--border);
  }

  .head {
    display: flex;
    align-items: baseline;
    gap: 8px;
    margin-bottom: 8px;
  }

  .title {
    flex: none;
    font-family: var(--font-display);
    font-size: 12.5px;
    color: var(--muted);
  }

  .plan {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    font-family: var(--font-display);
    font-size: 13px;
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tools {
    display: inline-flex;
    flex: none;
    gap: 10px;
    margin-left: auto;
  }

  .lead {
    margin: 0 0 10px;
    color: var(--muted);
    font-size: 12px;
  }

  .action {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 5px 10px;
    border: 1px solid color-mix(in srgb, var(--accent) 45%, transparent);
    border-radius: var(--radius-sm);
    background: var(--accent-soft);
    color: var(--accent-strong);
    font-size: 12px;
    cursor: pointer;
  }

  .action:hover {
    filter: brightness(1.2);
  }

  .link {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--muted);
    font-size: 11.5px;
    cursor: pointer;
  }

  .link:hover,
  .link:focus-visible {
    color: var(--text);
    outline: none;
  }

  /* The level's to-do list, set against the level like a row of the plan. */
  .now {
    display: flex;
    gap: 10px;
    align-items: baseline;
  }

  .at {
    flex: none;
    width: 42px;
    color: var(--accent);
    font-family: var(--font-display);
    font-size: 12.5px;
    font-weight: 600;
  }

  .todo {
    flex: 1;
    min-width: 0;
  }

  .todo p {
    display: flex;
    align-items: baseline;
    gap: 6px;
    margin: 0 0 3px;
    font-size: 12.5px;
  }

  .todo .quiet {
    color: var(--faint);
  }

  .tag {
    flex: none;
    width: 38px;
    color: var(--accent);
    font-size: 11px;
    font-weight: 600;
  }

  .dim {
    color: var(--muted);
    white-space: nowrap;
  }

  .advance {
    color: var(--accent-strong);
    font-weight: 600;
  }

  .stats {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 6px;
    margin: 10px 0 0;
  }

  .stats div {
    padding: 5px 8px;
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

  .next {
    margin: 10px 0 0;
    color: var(--muted);
    font-size: 12px;
  }

  .next strong {
    color: var(--text);
    font-weight: 600;
  }

  h4 {
    margin: 12px 0 4px;
    font-family: var(--font-display);
    font-size: 12px;
    font-weight: 400;
  }

  .warn-title {
    color: var(--warn);
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
    font-size: 12px;
  }

  .coming li {
    display: flex;
    gap: 10px;
    padding: 3px 0;
  }

  .coming li + li {
    border-top: 1px solid var(--border);
  }

  .when {
    flex: none;
    width: 42px;
    color: var(--faint);
  }

  .what {
    display: flex;
    flex-wrap: wrap;
    gap: 2px 10px;
    min-width: 0;
  }

  .goals li {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    padding: 2px 0;
  }

  .goals button {
    padding: 0;
    border: 0;
    background: transparent;
    font-size: 12px;
    font-weight: 600;
    text-align: left;
    cursor: pointer;
  }

  .goals button:hover,
  .goals button:focus-visible {
    color: var(--accent-strong);
    text-decoration: underline;
    outline: none;
  }

  .goals span {
    flex: none;
    color: var(--muted);
  }

  .goals .warn {
    color: var(--warn);
  }

  .warnings li {
    padding: 3px 0 3px 8px;
    border-left: 2px solid var(--warn);
    color: var(--muted);
  }

  .warnings li + li {
    margin-top: 4px;
  }

  footer {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 12px;
    margin-top: 12px;
  }

  .credit {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--faint);
    font-size: 10.5px;
    text-align: left;
    cursor: pointer;
  }

  .credit:hover,
  .credit:focus-visible {
    color: var(--muted);
    outline: none;
  }
</style>
