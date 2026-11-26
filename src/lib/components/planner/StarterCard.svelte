<script lang="ts">
  // One ready-made plan at a glance: a line on what it does, the Lv 1 roll, the skills it builds
  // on, its weapons, and for a plan of Leafline's own what it does well and gives up. The guide's
  // full credit sits on the link's tooltip, so the card stays short.
  import { ExternalLink } from "@lucide/svelte";
  import type { Snippet } from "svelte";

  import { backend } from "$lib/api";
  import { STATS } from "$lib/builds.svelte";
  import { tip } from "$lib/tip.svelte";
  import Pic from "$lib/ui/Pic.svelte";
  import type { StarterPlan } from "$lib/types";

  let {
    plan,
    jobName,
    selected = false,
    onpick,
    action,
  }: {
    plan: StarterPlan;
    /** The second job's name, for the guide link. */
    jobName: string;
    selected?: boolean;
    /** Makes the card a choice. */
    onpick?: () => void;
    /** A button beside the credit, such as Use this plan. */
    action?: Snippet;
  } = $props();

  /** The stat that takes whatever AP is left: the plan's main stat. */
  const main = $derived(plan.plan.stats.find((s) => s.target == null)?.stat ?? null);
  const guide = $derived(plan.source ?? plan.reading);
  const firstWeapon = $derived(plan.weapons[0]?.level);
  const lastWeapon = $derived(plan.weapons.at(-1)?.level);
</script>

{#snippet body()}
  <span class="head">
    <span class="name">{plan.name}</span>
    <span class="badge warn" use:tip={"Nobody has played this plan yet: a place to start, to change as you go"}>Untested</span>
  </span>
  <span class="summary">{plan.summary}</span>

  <span class="line">
    <span class="label">Lv 1</span>
    <span class="chips">
      {#each STATS as stat (stat.id)}
        <span class="stat" class:main={stat.id === main}>{stat.label} <b>{plan.start[stat.id]}</b></span>
      {/each}
    </span>
  </span>

  {#if plan.keySkills.length}
    <span class="line">
      <span class="label">Skills</span>
      <span class="chips">
        {#each plan.keySkills as skill (skill.id)}
          <span class="skill"><Pic kind="skill" id={skill.id} size={18} />{skill.name} <b>{skill.level}</b></span>
        {/each}
      </span>
    </span>
  {/if}

  {#if plan.weapons.length}
    <span class="line">
      <span class="label">
        Weapons
        <span class="range">Lv {firstWeapon}{lastWeapon !== firstWeapon ? `–${lastWeapon}` : ""}</span>
      </span>
      <span class="weapons">
        {#each plan.weapons as weapon (weapon.id)}
          <span class="weapon" use:tip={`${weapon.name}, Lv ${weapon.level}`}>
            <Pic kind="item" id={weapon.id} size={24}>
              {#snippet fallback()}<span class="no-pic"></span>{/snippet}
            </Pic>
          </span>
        {/each}
      </span>
    </span>
  {/if}

  {#if plan.pros.length || plan.cons.length}
    <span class="tradeoffs">
      {#each plan.pros as pro (pro)}<span class="pro">{pro}</span>{/each}
      {#each plan.cons as con (con)}<span class="con">{con}</span>{/each}
    </span>
  {/if}

  {#if plan.missing.length}<span class="warn-text">Not in this data: {plan.missing.join(", ")}</span>{/if}
{/snippet}

<article class="card" class:on={selected}>
  {#if onpick}
    <button class="body pick" aria-pressed={selected} onclick={onpick}>{@render body()}</button>
  {:else}
    <div class="body">{@render body()}</div>
  {/if}
  <footer>
    {#if !plan.source}<span class="faint">Leafline's plan</span>{/if}
    {#if guide}
      <button class="credit" use:tip={guide.credit} onclick={() => backend.openGuide(guide.url)}>
        {plan.source ? `From ${guide.name}'s ${jobName} guide` : `Read ${guide.name}'s guide`}
        <ExternalLink size={12} />
      </button>
    {/if}
    {#if action}<span class="action">{@render action()}</span>{/if}
  </footer>
</article>

<style>
  .card {
    display: grid;
    grid-template-rows: 1fr auto;
    min-width: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    transition: border-color 0.12s;
  }

  .card:hover {
    border-color: var(--border-strong);
  }

  .card.on {
    border-color: var(--accent);
    background: var(--accent-soft);
  }

  .body {
    display: grid;
    align-content: start;
    gap: var(--space-2);
    min-width: 0;
    padding: var(--space-3) var(--space-4);
    color: var(--text);
    text-align: left;
  }

  .pick {
    border: 0;
    border-radius: var(--radius) var(--radius) 0 0;
    background: transparent;
    font: inherit;
    cursor: pointer;
  }

  .pick:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }

  .head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--space-2);
  }

  .name {
    font-weight: var(--weight-strong);
  }

  .card.on .name {
    color: var(--accent-text-strong);
  }

  .summary {
    color: var(--muted);
  }

  .card.on .summary {
    color: var(--text);
  }

  .line {
    display: grid;
    grid-template-columns: 56px 1fr;
    align-items: center;
    gap: var(--space-2);
  }

  .label {
    color: var(--faint);
    font-size: var(--text-xs);
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1);
  }

  .stat,
  .skill {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    padding: 1px var(--space-1-5);
    border-radius: var(--radius-sm);
    background: var(--hover);
    color: var(--muted);
    font-size: var(--text-xs);
    white-space: nowrap;
  }

  .stat b,
  .skill b {
    color: var(--text);
    font-variant-numeric: tabular-nums;
  }

  .stat.main {
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .stat.main b {
    color: var(--accent-text-strong);
  }

  .weapons {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-0-5);
  }

  .weapon {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border-radius: var(--radius-sm);
    background: var(--hover);
  }

  .no-pic {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--faint);
  }

  .range {
    display: block;
    font-size: var(--text-2xs);
    font-variant-numeric: tabular-nums;
  }

  .tradeoffs {
    display: grid;
    gap: var(--space-1);
    font-size: var(--text-md);
  }

  .pro,
  .con {
    position: relative;
    padding-left: var(--space-4);
    color: var(--muted);
  }

  .card.on .pro,
  .card.on .con {
    color: var(--text);
  }

  .pro::before,
  .con::before {
    position: absolute;
    left: 0;
    font-weight: var(--weight-strong);
  }

  .pro::before {
    content: "+";
    color: var(--good);
  }

  .con::before {
    content: "−";
    color: var(--warn);
  }

  .warn-text {
    color: var(--warn);
    font-size: var(--text-md);
  }

  footer {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-1) var(--space-2);
    min-height: 36px;
    padding: var(--space-1) var(--space-4);
    border-top: 1px solid var(--border);
    font-size: var(--text-xs);
  }

  .credit {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--muted);
    font: inherit;
    white-space: nowrap;
    cursor: pointer;
  }

  .credit:hover {
    color: var(--accent-text);
    text-decoration: underline;
  }

  .faint {
    color: var(--faint);
    white-space: nowrap;
  }

  .action {
    margin-left: auto;
  }
</style>
