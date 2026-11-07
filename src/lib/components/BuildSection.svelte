<script lang="ts">
  // The build plan's part of the Character tab: what the plan says at the character's level. It
  // moves on with the level, and catches up with what was really spent whenever the stat and
  // skill windows are read. The level itself is shown once, in the character's header above.
  import { Check, ExternalLink, Pencil, Plus, RefreshCw, Search, X } from "@lucide/svelte";
  import { untrack } from "svelte";

  import AdvancementChecklist from "./AdvancementChecklist.svelte";
  import GearPicks from "./GearPicks.svelte";
  import MapDataPanel from "./map/MapDataPanel.svelte";
  import { backend } from "$lib/api";
  import { STATS, buildState, statLabel, watchBuilds } from "$lib/builds.svelte";
  import { OSMS_CREDIT, OSMS_URL, mapData, watchMapData } from "$lib/mapdata.svelte";
  import { display, keyHint } from "$lib/settings.svelte";
  import type { BuildPlan, GearItem, LevelStep } from "$lib/types";
  import Segmented from "$lib/ui/Segmented.svelte";

  let {
    level,
    character = null,
    glance = false,
  }: {
    /** Read from the HUD or the session. */
    level: number | null;
    character?: string | null;
    /** A pinned window's short form: this level's to-do and the next advancement, nothing to browse. */
    glance?: boolean;
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

  // The plan, the next job advancement and gear each get a view of their own, so the tab stays
  // short. The one last picked is remembered.
  type View = "plan" | "advance" | "gear";
  const VIEW_KEY = "leafline.build.view";
  let view = $state<View>(storedView());
  const canAdvance = $derived(!!nextAdvance?.advance && !!plan?.build.plan.job);
  const views = $derived([
    { id: "plan" as View, label: "Plan" },
    ...(canAdvance ? [{ id: "advance" as View, label: "Advancement" }] : []),
    { id: "gear" as View, label: "My build" },
  ]);

  // My build's equipment: added from the search here, the suggestions, or Lookup, and taken out here.
  const SEARCH_DELAY = 120;
  const FOUND_SHOWN = 6;
  let gearText = $state("");
  let gearFound = $state<GearItem[]>([]);
  /** The text the shown results answer, so "nothing found" waits for the answer. */
  let searched = $state("");
  let goalError = $state<string | null>(null);

  $effect(() => {
    const text = gearText;
    const job = plan?.build.plan.job ?? 0;
    if (text.trim().length < 2) {
      gearFound = [];
      searched = "";
      return;
    }
    const timer = setTimeout(() => {
      backend
        .searchGear(text, job)
        .then((found) => {
          if (text !== gearText) return;
          gearFound = found.slice(0, FOUND_SHOWN);
          searched = text;
        })
        .catch(() => {});
    }, SEARCH_DELAY);
    return () => clearTimeout(timer);
  });

  async function setGoal(id: number, wanted: boolean) {
    try {
      await backend.setGearGoal(id, wanted, character);
      goalError = null;
    } catch (e) {
      goalError = String(e);
    }
  }
  const shown = $derived(view === "advance" && !canAdvance ? "plan" : view);

  function storedView(): View {
    try {
      const saved = localStorage.getItem(VIEW_KEY);
      return saved === "advance" || saved === "gear" ? saved : "plan";
    } catch {
      return "plan";
    }
  }

  function pickView(next: View) {
    view = next;
    try {
      localStorage.setItem(VIEW_KEY, next);
    } catch {
      // A convenience only.
    }
  }
</script>

{#snippet todo(step: LevelStep)}
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
          <p><span class="tag">Gear</span><button class="ref ref-item" onclick={() => backend.showItem(null, name)}>{name}</button></p>
        {/each}
      {/if}
    </div>
  </div>
{/snippet}

{#if glance}
  <!-- Nothing to set up or browse in a pinned window: without a plan, it stays out of the way. -->
  {#if ready && plan && step}
    <section class="build" aria-label="Build plan">
      <header class="head">
        <span class="title">Build</span>
        <span class="plan" title={plan.build.name}>{plan.build.name}</span>
      </header>
      {@render todo(step)}
      {#if advance}
        <p class="next">
          Next job advancement: <strong>{advance.advance}</strong> at Lv {advance.level}
        </p>
      {/if}
      {#if warnings.length}
        <button class="link warn-line" onclick={() => backend.showBuild()}>
          {warnings.length === 1 ? "1 thing to fix" : `${warnings.length} things to fix`} in this plan
        </button>
      {/if}
      <footer>
        <button class="credit" onclick={() => backend.openLink(OSMS_URL)}>{OSMS_CREDIT}</button>
      </footer>
    </section>
  {/if}
{:else}
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
        <button class="link" onclick={() => backend.showBuild()}><Pencil size={12} />Edit</button>
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
    <div class="views"><Segmented label="Build view" options={views} value={shown} stretch size="sm" onchange={pickView} /></div>
    {#if shown === "plan"}
    {@render todo(step)}

    <dl class="stats">
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

    {#if coming.length}
      <h4 class="sub">Coming up</h4>
        <ul class="coming">
          {#each coming as s (s.level)}
            <li>
              <span class="when tabular">Lv {s.level}</span>
              <span class="what">
                {#if s.advance}<span class="advance">{s.advance}</span>{/if}
                <!-- What the level adds, then the total it makes: "Double Stab +3 (4)". -->
                {#each s.sp as gain (gain.skillId)}<span>{gain.name} <span class="tabular">+{gain.to - gain.from}</span> <span class="total tabular">({gain.to})</span></span>{/each}
                {#each s.ap as gain (gain.stat)}<span>{statLabel(gain.stat)} <span class="tabular">+{gain.amount}</span> <span class="total tabular">({gain.total})</span></span>{/each}
                {#each s.gear as name (name)}<button class="ref ref-item" onclick={() => backend.showItem(null, name)}>{name}</button>{/each}
              </span>
            </li>
          {/each}
        </ul>
    {/if}

    {#if warnings.length}
      <h4 class="warn-title">{warnings.length === 1 ? "1 thing to fix" : `${warnings.length} things to fix`}</h4>
      <ul class="warnings">
        {#each warnings.slice(0, 3) as problem (problem.message)}<li>{problem.message}</li>{/each}
      </ul>
      {#if warnings.length > 3}<button class="link" onclick={() => backend.showBuild()}>See all in the editor</button>{/if}
    {/if}
    {:else if shown === "advance" && nextAdvance?.advance && plan.build.plan.job}
      <p class="next">
        <strong>{nextAdvance.advance}</strong> at Lv {nextAdvance.level}
      </p>
      <AdvancementChecklist
        job={plan.build.plan.job}
        tier={nextAdvance.advance.startsWith("1st") ? 1 : 2}
        at={nextAdvance.level}
        {level}
        {character}
      />
    {:else}
    <!-- My build: the equipment the plan aims for, the same list "Add to my build" in Lookup adds to. -->
    {#if plan.expansion.gear.length}
      <ul class="goals">
        {#each plan.expansion.gear as check (check.item.id)}
          <li>
            <span class="goal-name">
              <button class="ref ref-item" onclick={() => backend.showItem(check.item.id, null)}>{check.item.name}</button>
              <span class="slot">{check.item.slot}</span>
            </span>
            <span class="tabular when" class:warn={!check.usable || check.level == null}>
              {#if !check.usable}Wrong job{:else if check.level != null}Lv {check.level}{:else}Out of reach{/if}
            </span>
            <button
              class="remove"
              title="Take it out of my build"
              aria-label="Take {check.item.name} out of my build"
              onclick={() => setGoal(check.item.id, false)}><X size={12} /></button
            >
          </li>
        {/each}
      </ul>
    {:else}
      <p class="quiet-line">No equipment yet. Add some below, or with <strong>Add to my build</strong> on an item in Lookup.</p>
    {/if}
    {#if goalError}<p class="goal-error">{goalError}</p>{/if}

    <label class="find">
      <Search size={13} />
      <input type="search" placeholder="Add equipment: type a name" aria-label="Find equipment to add" bind:value={gearText} />
    </label>
    {#if gearFound.length}
      <ul class="found">
        {#each gearFound as item (item.id)}
          {@const added = plan.build.plan.gear.includes(item.id)}
          <li>
            <button disabled={added} onclick={() => setGoal(item.id, true)} title={added ? "Already in my build" : "Add to my build"}>
              <span class="found-name">{item.name}</span>
              <small>{item.slot}{item.reqLevel ? ` · Lv ${item.reqLevel}` : ""}</small>
              {#if added}<Check size={12} />{:else}<Plus size={12} />{/if}
            </button>
          </li>
        {/each}
      </ul>
    {:else if gearText.trim().length >= 2 && searched === gearText}
      <p class="quiet-line">No equipment by that name for this job.</p>
    {/if}

    <h4 class="sub">Gear to look for</h4>
    <GearPicks build={plan.build.id} level={at} onAdd={(item) => setGoal(item.id, true)} />
    {/if}

    <footer>
      {#if plan.build.guide}
        <button class="credit" onclick={() => backend.openGuide(plan!.build.guide)}><ExternalLink size={11} />Open guide</button>
      {/if}
      <button class="credit" onclick={() => backend.openLink(OSMS_URL)}>{OSMS_CREDIT}</button>
    </footer>
  {/if}
</section>
{/if}

<style>
  .link.warn-line {
    margin-top: 8px;
    color: var(--warn);
  }

  .views {
    display: flex;
    margin: 8px 0 10px;
  }

  .sub {
    margin: 12px 0 6px;
    font-family: var(--font-display);
    font-size: var(--text-sm);
    font-weight: 400;
    color: var(--muted);
  }

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
    font-size: var(--text-md);
    color: var(--muted);
  }

  .plan {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    font-family: var(--font-display);
    font-size: var(--text-md);
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
    font-size: var(--text-sm);
  }

  .action {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: var(--control-h);
    padding: 0 10px;
    border: 1px solid var(--accent);
    border-radius: var(--radius-sm);
    background: var(--accent);
    color: var(--bg);
    font-size: var(--text-sm);
    font-weight: 600;
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
    font-size: var(--text-sm);
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
    color: var(--muted);
    font-family: var(--font-display);
    font-size: var(--text-md);
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
    font-size: var(--text-md);
  }

  .todo .quiet {
    color: var(--faint);
  }

  .tag {
    flex: none;
    width: 38px;
    color: var(--muted);
    font-size: var(--text-xs);
    font-weight: 600;
  }

  .dim {
    color: var(--muted);
    white-space: nowrap;
  }

  .advance {
    color: var(--text);
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
    font-size: var(--text-xs);
  }

  dd {
    margin: 0;
    font-weight: 600;
  }

  .next {
    margin: 10px 0 0;
    color: var(--muted);
    font-size: var(--text-sm);
  }

  .next strong {
    color: var(--text);
    font-weight: 600;
  }

  h4 {
    margin: 12px 0 4px;
    font-family: var(--font-display);
    font-size: var(--text-sm);
    font-weight: 400;
  }

  .warn-title {
    color: var(--warn);
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
    font-size: var(--text-sm);
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
    gap: 2px 14px;
    min-width: 0;
  }

  .what .total {
    color: var(--muted);
  }

  .goals li {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 3px 0;
  }

  .goals li + li {
    border-top: 1px solid var(--border);
  }

  .goals .goal-name {
    display: flex;
    flex: 1;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 0 6px;
    min-width: 0;
  }

  .goals .slot {
    color: var(--faint);
    font-size: var(--text-xs);
  }

  .goals .when {
    margin-left: auto;
  }

  .goals .remove {
    display: grid;
    flex: none;
    place-items: center;
    width: 20px;
    height: 20px;
    border-radius: var(--radius-sm);
    color: var(--faint);
  }

  .goals .remove:hover,
  .goals .remove:focus-visible {
    background: rgba(255, 255, 255, 0.07);
    color: var(--text);
    text-decoration: none;
  }

  .quiet-line {
    margin: 0 0 4px;
    color: var(--faint);
    font-size: var(--text-sm);
  }

  .quiet-line strong {
    color: var(--muted);
    font-weight: 600;
  }

  .goal-error {
    margin: 4px 0 0;
    color: var(--bad);
    font-size: var(--text-sm);
  }

  .find {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    margin-top: 10px;
    padding: 0 8px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--faint);
  }

  .find:focus-within {
    border-color: color-mix(in srgb, var(--accent) 60%, transparent);
  }

  .find input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: 0;
    background: transparent;
    color: var(--text);
    font-size: var(--text-sm);
  }

  .found {
    margin-top: 4px;
    border: 1px solid var(--border);
  }

  .found li + li {
    border-top: 1px solid var(--border);
  }

  .found button {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 5px 8px;
    border: 0;
    background: transparent;
    color: var(--text);
    font-size: var(--text-sm);
    text-align: left;
    cursor: pointer;
  }

  .found button:hover:not(:disabled) {
    background: var(--surface-2);
  }

  .found button:disabled {
    color: var(--muted);
    cursor: default;
  }

  .found .found-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .found small {
    flex: none;
    color: var(--faint);
  }

  .found :global(svg) {
    flex: none;
    color: var(--accent);
  }

  .goals button {
    padding: 0;
    border: 0;
    background: transparent;
    font-size: var(--text-sm);
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
    font-size: var(--text-xs);
    text-align: left;
    cursor: pointer;
  }

  .credit:hover,
  .credit:focus-visible {
    color: var(--muted);
    outline: none;
  }
</style>
