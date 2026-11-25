<script lang="ts">
  // One level of the plan, picked on the chart: what to spend its SP and AP on, the stats after
  // it, what each skill's new level does, and gear it first lets the character wear.
  import { ChevronLeft, ChevronRight, TriangleAlert } from "@lucide/svelte";

  import Ref from "$lib/components/Ref.svelte";
  import { tip } from "$lib/tip.svelte";
  import Pic from "$lib/ui/Pic.svelte";

  import { STATS, statLabel } from "$lib/builds.svelte";
  import { STAT_COLOURS, stepAt } from "$lib/planner";
  import type { BaseStats, BuildProblem, LevelStep, SkillInfo } from "$lib/types";

  let {
    steps,
    selected,
    start,
    end,
    level,
    startStats,
    problems,
    skillOf,
    onselect,
  }: {
    steps: LevelStep[];
    selected: number;
    start: number;
    end: number;
    /** The character's level, read from the game. */
    level: number | null;
    startStats: BaseStats | null;
    problems: BuildProblem[];
    skillOf: (id: number) => SkillInfo | undefined;
    onselect: (level: number) => void;
  } = $props();

  const step = $derived(steps.find((s) => s.level === selected) ?? null);
  const before = $derived(selected > start ? stepAt(steps, selected - 1) : null);
  const stats = $derived(step?.stats ?? (selected === start ? startStats : stepAt(steps, selected)?.stats) ?? null);
  const priorStats = $derived(before?.stats ?? (selected - 1 === start ? startStats : null));
  const next = $derived(level != null ? Math.min(end, Math.max(start, level + 1)) : null);
  const here = $derived(problems.filter((p) => p.level === selected));

  const heading = $derived.by(() => {
    if (level == null) return selected === start ? "Where the plan starts" : "On the plan";
    if (selected === level + 1) return "Your next level";
    if (selected === level) return "Your level now";
    return selected < level ? `Done ${level - selected} level${level - selected === 1 ? "" : "s"} ago` : `${selected - level} levels ahead`;
  });

  const spTotal = $derived(step?.sp.reduce((sum, g) => sum + (g.to - g.from), 0) ?? 0);
  const apTotal = $derived(step?.ap.reduce((sum, g) => sum + g.amount, 0) ?? 0);
</script>

<section class="inspector" aria-label="Level {selected}">
  <div class="top">
    <div>
      <p class="kicker">{heading}</p>
      <h2 class="tabular">Level {selected}</h2>
    </div>
    <span class="steps">
      <button class="icon" use:tip={"Level before"} aria-label="Level before" disabled={selected <= start} onclick={() => onselect(selected - 1)}
        ><ChevronLeft size={14} /></button
      >
      <button class="icon" use:tip={"Level after"} aria-label="Level after" disabled={selected >= end} onclick={() => onselect(selected + 1)}
        ><ChevronRight size={14} /></button
      >
    </span>
  </div>
  {#if next != null && selected !== next}
    <button class="back" onclick={() => onselect(next)}>Back to your next level, {next}</button>
  {/if}

  {#if step?.advance}<p class="advance">{step.advance}</p>{/if}

  {#each here as p (p.message)}
    <p class="warning"><TriangleAlert size={12} /><span>{p.message}</span></p>
  {/each}

  {#if step && (spTotal || apTotal || step.spLeft || step.apLeft)}
    <div class="spend">
      <div class="line">
        <span class="what tabular">{spTotal} SP</span>
        <span class="into">
          {#each step.sp as gain (gain.skillId)}
            <span class="gain"><Ref kind="skill" id={gain.skillId} name={gain.name} /><span class="tabular amount">+{gain.to - gain.from}</span></span>
          {:else}
            <span class="faint">none</span>
          {/each}
        </span>
      </div>
      <div class="line">
        <span class="what tabular">{apTotal} AP</span>
        <span class="into">
          {#each step.ap as gain (gain.stat)}
            <span class="gain"><i style:background={STAT_COLOURS[gain.stat]}></i>+{gain.amount} {statLabel(gain.stat)}</span>
          {:else}
            <span class="faint">none</span>
          {/each}
        </span>
      </div>
      {#if step.spLeft || step.apLeft}
        <p class="left">
          {[step.spLeft ? `${step.spLeft} SP` : "", step.apLeft ? `${step.apLeft} AP` : ""].filter(Boolean).join(" and ")} not spent by this level: no goal
          can take {step.spLeft + step.apLeft === 1 ? "it" : "them"} yet.
        </p>
      {/if}
    </div>
  {:else if selected === start}
    <p class="quiet">The plan starts here, from the stats and skills in Settings. Points start going in at Lv {Math.min(end, start + 1)}.</p>
  {/if}

  {#if stats}
    <div>
      <p class="sub">Stats at Lv {selected}</p>
      <div class="stats">
        {#each STATS as stat (stat.id)}
          {@const gain = priorStats ? stats[stat.id] - priorStats[stat.id] : 0}
          <div class="stat" class:up={gain > 0} style:--stat={STAT_COLOURS[stat.id]}>
            <span class="name">{stat.label}</span>
            <span class="value tabular">{stats[stat.id]}{#if gain > 0}<small>+{gain}</small>{/if}</span>
          </div>
        {/each}
      </div>
    </div>
  {/if}

  {#if step?.sp.length}
    <div>
      <p class="sub">What the new skill levels do</p>
      <ul class="skills">
        {#each step.sp as gain (gain.skillId)}
          {@const skill = skillOf(gain.skillId)}
          <li>
            <Pic kind="skill" id={gain.skillId} size={32}>
              {#snippet fallback()}<span class="no-pic"></span>{/snippet}
            </Pic>
            <div>
              <p class="skill-name"><Ref kind="skill" id={gain.skillId} name={gain.name} icon={false} /> <span class="tabular faint">{gain.from} › {gain.to}{skill ? ` of ${skill.maxLevel}` : ""}</span></p>
              {#if skill?.levelDescriptions[gain.to - 1]}<p class="desc">{skill.levelDescriptions[gain.to - 1]}</p>{/if}
            </div>
          </li>
        {/each}
      </ul>
    </div>
  {/if}

  {#if step?.gear.length}
    <div>
      <p class="sub">Gear you can first wear</p>
      <ul class="gear">
        {#each step.gear as name (name)}<li><Ref kind="item" {name} /></li>{/each}
      </ul>
    </div>
  {/if}

  <p class="foot">The planner counts points; it doesn't work out damage.</p>
</section>

<style>
  .inspector {
    display: grid;
    gap: 12px;
    padding-bottom: 14px;
    margin-bottom: 12px;
    border-bottom: 1px solid var(--border);
  }

  .top {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 8px;
  }

  .kicker,
  .sub {
    margin: 0;
    color: var(--faint);
    font-size: var(--text-xs);
    font-weight: var(--weight-body);
  }

  .sub {
    margin-bottom: 6px;
  }

  h2 {
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--text-2xl);
    font-weight: var(--weight-strong);
    line-height: 1.15;
  }

  .steps {
    display: inline-flex;
    gap: 2px;
  }

  .icon {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    color: var(--muted);
    cursor: pointer;
  }

  .icon:hover:not(:disabled) {
    color: var(--text);
    border-color: var(--border-strong);
  }

  .icon:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .back {
    justify-self: start;
    margin-top: -6px;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--accent-text);
    font-size: var(--text-md);
    cursor: pointer;
  }

  .back:hover {
    text-decoration: underline;
  }

  .advance {
    margin: 0;
    padding: 6px 10px;
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--ref-skill) 14%, var(--surface));
    color: var(--ref-skill);
    font-weight: var(--weight-strong);
  }

  .warning {
    display: flex;
    gap: 6px;
    margin: 0;
    color: var(--warn);
    font-size: var(--text-md);
  }

  .warning :global(svg) {
    flex: none;
    margin-top: 3px;
  }

  .spend {
    display: grid;
    gap: 8px;
    padding: 10px 12px;
    border: 1px solid color-mix(in srgb, var(--accent) 40%, transparent);
    border-radius: var(--radius);
    background: var(--accent-soft);
  }

  .line {
    display: grid;
    grid-template-columns: 52px minmax(0, 1fr);
    gap: 8px;
    align-items: baseline;
  }

  .what {
    font-family: var(--font-display);
    font-weight: var(--weight-strong);
  }

  .into {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 2px 10px;
    text-align: right;
  }

  .gain {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-weight: var(--weight-body);
  }

  .gain i {
    display: inline-block;
    width: 8px;
    height: 8px;
    border-radius: 2px;
  }

  .amount {
    color: var(--text);
  }

  .left {
    margin: 0;
    color: var(--warn);
    font-size: var(--text-xs);
  }

  .quiet,
  .faint {
    color: var(--faint);
  }

  .quiet {
    margin: 0;
    font-size: var(--text-md);
  }

  .stats {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 6px;
  }

  .stat {
    display: grid;
    gap: 1px;
    padding: 7px 8px;
    border-radius: var(--radius-sm);
    background: var(--surface-2);
  }

  .stat.up {
    background: color-mix(in srgb, var(--stat) 14%, var(--surface));
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--stat) 40%, transparent);
  }

  .stat .name {
    color: var(--faint);
    font-size: var(--text-xs);
  }

  .stat .value {
    font-family: var(--font-display);
    font-size: var(--text-lg);
    font-weight: var(--weight-strong);
  }

  .stat small {
    margin-left: 3px;
    color: var(--good);
    font-size: var(--text-xs);
  }

  .skills,
  .gear {
    display: grid;
    gap: 8px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .skills li {
    display: grid;
    grid-template-columns: 32px minmax(0, 1fr);
    gap: 10px;
    padding: 8px 10px;
    border-radius: var(--radius-sm);
    background: var(--surface-2);
  }

  .no-pic {
    display: block;
    width: 32px;
    height: 32px;
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--ref-skill) 16%, var(--surface));
  }

  .skill-name,
  .desc {
    margin: 0;
  }

  .skill-name {
    font-weight: var(--weight-strong);
  }

  .desc {
    margin-top: 2px;
    color: var(--muted);
    font-size: var(--text-md);
    line-height: 1.4;
  }

  .gear {
    gap: 4px;
  }

  .foot {
    margin: 0;
    color: var(--faint);
    font-size: var(--text-xs);
  }
</style>
