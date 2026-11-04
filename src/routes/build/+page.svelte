<script lang="ts">
  // The build editor. A build is ordered goals, the way guides put them, whether copied from one
  // or the player's own. Changes save themselves, and the level table follows.
  import {
    ArrowDown,
    ArrowUp,
    ClipboardCopy,
    ClipboardPaste,
    ExternalLink,
    Milestone,
    Plus,
    RotateCcw,
    Trash2,
    TriangleAlert,
    X,
  } from "@lucide/svelte";
  import { onMount, untrack } from "svelte";

  import { backend } from "$lib/api";
  import { STATS, buildState, statLabel, watchBuilds } from "$lib/builds.svelte";
  import GearPicks from "$lib/components/GearPicks.svelte";
  import MapDataPanel from "$lib/components/map/MapDataPanel.svelte";
  import { guideLinks } from "$lib/guides";
  import { OSMS_CREDIT, OSMS_URL, mapData, watchMapData } from "$lib/mapdata.svelte";
  import { live, watchLive } from "$lib/readings.svelte";
  import { DEFAULT_RULES, changeSettings, display, keyHint, watchSettings } from "$lib/settings.svelte";
  import type {
    BaseStat,
    Build,
    BuildCatalog,
    BuildPlan,
    BuildRules,
    BuildSummary,
    Expansion,
    GearItem,
    LevelStep,
    SkillInfo,
    StarterPlan,
  } from "$lib/types";

  const SAVE_DELAY = 250;
  /** The most gear results shown under the search box. */
  const GEAR_SHOWN = 8;

  let catalog = $state<BuildCatalog | null>(null);
  let summaries = $state<BuildSummary[]>([]);
  let draft = $state<Build | null>(null);
  let expansion = $state<Expansion | null>(null);
  let loaded = $state(false);
  let error = $state<string | null>(null);
  let copied = $state(false);
  let confirmDelete = $state(false);
  let pasting = $state(false);
  let pasted = $state("");
  let gearText = $state("");
  let gearFound = $state<GearItem[]>([]);
  /** The level the suggestions are for, until the player picks one: the character's own. */
  let gearLevel = $state<number | null>(null);
  let starters = $state<StarterPlan[]>([]);
  /** The plan waiting for a second click, because it would replace goals already written. */
  let confirmStarter = $state<string | null>(null);

  /** The draft as last saved, so an unchanged draft isn't saved again. */
  let saved = "";
  let saveTimer: ReturnType<typeof setTimeout> | undefined;
  let rulesTimer: ReturnType<typeof setTimeout> | undefined;

  const ready = $derived(mapData.status?.ready ?? false);
  const character = $derived(live.analysis?.hud?.name ?? null);
  const level = $derived(live.analysis?.hud?.level ?? live.session?.level ?? null);

  onMount(() => {
    watchLive();
    watchSettings();
    watchBuilds();
    watchMapData();
    return () => clearTimeout(saveTimer);
  });

  $effect(() => {
    void mapData.status?.dataUpdatedAt;
    if (!ready) return;
    backend.buildCatalog().then((c) => (catalog = c));
    backend.starterPlans().then((plans) => (starters = plans));
    if (!draft) untrack(openActive);
  });

  $effect(() => {
    void buildState.version;
    backend.listBuilds().then((list) => (summaries = list));
  });

  // Saves a moment after typing stops, and only what really changed.
  $effect(() => {
    if (!draft) return;
    const snapshot = JSON.stringify(draft);
    if (snapshot === saved) return;
    clearTimeout(saveTimer);
    saveTimer = setTimeout(() => save(snapshot), SAVE_DELAY);
  });

  // A gear goal can be set from the Lookup tab. Unsaved typing here comes first.
  $effect(() => {
    void buildState.version;
    const id = untrack(() => draft?.id);
    if (!id) return;
    backend.buildPlan(id).then((plan) => {
      if (!plan || !draft || draft.id !== id || JSON.stringify(draft) !== saved) return;
      if (JSON.stringify(plan.build.plan.gear) === JSON.stringify(draft.plan.gear)) return;
      draft.plan.gear = plan.build.plan.gear;
      saved = JSON.stringify(draft);
      expansion = plan.expansion;
    });
  });

  // The rules decide how the plan plays out, so the table follows them.
  $effect(() => {
    void JSON.stringify(display.buildRules);
    const id = untrack(() => draft?.id);
    if (id) backend.buildPlan(id).then((plan) => plan && (expansion = plan.expansion));
  });

  async function openActive() {
    const plan = await backend.activeBuild(untrack(() => character));
    if (plan) show(plan);
    loaded = true;
  }

  function show(plan: BuildPlan) {
    draft = plan.build;
    expansion = plan.expansion;
    saved = JSON.stringify(plan.build);
    confirmDelete = false;
    copied = false;
    gearText = "";
    gearFound = [];
    gearLevel = null;
  }

  async function save(snapshot: string) {
    try {
      const plan = await backend.saveBuild(tidied(JSON.parse(snapshot)));
      saved = snapshot;
      expansion = plan.expansion;
      error = null;
    } catch (e) {
      error = String(e);
    }
  }

  /** Emptied number boxes come back as nothing; the backend wants numbers. */
  const whole = (value: unknown) => Math.max(0, Math.floor(Number(value) || 0));

  function tidied(build: Build): Build {
    return {
      ...build,
      start: {
        ...build.start,
        level: Math.max(1, whole(build.start.level)),
        ap: whole(build.start.ap),
        sp: whole(build.start.sp),
      },
      plan: {
        ...build.plan,
        planTo: Math.max(1, whole(build.plan.planTo)),
        skills: build.plan.skills.map((g) => ({ ...g, level: whole(g.level) })),
        stats: build.plan.stats.map((g) => ({
          ...g,
          target: g.target == null ? null : whole(g.target),
          fromLevel: g.fromLevel ? whole(g.fromLevel) : null,
        })),
      },
    };
  }

  async function open(id: number) {
    clearTimeout(saveTimer);
    const plan = await backend.buildPlan(id);
    if (plan) show(plan);
    await backend.useBuild(id);
  }

  async function create() {
    const name = character ? `${character}'s build` : "New build";
    const plan = await backend.saveBuild({
      id: 0,
      name,
      character,
      guide: "",
      start: { level: level ?? 1, stats: null, ap: 0, sp: 0, skills: [] },
      plan: { job: 0, skills: [], stats: [], gear: [], planTo: 70 },
    });
    show(plan);
  }

  async function remove() {
    if (!draft) return;
    if (!confirmDelete) {
      confirmDelete = true;
      return;
    }
    clearTimeout(saveTimer);
    await backend.deleteBuild(draft.id);
    draft = null;
    expansion = null;
    await openActive();
  }

  async function copyCode() {
    if (!draft) return;
    try {
      await navigator.clipboard.writeText(await backend.exportBuild(draft.id));
      copied = true;
    } catch (e) {
      error = `Couldn't copy: ${e}`;
    }
  }

  async function addFromCode() {
    try {
      show(await backend.importBuild(pasted, character));
      pasted = "";
      pasting = false;
      error = null;
    } catch (e) {
      error = String(e);
    }
  }

  const job = $derived(draft?.plan.job ?? 0);
  const firstJob = $derived(job % 100 === 0 ? job : Math.floor(job / 100) * 100);
  const secondJob = $derived(job % 100 === 0 ? 0 : job);
  const firstJobs = $derived(catalog?.jobs.filter((j) => j.tier === 1) ?? []);
  const secondJobs = $derived(catalog?.jobs.filter((j) => j.parent === firstJob) ?? []);
  const jobName = (id: number) => catalog?.jobs.find((j) => j.id === id)?.name ?? "";

  function setJob(id: number) {
    if (!draft) return;
    draft.plan.job = id;
    // A place to start, not advice. Goals already written are left alone.
    const primary = catalog?.jobs.find((j) => j.id === id)?.primary;
    if (primary && !draft.plan.stats.length) draft.plan.stats.push({ stat: primary, target: null, fromLevel: null });
  }

  /** The plans for this job, or all of them before one is chosen. */
  const jobPlans = $derived(
    starters.filter((s) => (secondJob ? s.job === secondJob : firstJob ? Math.floor(s.job / 100) * 100 === firstJob : true)),
  );
  const jobGuides = $derived([firstJob, secondJob].filter(Boolean).flatMap((id) => guideLinks(id).map((link) => ({ ...link, job: jobName(id) }))));

  /** Fills in a plan's goals. The character's own starting point stays. */
  function useStarter(starter: StarterPlan) {
    if (!draft) return;
    const written = draft.plan.skills.length > 0 || draft.plan.gear.length > 0 || draft.plan.stats.length > 1;
    if (written && confirmStarter !== starter.id) {
      confirmStarter = starter.id;
      return;
    }
    confirmStarter = null;
    draft.plan = JSON.parse(JSON.stringify(starter.plan));
  }

  const pathSkills = $derived(
    catalog?.skills.filter((s) => s.jobId === 0 || s.jobId === firstJob || (secondJob !== 0 && s.jobId === secondJob)) ?? [],
  );
  const groups = $derived(
    [0, firstJob, secondJob]
      .filter((id, i) => i === 0 || id !== 0)
      .map((id) => ({ name: id ? jobName(id) : "Beginner", skills: pathSkills.filter((s) => s.jobId === id) })),
  );
  const skillOf = (id: number) => catalog?.skills.find((s) => s.id === id);

  function addSkill(id: number) {
    const skill = skillOf(id);
    if (draft && skill) draft.plan.skills.push({ skillId: id, level: skill.maxLevel });
  }

  /** Moves a goal up or down its list; a goal's place decides what gets points first. */
  function move<T>(list: T[], i: number, by: number) {
    const to = i + by;
    if (to < 0 || to >= list.length) return;
    [list[i], list[to]] = [list[to], list[i]];
  }

  function needsText(skill: SkillInfo) {
    return skill.requiredSkills.map((n) => `${skillOf(n.skillId)?.name ?? "Another skill"} ${n.level}`).join(", ");
  }

  function setStartStat(stat: BaseStat, value: string) {
    if (!draft) return;
    draft.start.stats ??= { str: 0, dex: 0, int: 0, luk: 0 };
    draft.start.stats[stat] = whole(value);
  }

  const haveLevel = (id: number) => draft?.start.skills.find((s) => s.skillId === id)?.level ?? 0;

  function setHave(id: number, value: string) {
    if (!draft) return;
    const level = whole(value);
    draft.start.skills = draft.start.skills.filter((s) => s.skillId !== id);
    if (level) draft.start.skills.push({ skillId: id, level });
  }

  // Each answer belongs to the search that asked; a slower, older one is dropped.
  $effect(() => {
    const text = gearText;
    const jobNow = job;
    if (text.trim().length < 2) {
      gearFound = [];
      return;
    }
    backend.searchGear(text, jobNow).then((found) => {
      if (text === gearText) gearFound = found.slice(0, GEAR_SHOWN);
    });
  });

  function requires(item: GearItem) {
    const needs: [string, number][] = [
      ["Lv", item.reqLevel],
      ["STR", item.reqStr],
      ["DEX", item.reqDex],
      ["INT", item.reqInt],
      ["LUK", item.reqLuk],
    ];
    return needs.filter(([, n]) => n > 0).map(([label, n]) => `${label} ${n}`).join(", ") || "No requirements";
  }

  const suggestAt = $derived(gearLevel ?? level ?? draft?.start.level ?? 1);

  const gives = (item: GearItem) => item.stats.map((s) => `${s.label} +${s.value}`).join(", ");

  function setRules(change: Partial<BuildRules>) {
    const rules = { ...display.buildRules, ...change };
    display.buildRules = rules;
    clearTimeout(rulesTimer);
    rulesTimer = setTimeout(() => changeSettings({ buildRules: rules }), SAVE_DELAY);
  }

  const rulesChanged = $derived(JSON.stringify(display.buildRules) !== JSON.stringify(DEFAULT_RULES));

  const hasWork = (s: LevelStep) => s.ap.length > 0 || s.sp.length > 0 || s.advance != null || s.gear.length > 0;
  const steps = $derived(expansion?.steps.filter(hasWork) ?? []);
  const warnings = $derived(expansion?.problems.filter((p) => p.severity === "warning") ?? []);
  const notes = $derived(expansion?.problems.filter((p) => p.severity === "note") ?? []);
</script>

<svelte:head><title>Leafline — Build planner</title></svelte:head>

{#snippet moveButtons(list: unknown[], i: number)}
  <span class="moves">
    <button class="icon" title="Move up" aria-label="Move up" disabled={i === 0} onclick={() => move(list, i, -1)}><ArrowUp size={12} /></button>
    <button class="icon" title="Move down" aria-label="Move down" disabled={i === list.length - 1} onclick={() => move(list, i, 1)}><ArrowDown size={12} /></button>
  </span>
{/snippet}

<div class="page">
  <nav aria-label="Builds">
    <div class="brand"><Milestone size={16} /><span>Builds</span></div>
    <div class="list">
      {#each summaries as summary (summary.id)}
        <button class="build" class:active={draft?.id === summary.id} onclick={() => open(summary.id)}>
          <span class="build-name">{summary.name}</span>
          {#if summary.character}<span class="build-who">{summary.character}</span>{/if}
        </button>
      {/each}
    </div>
    {#if ready}
      <button class="side" onclick={create}><Plus size={13} />New build</button>
      <button class="side" onclick={() => (pasting = !pasting)}><ClipboardPaste size={13} />Paste a code</button>
      {#if pasting}
        <textarea rows="3" placeholder="LEAFLINE-BUILD-1:…" aria-label="Build code" bind:value={pasted}></textarea>
        <button class="side" disabled={!pasted.trim()} onclick={addFromCode}>Add this build</button>
      {/if}
    {/if}
  </nav>

  {#if !ready}
    <main class="center">
      <h1>Plan a build</h1>
      <p class="lead">Builds use the skills and equipment in the map data.</p>
      <div class="box"><MapDataPanel /></div>
    </main>
  {:else if !draft}
    <main class="center">
      {#if loaded}
        <h1>Plan a build</h1>
        <p class="lead">
          Choose a job, then write your goals in order, from a guide or your own idea. Leafline lays them out level by
          level and says what can't work.
        </p>
        <button class="primary" onclick={create}><Plus size={13} />New build</button>
      {/if}
    </main>
  {:else}
    <main>
      {#if error}<p class="error" role="alert">{error}</p>{/if}

      <section class="title">
        <label class="grow"><span>Build name</span><input maxlength="80" bind:value={draft.name} /></label>
        <label>
          <span title="This build shows when the HUD reads this name.">Character</span>
          <input
            maxlength="30"
            placeholder="Any"
            value={draft.character ?? ""}
            oninput={(e) => draft && (draft.character = e.currentTarget.value || null)}
          />
        </label>
        <div class="tools">
          <button class="plain" onclick={copyCode} title="Copy this plan as a line of text. It holds your goals only, not your character or stats.">
            <ClipboardCopy size={13} />{copied ? "Copied" : "Copy plan"}
          </button>
          <button class="plain" class:danger={confirmDelete} onclick={remove}>
            <Trash2 size={13} />{confirmDelete ? "Really delete?" : "Delete"}
          </button>
        </div>
      </section>

      <section>
        <h2>Job</h2>
        <div class="fields">
          <label>
            <span>1st job</span>
            <select value={firstJob} onchange={(e) => setJob(Number(e.currentTarget.value))}>
              <option value={0}>Not chosen yet</option>
              {#each firstJobs as choice (choice.id)}<option value={choice.id}>{choice.name}</option>{/each}
            </select>
          </label>
          <label>
            <span>2nd job</span>
            <select
              value={secondJob}
              disabled={!firstJob}
              onchange={(e) => setJob(Number(e.currentTarget.value) || firstJob)}
            >
              <option value={0}>Not chosen yet</option>
              {#each secondJobs as choice (choice.id)}<option value={choice.id}>{choice.name}</option>{/each}
            </select>
          </label>
          <label><span>Plan up to Lv</span><input class="number" type="number" min="1" max="200" bind:value={draft.plan.planTo} /></label>
        </div>
      </section>

      {#if jobPlans.length}
        <section>
          <h2>Start from a plan</h2>
          <p class="help">
            Written for Leafline from what each skill does and what each weapon needs. Nobody has played them yet, so treat
            them as a starting point. Using one fills in your goals, which you can then change. Your stats stay as they are.
          </p>
          <ul class="plans">
            {#each jobPlans as starter (starter.id)}
              <li>
                <span class="goal-name">
                  <span>{starter.name}</span>
                  <small>{starter.summary}</small>
                  {#if starter.missing.length}<small class="warn">Not in this data: {starter.missing.join(", ")}</small>{/if}
                </span>
                <button class="plain" class:danger={confirmStarter === starter.id} onclick={() => useStarter(starter)}>
                  {confirmStarter === starter.id ? "Replace my goals?" : "Use this plan"}
                </button>
              </li>
            {/each}
          </ul>
        </section>
      {/if}

      <section>
        <h2>Where you are now</h2>
        <p class="help">Live reading fills these in while your Character Stat and Skill windows are open in game. With it off, use Read stats in the overlay{keyHint("ReadCharacter")}, or type them here. The plan carries on from whatever they say.</p>
        <div class="fields">
          <label><span>Level</span><input class="number" type="number" min="1" max="200" bind:value={draft.start.level} /></label>
          {#each STATS as stat (stat.id)}
            <label>
              <span>{stat.label}</span>
              <input
                class="number"
                type="number"
                min="0"
                value={draft.start.stats?.[stat.id] ?? ""}
                oninput={(e) => setStartStat(stat.id, e.currentTarget.value)}
              />
            </label>
          {/each}
          <label><span>Unspent AP</span><input class="number" type="number" min="0" bind:value={draft.start.ap} /></label>
          <label><span>Unspent SP</span><input class="number" type="number" min="0" bind:value={draft.start.sp} /></label>
        </div>
        {#if pathSkills.length}
          <details>
            <summary>Skill points you already spent</summary>
            <div class="have">
              {#each pathSkills as skill (skill.id)}
                <label>
                  <span>{skill.name}</span>
                  <input
                    class="number"
                    type="number"
                    min="0"
                    max={skill.maxLevel}
                    value={haveLevel(skill.id) || ""}
                    oninput={(e) => setHave(skill.id, e.currentTarget.value)}
                  />
                </label>
              {/each}
            </div>
          </details>
        {/if}
      </section>

      <section>
        <h2>Skill goals</h2>
        <p class="help">Each SP goes to the first goal that can take it. A skill can come back later in the list.</p>
        {#if draft.plan.skills.length}
          <ol class="goals">
            {#each draft.plan.skills as goal, i}
              {@const skill = skillOf(goal.skillId)}
              <li>
                {@render moveButtons(draft.plan.skills, i)}
                <span class="goal-name">
                  <span>{skill?.name ?? `Unknown skill ${goal.skillId}`}</span>
                  {#if skill?.levelDescriptions[goal.level - 1]}
                    <small>Level {goal.level}: {skill.levelDescriptions[goal.level - 1]}</small>
                  {/if}
                  {#if skill?.requiredSkills.length}<small>Needs {needsText(skill)}</small>{/if}
                </span>
                <label class="inline">
                  <span>to</span>
                  <input class="number" type="number" min="1" max={skill?.maxLevel} bind:value={goal.level} />
                </label>
                <span class="max tabular">of {skill?.maxLevel ?? "?"}</span>
                <button class="icon" title="Remove" aria-label="Remove" onclick={() => draft?.plan.skills.splice(i, 1)}><X size={12} /></button>
              </li>
            {/each}
          </ol>
        {/if}
        <label class="add">
          <span>Add a skill</span>
          <select
            value=""
            onchange={(e) => {
              addSkill(Number(e.currentTarget.value));
              e.currentTarget.value = "";
            }}
          >
            <option value="" disabled>Choose a skill</option>
            {#each groups as group (group.name)}
              <optgroup label={group.name}>
                {#each group.skills as skill (skill.id)}<option value={skill.id}>{skill.name}</option>{/each}
              </optgroup>
            {/each}
          </select>
        </label>
        {#if !firstJob}<p class="help">Choose a job to add its skills.</p>{/if}
      </section>

      <section>
        <h2>Stat goals</h2>
        <p class="help">AP goes to the first goal that can take it. End with one goal that takes the rest.</p>
        {#if draft.plan.stats.length}
          <ol class="goals">
            {#each draft.plan.stats as goal, i}
              <li>
                {@render moveButtons(draft.plan.stats, i)}
                <select aria-label="Stat" bind:value={goal.stat}>
                  {#each STATS as stat (stat.id)}<option value={stat.id}>{stat.label}</option>{/each}
                </select>
                <select
                  aria-label="How much"
                  value={goal.target == null ? "rest" : "to"}
                  onchange={(e) => (goal.target = e.currentTarget.value === "rest" ? null : (draft?.start.stats?.[goal.stat] ?? 0) + 5)}
                >
                  <option value="to">up to</option>
                  <option value="rest">all the rest</option>
                </select>
                {#if goal.target != null}
                  <input class="number" aria-label="Target" type="number" min="0" bind:value={goal.target} />
                {/if}
                <label class="inline from">
                  <span>from Lv</span>
                  <input
                    class="number"
                    type="number"
                    min="1"
                    placeholder="now"
                    value={goal.fromLevel ?? ""}
                    oninput={(e) => (goal.fromLevel = whole(e.currentTarget.value) || null)}
                  />
                </label>
                <button class="icon" title="Remove" aria-label="Remove" onclick={() => draft?.plan.stats.splice(i, 1)}><X size={12} /></button>
              </li>
            {/each}
          </ol>
        {/if}
        <button
          class="plain"
          onclick={() => draft?.plan.stats.push({ stat: "dex", target: (draft?.start.stats?.dex ?? 0) + 5, fromLevel: null })}
        >
          <Plus size={13} />Add a stat goal
        </button>
      </section>

      <section>
        <h2>Gear to aim for</h2>
        <p class="help">Shows the first level this plan can wear it, counting the stats on earlier gear goals. Names open in the Lookup tab.</p>
        {#if expansion?.gear.length}
          <ol class="goals">
            {#each expansion.gear as check (check.item.id)}
              <li>
                <span class="goal-name">
                  <button class="item-link" title="Show it in the Lookup tab" onclick={() => backend.showItem(check.item.id, null)}>{check.item.name}</button>
                  <small>{check.item.slot} · {requires(check.item)}{gives(check.item) ? ` · ${gives(check.item)}` : ""}</small>
                </span>
                <span class="max tabular" class:warn={!check.usable || check.level == null}>
                  {#if !check.usable}Wrong job{:else if check.level != null}Lv {check.level}{:else}Out of reach{/if}
                </span>
                <button
                  class="icon"
                  title="Remove"
                  aria-label="Remove"
                  onclick={() => draft && (draft.plan.gear = draft.plan.gear.filter((id) => id !== check.item.id))}><X size={12} /></button
                >
              </li>
            {/each}
          </ol>
        {/if}
        <label class="add">
          <span>Find equipment</span>
          <input placeholder="Type a name" bind:value={gearText} />
        </label>
        {#if gearFound.length}
          <ul class="found">
            {#each gearFound as item (item.id)}
              <li>
                <button
                  disabled={draft.plan.gear.includes(item.id)}
                  onclick={() => draft?.plan.gear.push(item.id)}
                >
                  <span>{item.name}</span>
                  <small>{item.slot} · {requires(item)}</small>
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      </section>

      <section>
        <h2>Suggested gear</h2>
        <p class="help">What this plan could wear at a level, slot by slot, to look through. Add any you like as goals above.</p>
        <div class="fields">
          <label>
            <span>At level</span>
            <input class="number" type="number" min="1" max="200" value={suggestAt} oninput={(e) => (gearLevel = Math.max(1, whole(e.currentTarget.value)))} />
          </label>
          <label>
            <span>Gear made for</span>
            <select value={draft.start.gender ?? ""} onchange={(e) => draft && (draft.start.gender = e.currentTarget.value || null)}>
              <option value="">Either</option>
              <option value="male">Male</option>
              <option value="female">Female</option>
            </select>
          </label>
        </div>
        {#if draft.id}
          <GearPicks build={draft.id} level={suggestAt} full onAdd={(item) => draft?.plan.gear.push(item.id)} />
        {/if}
      </section>

      <section>
        <h2>Guides</h2>
        {#if jobGuides.length}
          <p class="help">Written by other players, on their own sites. Enter what you want from them as goals above.</p>
          <div class="links">
            {#each jobGuides as guide (guide.url)}
              <button class="plain" onclick={() => backend.openGuide(guide.url)}>
                <ExternalLink size={13} />{guide.job}, {guide.site} ({guide.levels})
              </button>
            {/each}
          </div>
        {/if}
        <p class="help">Following another guide? Keep its link here. It stays on this PC and isn't part of a copied plan.</p>
        <div class="guide">
          <input type="url" placeholder="https://" aria-label="Guide link" bind:value={draft.guide} />
          <button class="plain" disabled={!draft.guide.trim()} onclick={() => draft && backend.openGuide(draft.guide)}>
            <ExternalLink size={13} />Open
          </button>
        </div>
      </section>

      <section>
        <details>
          <summary>How points are counted</summary>
          <p class="help">
            AP and Beginner SP at Lv 2 were seen in game. The rest come from community guides and aren't confirmed by
            Nexon. If your game shows something else, change them here. They apply to every build.
          </p>
          <div class="fields">
            <label><span>AP per level</span><input class="number" type="number" min="0" value={display.buildRules.apPerLevel} oninput={(e) => setRules({ apPerLevel: whole(e.currentTarget.value) })} /></label>
            <label><span>SP per level</span><input class="number" type="number" min="0" value={display.buildRules.spPerLevel} oninput={(e) => setRules({ spPerLevel: whole(e.currentTarget.value) })} /></label>
            <label><span>Beginner SP per level</span><input class="number" type="number" min="0" value={display.buildRules.beginnerSpPerLevel} oninput={(e) => setRules({ beginnerSpPerLevel: whole(e.currentTarget.value) })} /></label>
            <label><span>SP per advancement</span><input class="number" type="number" min="0" value={display.buildRules.advancementSp} oninput={(e) => setRules({ advancementSp: whole(e.currentTarget.value) })} /></label>
            <label><span>1st job at Lv</span><input class="number" type="number" min="2" value={display.buildRules.firstJobLevel} oninput={(e) => setRules({ firstJobLevel: whole(e.currentTarget.value) })} /></label>
            <label><span>2nd job at Lv</span><input class="number" type="number" min="2" value={display.buildRules.secondJobLevel} oninput={(e) => setRules({ secondJobLevel: whole(e.currentTarget.value) })} /></label>
          </div>
          <label class="check">
            <input type="checkbox" checked={display.buildRules.secondSpBuysFirst} onchange={(e) => setRules({ secondSpBuysFirst: e.currentTarget.checked })} />
            <span>2nd-job SP can buy 1st-job skills</span>
          </label>
          <label class="check">
            <input type="checkbox" checked={display.buildRules.gearCounts} onchange={(e) => setRules({ gearCounts: e.currentTarget.checked })} />
            <span>Stats from other worn gear count toward requirements</span>
          </label>
          {#if rulesChanged}
            <button class="plain" onclick={() => setRules(DEFAULT_RULES)}><RotateCcw size={13} />Back to the defaults</button>
          {/if}
        </details>
      </section>

      <button class="credit" onclick={() => backend.openLink(OSMS_URL)}>{OSMS_CREDIT}</button>
    </main>

    <aside aria-label="The plan level by level">
      {#if warnings.length || notes.length}
        <div class="problems">
          {#each warnings as problem (problem.message)}
            <p class="warning"><TriangleAlert size={12} /><span>{problem.message}</span></p>
          {/each}
          {#each notes as problem (problem.message)}<p class="note">{problem.message}</p>{/each}
        </div>
      {/if}
      <h2>Level by level</h2>
      {#if steps.length}
        <ol class="levels">
          {#each steps as step (step.level)}
            <li class:current={step.level === level}>
              <span class="at tabular">Lv {step.level}</span>
              <div>
                {#if step.advance}<p class="advance">{step.advance}</p>{/if}
                {#each step.sp as gain (gain.skillId)}
                  <p><span class="tag">{gain.to - gain.from} SP</span>{gain.name} <span class="tabular dim">{gain.from} → {gain.to}</span></p>
                {/each}
                {#each step.ap as gain (gain.stat)}
                  <p><span class="tag">{gain.amount} AP</span>{statLabel(gain.stat)} <span class="tabular dim">to {gain.total}</span></p>
                {/each}
                {#each step.gear as name (name)}<p><span class="tag">Gear</span>{name}</p>{/each}
              </div>
            </li>
          {/each}
        </ol>
      {:else}
        <p class="help">Your plan shows here as you add goals.</p>
      {/if}
    </aside>
  {/if}
</div>

<style>
  :global(body) {
    background: var(--bg);
  }

  .page {
    display: grid;
    grid-template-columns: 176px minmax(0, 1fr) 300px;
    height: 100vh;
    overflow: hidden;
  }

  nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-height: 0;
    padding: 16px 10px;
    border-right: 1px solid var(--border);
    background: var(--surface);
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0 8px 12px;
    font-family: var(--font-display);
    font-weight: 600;
  }

  .list {
    display: grid;
    align-content: start;
    gap: 1px;
    min-height: 0;
    margin-bottom: 8px;
    overflow-y: auto;
  }

  .build {
    display: grid;
    gap: 0;
    width: 100%;
    padding: 6px 10px;
    border: 0;
    border-left: 2px solid transparent;
    border-radius: 0;
    background: transparent;
    color: var(--muted);
    text-align: left;
    cursor: pointer;
  }

  .build:hover {
    background: rgba(255, 255, 255, 0.05);
    color: var(--text);
  }

  .build.active {
    border-left-color: var(--accent);
    color: var(--text);
  }

  .build-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .build-who {
    color: var(--faint);
    font-size: 11px;
  }

  .side {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 10px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--muted);
    font-size: 12px;
    cursor: pointer;
  }

  .side:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.05);
    color: var(--text);
  }

  .side:disabled {
    color: var(--faint);
    cursor: default;
  }

  textarea {
    width: 100%;
    margin: 4px 0;
    resize: vertical;
  }

  main {
    min-width: 0;
    padding: 4px 22px 28px;
    overflow-y: auto;
  }

  main.center {
    grid-column: 2 / -1;
    align-self: center;
    justify-self: center;
    max-width: 420px;
    padding: 24px;
  }

  h1 {
    margin: 0 0 6px;
    font-family: var(--font-display);
    font-size: 18px;
    font-weight: 600;
  }

  .lead {
    margin: 0 0 14px;
    color: var(--muted);
  }

  .box {
    padding: 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
  }

  section {
    padding: 14px 0;
    border-bottom: 1px solid var(--border);
  }

  .title {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: 12px;
  }

  .grow {
    flex: 1;
    min-width: 180px;
  }

  .grow input {
    width: 100%;
  }

  .tools {
    display: flex;
    gap: 6px;
  }

  h2 {
    margin: 0 0 3px;
    font-family: var(--font-display);
    font-size: 14px;
    font-weight: 600;
  }

  .help {
    margin: 0 0 8px;
    color: var(--faint);
    font-size: 12px;
  }

  .fields {
    display: flex;
    flex-wrap: wrap;
    gap: 8px 12px;
    margin-top: 6px;
  }

  label {
    display: grid;
    gap: 3px;
    color: var(--muted);
    font-size: 11.5px;
  }

  label.check {
    display: flex;
    align-items: center;
    gap: 7px;
    margin-top: 8px;
  }

  label.check input {
    accent-color: var(--accent);
  }

  label.inline {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    flex: none;
  }

  input,
  select,
  textarea {
    padding: 4px 7px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    color: var(--text);
    font: inherit;
    font-size: 12.5px;
  }

  input:focus-visible,
  select:focus-visible,
  textarea:focus-visible {
    border-color: var(--accent);
    outline: none;
  }

  input::placeholder,
  textarea::placeholder {
    color: var(--faint);
  }

  .number {
    width: 62px;
    font-variant-numeric: tabular-nums;
  }

  details {
    margin-top: 10px;
  }

  summary {
    color: var(--muted);
    font-size: 12px;
    cursor: pointer;
  }

  summary:hover {
    color: var(--text);
  }

  section > details:only-child {
    margin-top: 0;
  }

  details .help:first-of-type {
    margin-top: 8px;
  }

  .have {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(170px, 1fr));
    gap: 6px 14px;
    margin-top: 8px;
  }

  .have label {
    grid-template-columns: 1fr auto;
    align-items: center;
  }

  .goals {
    display: grid;
    margin: 0 0 8px;
    padding: 0;
    list-style: none;
  }

  .goals li {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 0;
  }

  .goals li + li {
    border-top: 1px solid var(--border);
  }

  .item-link {
    padding: 0;
    border: 0;
    background: transparent;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .item-link:hover,
  .item-link:focus-visible {
    color: var(--accent-strong);
    text-decoration: underline;
    outline: none;
  }

  .goal-name {
    display: grid;
    flex: 1;
    min-width: 0;
  }

  small {
    color: var(--faint);
    font-size: 11px;
    overflow-wrap: anywhere;
  }

  .max {
    flex: none;
    color: var(--faint);
    font-size: 11.5px;
  }

  .max.warn {
    color: var(--warn);
  }

  .from {
    margin-left: auto;
  }

  .moves {
    display: inline-flex;
    flex: none;
  }

  .icon {
    display: grid;
    flex: none;
    place-items: center;
    width: 20px;
    height: 20px;
    padding: 0;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--faint);
    cursor: pointer;
  }

  .icon:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.07);
    color: var(--text);
  }

  .icon:disabled {
    opacity: 0.35;
    cursor: default;
  }

  .add {
    margin-top: 4px;
  }

  .add select,
  .add input {
    max-width: 300px;
  }

  .found {
    display: grid;
    gap: 1px;
    max-width: 420px;
    margin: 4px 0 0;
    padding: 0;
    list-style: none;
  }

  .found button {
    display: grid;
    width: 100%;
    padding: 4px 8px;
    border: 0;
    border-radius: var(--radius-sm);
    background: rgba(255, 255, 255, 0.03);
    text-align: left;
    cursor: pointer;
  }

  .found button:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.08);
  }

  .found button:disabled {
    opacity: 0.45;
    cursor: default;
  }

  .guide {
    display: flex;
    gap: 6px;
  }

  .links {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-bottom: 10px;
  }

  .plans {
    display: grid;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .plans li {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 0;
  }

  .plans li + li {
    border-top: 1px solid var(--border);
  }

  small.warn {
    color: var(--warn);
  }

  .guide input {
    flex: 1;
    min-width: 0;
  }

  .plain,
  .primary {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 9px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--muted);
    font-size: 12px;
    white-space: nowrap;
    cursor: pointer;
  }

  .plain:hover:not(:disabled) {
    color: var(--text);
    background: rgba(255, 255, 255, 0.05);
  }

  .plain:disabled {
    color: var(--faint);
    cursor: default;
  }

  .plain.danger {
    border-color: var(--bad);
    color: var(--bad);
  }

  .primary {
    border-color: color-mix(in srgb, var(--accent) 45%, transparent);
    background: var(--accent-soft);
    color: var(--accent-strong);
  }

  .primary:hover {
    filter: brightness(1.2);
  }

  .error {
    margin: 12px 0 0;
    padding: 6px 10px;
    background: color-mix(in srgb, var(--bad) 12%, transparent);
    color: var(--bad);
    font-size: 12px;
  }

  .credit {
    margin-top: 14px;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--faint);
    font-size: 10.5px;
    cursor: pointer;
  }

  .credit:hover {
    color: var(--muted);
  }

  aside {
    min-height: 0;
    padding: 14px 14px 24px;
    border-left: 1px solid var(--border);
    background: var(--surface);
    overflow-y: auto;
  }

  .problems {
    display: grid;
    gap: 6px;
    margin-bottom: 14px;
    padding-bottom: 12px;
    border-bottom: 1px solid var(--border);
    font-size: 12px;
  }

  .problems p {
    margin: 0;
  }

  .warning {
    display: flex;
    gap: 6px;
    color: var(--warn);
  }

  .warning :global(svg) {
    flex: none;
    margin-top: 3px;
  }

  .note {
    color: var(--muted);
  }

  .levels {
    margin: 8px 0 0;
    padding: 0;
    list-style: none;
    font-size: 12px;
  }

  .levels li {
    display: flex;
    gap: 8px;
    padding: 5px 0;
    border-top: 1px solid var(--border);
  }

  .levels li.current {
    box-shadow: inset 2px 0 0 var(--accent);
    padding-left: 6px;
  }

  .levels p {
    display: flex;
    align-items: baseline;
    gap: 5px;
    margin: 0 0 1px;
  }

  .at {
    flex: none;
    width: 40px;
    color: var(--faint);
  }

  .tag {
    flex: none;
    width: 34px;
    color: var(--accent);
    font-size: 10.5px;
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
</style>
