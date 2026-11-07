<script lang="ts">
  // The build planner. A build is ordered goals, the way guides put them, whether copied from one
  // or the player's own. A new build starts with three choices; after that the plan is split into
  // Skills, Stats, Gear and Settings, with the levels beside every tab. Changes save themselves.
  import { ArrowDown, ArrowUp, ClipboardCopy, ClipboardPaste, ExternalLink, Milestone, Plus, RotateCcw, Search, Trash2, X } from "@lucide/svelte";
  import { onMount, untrack } from "svelte";

  import { backend } from "$lib/api";
  import { STATS, buildState, statLabel, watchBuilds } from "$lib/builds.svelte";
  import GearPicks from "$lib/components/GearPicks.svelte";
  import MapDataPanel from "$lib/components/map/MapDataPanel.svelte";
  import NewBuild from "$lib/components/planner/NewBuild.svelte";
  import Timeline from "$lib/components/planner/Timeline.svelte";
  import { guideLinks } from "$lib/guides";
  import { gives } from "$lib/items.svelte";
  import { OSMS_CREDIT, OSMS_URL, mapData, watchMapData } from "$lib/mapdata.svelte";
  import { apSplit, skillReached, statReached, stepAt, type Reached } from "$lib/planner";
  import { live, watchLive } from "$lib/readings.svelte";
  import { DEFAULT_RULES, changeSettings, display, keyHint, watchSettings } from "$lib/settings.svelte";
  import { watchTyping } from "$lib/typing";
  import type {
    BaseStat,
    Build,
    BuildCatalog,
    BuildPlan,
    BuildRules,
    BuildSummary,
    Expansion,
    GearCheck,
    GearItem,
    ItemRecord,
    Outfit,
    SkillInfo,
    StarterPlan,
  } from "$lib/types";

  const SAVE_DELAY = 250;
  /** The most gear results shown under a slot's search box. */
  const GEAR_SHOWN = 8;

  let catalog = $state<BuildCatalog | null>(null);
  let summaries = $state<BuildSummary[]>([]);
  let draft = $state<Build | null>(null);
  let expansion = $state<Expansion | null>(null);
  let loaded = $state(false);
  /** The New build screen is up, in place of a plan. */
  let creating = $state(false);
  let error = $state<string | null>(null);
  let copied = $state(false);
  let confirmDelete = $state(false);
  let pasting = $state(false);
  let pasted = $state("");
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
  const hudJob = $derived(live.analysis?.hud?.job ?? null);

  // Clicks leave the game focused; typing in a box takes focus while it lasts.
  onMount(() => watchTyping());

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

  // A gear goal can be set from the Lookup tab, and a piece put on from an item check. Unsaved
  // typing here comes first for goals; pieces are kept by the backend whatever this window saves.
  $effect(() => {
    void buildState.version;
    const id = untrack(() => draft?.id);
    if (!id) return;
    backend.buildPlan(id).then((plan) => {
      if (!plan || !draft || draft.id !== id || JSON.stringify(draft) !== saved) return;
      const same =
        JSON.stringify(plan.build.plan.gear) === JSON.stringify(draft.plan.gear) &&
        JSON.stringify(plan.build.start) === JSON.stringify(draft.start);
      if (same) return;
      draft.plan.gear = plan.build.plan.gear;
      draft.start = plan.build.start;
      saved = JSON.stringify(draft);
      expansion = plan.expansion;
    });
  });

  // The rules decide how the plan plays out, so the levels follow them.
  $effect(() => {
    void JSON.stringify(display.buildRules);
    const id = untrack(() => draft?.id);
    if (id) backend.buildPlan(id).then((plan) => plan && (expansion = plan.expansion));
  });

  async function openActive() {
    const plan = await backend.activeBuild(untrack(() => character));
    if (plan) show(plan);
    else creating = true;
    loaded = true;
  }

  function show(plan: BuildPlan) {
    draft = plan.build;
    expansion = plan.expansion;
    saved = JSON.stringify(plan.build);
    creating = false;
    confirmDelete = false;
    confirmStarter = null;
    copied = false;
    adding = null;
    gearText = "";
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

  async function duplicate() {
    if (!draft) return;
    clearTimeout(saveTimer);
    try {
      show(await backend.saveBuild({ ...tidied($state.snapshot(draft)), id: 0, name: `${draft.name} (copy)`.slice(0, 80) }));
    } catch (e) {
      error = String(e);
    }
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

  // Tabs: the one last picked is remembered.
  type Tab = "skills" | "stats" | "gear" | "settings";
  const TAB_KEY = "leafline.planner.tab";
  const TABS: { id: Tab; label: string }[] = [
    { id: "skills", label: "Skills" },
    { id: "stats", label: "Stats" },
    { id: "gear", label: "Gear" },
    { id: "settings", label: "Settings" },
  ];
  let tab = $state<Tab>(storedTab());

  function storedTab(): Tab {
    try {
      const saved = localStorage.getItem(TAB_KEY);
      return TABS.some((t) => t.id === saved) ? (saved as Tab) : "skills";
    } catch {
      return "skills";
    }
  }

  function pickTab(next: Tab) {
    tab = next;
    try {
      localStorage.setItem(TAB_KEY, next);
    } catch {
      // A convenience only.
    }
  }

  // The job.
  const job = $derived(draft?.plan.job ?? 0);
  const firstJob = $derived(job % 100 === 0 ? job : Math.floor(job / 100) * 100);
  const secondJob = $derived(job % 100 === 0 ? 0 : job);
  const firstJobs = $derived(catalog?.jobs.filter((j) => j.tier === 1) ?? []);
  const secondJobs = $derived(catalog?.jobs.filter((j) => j.parent === firstJob) ?? []);
  const jobName = (id: number) => catalog?.jobs.find((j) => j.id === id)?.name ?? "";
  const jobPath = $derived(
    [firstJob, secondJob]
      .filter(Boolean)
      .map(jobName)
      .join(" → ") || "No job chosen",
  );

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
    if (confirmStarter !== starter.id) {
      confirmStarter = starter.id;
      return;
    }
    confirmStarter = null;
    draft.plan = JSON.parse(JSON.stringify(starter.plan));
  }

  // Skills.
  const pathSkills = $derived(
    catalog?.skills.filter((s) => s.jobId === 0 || s.jobId === firstJob || (secondJob !== 0 && s.jobId === secondJob)) ?? [],
  );
  const groups = $derived(
    [0, firstJob, secondJob]
      .filter((id, i) => i === 0 || id !== 0)
      .map((id) => ({ name: id ? jobName(id) : "Beginner", skills: pathSkills.filter((s) => s.jobId === id) })),
  );
  const skillOf = (id: number) => catalog?.skills.find((s) => s.id === id);

  /** A goal's tier, which decides its group: Beginner, 1st job, 2nd job, or another path's. */
  function tierOf(skillId: number): number {
    const skill = skillOf(skillId);
    if (!skill) return 9;
    if (skill.jobId === 0) return 0;
    if (skill.jobId === firstJob) return 1;
    if (skill.jobId === secondJob) return 2;
    return 9;
  }

  const rules = $derived(display.buildRules);
  const tiers = $derived.by(() => {
    if (!draft) return [];
    const end = draft.plan.planTo;
    const names: Record<number, string> = {
      0: "Beginner",
      1: `1st job · ${jobName(firstJob) || "not chosen"}`,
      2: `2nd job · ${jobName(secondJob) || "not chosen"}`,
      9: "Not on this job's path",
    };
    const spans: Record<number, string> = {
      0: `to Lv ${rules.firstJobLevel - 1}`,
      1: `Lv ${rules.firstJobLevel} to ${rules.secondJobLevel - 1}`,
      2: `Lv ${rules.secondJobLevel} to ${end}`,
      9: "these get no SP",
    };
    return [0, 1, 2, 9]
      .map((tier) => ({
        tier,
        name: names[tier],
        span: spans[tier],
        goals: draft!.plan.skills.map((goal, index) => ({ goal, index })).filter(({ goal }) => tierOf(goal.skillId) === tier),
      }))
      .filter((t) => t.goals.length);
  });

  function addSkill(id: number) {
    const skill = skillOf(id);
    if (draft && skill) draft.plan.skills.push({ skillId: id, level: skill.maxLevel });
  }

  /** Moves a skill goal past the next one of its own tier: its place decides what gets SP first. */
  function moveSkill(index: number, by: number) {
    if (!draft) return;
    const list = draft.plan.skills;
    const tier = tierOf(list[index].skillId);
    let to = index + by;
    while (to >= 0 && to < list.length && tierOf(list[to].skillId) !== tier) to += by;
    if (to < 0 || to >= list.length) return;
    [list[index], list[to]] = [list[to], list[index]];
  }

  function move<T>(list: T[], i: number, by: number) {
    const to = i + by;
    if (to < 0 || to >= list.length) return;
    [list[i], list[to]] = [list[to], list[i]];
  }

  function needsText(skill: SkillInfo) {
    return skill.requiredSkills.map((n) => `${skillOf(n.skillId)?.name ?? "Another skill"} ${n.level}`).join(", ");
  }

  const steps = $derived(expansion?.steps ?? []);
  const endLevel = $derived(steps.at(-1)?.level ?? draft?.plan.planTo ?? 70);

  function reachedText(reached: Reached, verb = "by"): { text: string; tone: "good" | "normal" | "warn" } {
    if (reached.kind === "already") return { text: "done", tone: "good" };
    if (reached.kind === "at") return { text: `${verb} Lv ${reached.level}`, tone: level != null && reached.level <= level ? "good" : "normal" };
    return { text: `not by Lv ${reached.end}`, tone: "warn" };
  }

  // Stats.
  const STAT_COLOURS: Record<BaseStat, string> = { str: "var(--hp)", dex: "var(--travel)", int: "var(--mp)", luk: "var(--accent)" };
  const split = $derived(apSplit(steps));
  const splitTotal = $derived(split.reduce((sum, s) => sum + s.amount, 0));
  const milestones = $derived.by(() => {
    if (!draft || !steps.length) return [];
    const start = draft.start.level;
    const levels = [
      { level: start, label: level != null && level === start ? `Lv ${start} · now` : `Lv ${start} · start` },
      { level: rules.secondJobLevel, label: `Lv ${rules.secondJobLevel} · 2nd job` },
      { level: 50, label: "Lv 50" },
      { level: endLevel, label: `Lv ${endLevel}` },
    ].filter((m, i, all) => m.level >= start && m.level <= endLevel && all.findIndex((o) => o.level === m.level) === i);
    return levels.map((m) => ({ ...m, stats: m.level === start && draft!.start.stats ? draft!.start.stats : stepAt(steps, m.level)?.stats }));
  });

  // Gear: each slot's goals, in the order the plan can wear them.
  const SLOT_NAMES: Record<string, string> = { Glove: "Gloves", Earring: "Earrings" };
  const slotName = (slot: string) => SLOT_NAMES[slot] ?? slot;
  let outfit = $state<Outfit | null>(null);
  $effect(() => {
    void expansion;
    void buildState.version;
    const id = untrack(() => draft?.id);
    const at = untrack(() => level ?? draft?.start.level ?? 1);
    if (!id) return;
    backend.buildOutfit(id, at).then((o) => {
      if (untrack(() => draft?.id) === id) outfit = o;
    });
  });

  const goalsBySlot = $derived.by(() => {
    const by = new Map<string, GearCheck[]>();
    for (const check of expansion?.gear ?? []) by.set(check.item.slot, [...(by.get(check.item.slot) ?? []), check]);
    for (const list of by.values()) list.sort((a, b) => (a.level ?? 999) - (b.level ?? 999) || a.item.reqLevel - b.item.reqLevel);
    return by;
  });
  /** The outfit's slots, then any slot with goals it left out. */
  const gearSlots = $derived.by(() => {
    const slots = (outfit?.slots ?? []).map((s) => s.slot);
    for (const slot of goalsBySlot.keys()) if (!slots.includes(slot)) slots.push(slot);
    return slots;
  });
  const wearing = (slot: string) => outfit?.slots.find((s) => s.slot === slot)?.wearing ?? null;

  /** The slot whose finder is open. */
  let adding = $state<string | null>(null);
  let finderLevel = $state<number | null>(null);
  const finderAt = $derived(finderLevel ?? level ?? draft?.start.level ?? 1);
  let gearText = $state("");
  let gearFound = $state<GearItem[]>([]);

  // Each answer belongs to the search that asked; a slower, older one is dropped.
  $effect(() => {
    const text = gearText;
    const jobNow = job;
    const slot = adding;
    if (text.trim().length < 2 || !slot) {
      gearFound = [];
      return;
    }
    backend.searchGear(text, jobNow).then((found) => {
      if (text === gearText) gearFound = found.filter((i) => i.slot === slot).slice(0, GEAR_SHOWN);
    });
  });

  function openFinder(slot: string) {
    adding = adding === slot ? null : slot;
    gearText = "";
  }

  function addGoal(id: number) {
    if (draft && !draft.plan.gear.includes(id)) draft.plan.gear.push(id);
  }

  function dropGoal(id: number) {
    if (draft) draft.plan.gear = draft.plan.gear.filter((g) => g !== id);
  }

  // Settings.
  function setStartStat(stat: BaseStat, value: string) {
    if (!draft) return;
    draft.start.stats ??= { str: 0, dex: 0, int: 0, luk: 0 };
    draft.start.stats[stat] = whole(value);
  }

  const haveLevel = (id: number) => draft?.start.skills.find((s) => s.skillId === id)?.level ?? 0;
  const spent = $derived(
    (draft?.start.skills ?? [])
      .filter((s) => s.level > 0)
      .map((s) => `${skillOf(s.skillId)?.name ?? "A skill"} ${s.level}`)
      .join(", "),
  );

  function setHave(id: number, value: string) {
    if (!draft) return;
    const have = whole(value);
    draft.start.skills = draft.start.skills.filter((s) => s.skillId !== id);
    if (have) draft.start.skills.push({ skillId: id, level: have });
  }

  function setRules(change: Partial<BuildRules>) {
    const next = { ...display.buildRules, ...change };
    display.buildRules = next;
    clearTimeout(rulesTimer);
    rulesTimer = setTimeout(() => changeSettings({ buildRules: next }), SAVE_DELAY);
  }

  const rulesChanged = $derived(JSON.stringify(display.buildRules) !== JSON.stringify(DEFAULT_RULES));
  const pathSummary = $derived(
    draft
      ? `${jobPath} · ${draft.character ? `${draft.character}, ` : ""}Lv ${draft.start.level} · planned to Lv ${draft.plan.planTo}`
      : "",
  );
</script>

<svelte:head><title>Leafline — Build planner</title></svelte:head>

{#snippet moveButtons(onmove: (by: number) => void, first: boolean, last: boolean)}
  <span class="moves">
    <button class="icon" title="Move up" aria-label="Move up" disabled={first} onclick={() => onmove(-1)}><ArrowUp size={12} /></button>
    <button class="icon" title="Move down" aria-label="Move down" disabled={last} onclick={() => onmove(1)}><ArrowDown size={12} /></button>
  </span>
{/snippet}

{#snippet reachedBadge(reached: Reached, verb?: string)}
  {@const r = reachedText(reached, verb)}
  <span class="reached tabular {r.tone}">{r.text}</span>
{/snippet}

<div class="page" class:setup={creating || !draft}>
  <nav aria-label="Builds">
    <div class="brand"><Milestone size={16} /><span>Builds</span></div>
    <div class="list">
      {#each summaries as summary (summary.id)}
        <button class="build" class:active={!creating && draft?.id === summary.id} onclick={() => open(summary.id)}>
          <span class="build-name">{summary.name}</span>
          {#if summary.character}<span class="build-who">{summary.character}</span>{/if}
        </button>
      {/each}
    </div>
    {#if ready}
      <button class="side" class:active={creating} onclick={() => (creating = true)}><Plus size={13} />New build</button>
      <button class="side" onclick={() => (pasting = !pasting)}><ClipboardPaste size={13} />Paste a plan code</button>
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
  {:else if creating && catalog}
    <NewBuild
      {catalog}
      {starters}
      {character}
      {level}
      {hudJob}
      canCancel={!!draft}
      onCreated={(plan) => {
        show(plan);
        pickTab("skills");
      }}
      onCancel={() => (creating = false)}
    />
  {:else if !draft}
    <main class="center">{#if !loaded}<p class="lead">Loading…</p>{/if}</main>
  {:else}
    <main>
      <header class="top">
        <div class="titles">
          <input class="name" maxlength="80" aria-label="Build name" bind:value={draft.name} />
          <p class="path">{pathSummary}</p>
        </div>
        <button class="plain" onclick={copyCode} title="Copy this plan as a line of text: your goals only, not your character or stats.">
          <ClipboardCopy size={13} />{copied ? "Copied" : "Copy plan code"}
        </button>
      </header>

      <div class="tabs" role="tablist" aria-label="Plan">
        {#each TABS as t (t.id)}
          <button role="tab" aria-selected={tab === t.id} onclick={() => pickTab(t.id)}>{t.label}</button>
        {/each}
      </div>

      {#if error}<p class="error" role="alert">{error}</p>{/if}

      <div class="content">
        {#if tab === "skills"}
          <p class="help">SP goes to the first goal that can take it. A skill can come back later in the list.</p>
          {#each tiers as group (group.tier)}
            <h2 class="group">{group.name} <span class="faint">· {group.span}</span></h2>
            <ol class="goals">
              {#each group.goals as { goal, index }, k (index)}
                {@const skill = skillOf(goal.skillId)}
                <li>
                  {@render moveButtons((by) => moveSkill(index, by), k === 0, k === group.goals.length - 1)}
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
                  {@render reachedBadge(skillReached(goal, draft.start.skills, steps))}
                  <button class="icon" title="Remove" aria-label="Remove {skill?.name ?? 'skill'}" onclick={() => draft?.plan.skills.splice(index, 1)}
                    ><X size={12} /></button
                  >
                </li>
              {/each}
            </ol>
          {:else}
            <p class="quiet">No skill goals yet.</p>
          {/each}
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
          {#if !firstJob}<p class="help">Choose a job in Settings to add its skills.</p>{/if}
        {:else if tab === "stats"}
          <p class="help">Each level's AP goes to the first goal that can take it. End with one goal that takes the rest.</p>
          {#if draft.plan.stats.length}
            <ol class="goals">
              {#each draft.plan.stats as goal, i}
                <li>
                  {@render moveButtons((by) => draft && move(draft.plan.stats, i, by), i === 0, i === draft.plan.stats.length - 1)}
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
                  <label class="inline">
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
                  <span class="goal-name"></span>
                  {#if goal.target != null}
                    {@render reachedBadge(statReached(goal, draft.start.stats, steps), "reaches it at")}
                  {:else}
                    <span class="reached tabular normal">{stepAt(steps, endLevel)?.stats[goal.stat] ?? "?"} by Lv {endLevel}</span>
                  {/if}
                  <button class="icon" title="Remove" aria-label="Remove {statLabel(goal.stat)} goal" onclick={() => draft?.plan.stats.splice(i, 1)}
                    ><X size={12} /></button
                  >
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

          {#if splitTotal}
            <h2 class="group">Where your AP goes, Lv {draft.start.level} to {endLevel}</h2>
            <div class="split" role="img" aria-label={split.map((s) => `${statLabel(s.stat)} ${s.amount} AP`).join(", ")}>
              {#each split as part (part.stat)}<span style:flex={part.amount} style:background={STAT_COLOURS[part.stat]}></span>{/each}
            </div>
            <p class="legend">
              {#each split as part (part.stat)}
                <span><i style:background={STAT_COLOURS[part.stat]}></i>{statLabel(part.stat)} {part.amount} AP</span>
              {/each}
            </p>
          {/if}

          {#if milestones.length}
            <h2 class="group">Your stats along the way</h2>
            <table class="along tabular">
              <thead><tr><th>Level</th>{#each STATS as stat (stat.id)}<th>{stat.label}</th>{/each}</tr></thead>
              <tbody>
                {#each milestones as m (m.level)}
                  <tr class:now={m.label.includes("now")}>
                    <td>{m.label}</td>
                    {#each STATS as stat (stat.id)}<td>{m.stats?.[stat.id] ?? "?"}</td>{/each}
                  </tr>
                {/each}
              </tbody>
            </table>
          {/if}
        {:else if tab === "gear"}
          <p class="help">
            What you aim for in each slot, in the order this plan can wear it. The first one ahead shows as Next in Character →
            Gear; a slot with none gets the data's next better piece there.
          </p>
          <ul class="slots">
            {#each gearSlots as slot (slot)}
              {@const worn = wearing(slot)}
              {@const goals = goalsBySlot.get(slot) ?? []}
              <li>
                <span class="slot-name">{slotName(slot)}</span>
                <div class="path">
                  {#if worn}
                    <span class="chip worn">
                      <b>now</b>
                      <button class="ref ref-item" class:upgraded={!!worn.upgrades} onclick={() => backend.showItem(worn.item, worn.item == null ? worn.name : null)}
                        >{worn.name}{#if worn.upgrades}{` (+${worn.upgrades})`}{/if}</button
                      >
                    </span>
                  {:else}
                    <span class="faint">Nothing worn</span>
                  {/if}
                  {#each goals as check (check.item.id)}
                    <span class="chip">
                      <b class:warn={!check.usable || check.level == null}
                        >{#if !check.usable}wrong job{:else if check.level != null}{check.level}{:else}out of reach{/if}</b
                      >
                      <button class="ref ref-item" onclick={() => backend.showItem(check.item.id, null)}>{check.item.name}</button>
                      <button class="icon small" aria-label="Stop aiming for {check.item.name}" onclick={() => dropGoal(check.item.id)}><X size={11} /></button>
                    </span>
                  {/each}
                  <button class="add-chip" class:on={adding === slot} aria-expanded={adding === slot} onclick={() => openFinder(slot)}>
                    <Plus size={11} />Add
                  </button>
                </div>
                {#if adding === slot}
                  <div class="finder">
                    <div class="finder-head">
                      <label class="inline">
                        <span>Best at Lv</span>
                        <input
                          class="number"
                          type="number"
                          min="1"
                          max="200"
                          value={finderAt}
                          oninput={(e) => (finderLevel = Math.max(1, whole(e.currentTarget.value)))}
                        />
                      </label>
                      <label class="search">
                        <Search size={13} />
                        <input placeholder="Or type a name" aria-label="Find {slotName(slot)} by name" bind:value={gearText} />
                      </label>
                    </div>
                    {#if gearFound.length}
                      <ul class="found">
                        {#each gearFound as item (item.id)}
                          <li>
                            <button disabled={draft.plan.gear.includes(item.id)} onclick={() => addGoal(item.id)}>
                              <span>{item.name}</span>
                              <small>Lv {item.reqLevel} · {gives(item as ItemRecord, 3)}</small>
                            </button>
                          </li>
                        {/each}
                      </ul>
                    {:else}
                      <GearPicks build={draft.id} level={finderAt} {slot} full onAdd={(item) => addGoal(item.id)} />
                    {/if}
                  </div>
                {/if}
              </li>
            {/each}
          </ul>
        {:else}
          <section>
            <h2 class="group">Where you are now</h2>
            <p class="help">
              Leafline keeps these up to date while your Character Stat and Skill windows are open{keyHint("ReadCharacter")}. Type
              here only to plan from somewhere else.
            </p>
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
              <label><span>AP left</span><input class="number" type="number" min="0" bind:value={draft.start.ap} /></label>
              <label><span>SP left</span><input class="number" type="number" min="0" bind:value={draft.start.sp} /></label>
            </div>
            {#if pathSkills.length}
              <details>
                <summary>Skill points already spent{spent ? `: ${spent}` : ""}</summary>
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
            <h2 class="group">Job and character</h2>
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
                <select value={secondJob} disabled={!firstJob} onchange={(e) => setJob(Number(e.currentTarget.value) || firstJob)}>
                  <option value={0}>Not chosen yet</option>
                  {#each secondJobs as choice (choice.id)}<option value={choice.id}>{choice.name}</option>{/each}
                </select>
              </label>
              <label><span>Plan to Lv</span><input class="number" type="number" min="1" max="200" bind:value={draft.plan.planTo} /></label>
              <label>
                <span>For character</span>
                <input
                  maxlength="30"
                  placeholder="Any"
                  value={draft.character ?? ""}
                  oninput={(e) => draft && (draft.character = e.currentTarget.value || null)}
                />
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
          </section>

          {#if jobPlans.length}
            <section>
              <h2 class="group">Ready-made plans</h2>
              <p class="help">Written for Leafline from what each skill does and what each weapon needs; untested in game. Using one replaces your goals; your stats stay.</p>
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
            <h2 class="group">Game rules <span class="faint">· every build uses them</span></h2>
            <p class="help">AP and Beginner SP at Lv 2 were seen in game; the rest come from community guides. Change them if Classic shows otherwise.</p>
            <div class="fields">
              <label><span>AP a level</span><input class="number" type="number" min="0" value={rules.apPerLevel} oninput={(e) => setRules({ apPerLevel: whole(e.currentTarget.value) })} /></label>
              <label><span>SP a level</span><input class="number" type="number" min="0" value={rules.spPerLevel} oninput={(e) => setRules({ spPerLevel: whole(e.currentTarget.value) })} /></label>
              <label><span>Beginner SP a level</span><input class="number" type="number" min="0" value={rules.beginnerSpPerLevel} oninput={(e) => setRules({ beginnerSpPerLevel: whole(e.currentTarget.value) })} /></label>
              <label><span>SP at each advancement</span><input class="number" type="number" min="0" value={rules.advancementSp} oninput={(e) => setRules({ advancementSp: whole(e.currentTarget.value) })} /></label>
              <label><span>1st job at Lv</span><input class="number" type="number" min="2" value={rules.firstJobLevel} oninput={(e) => setRules({ firstJobLevel: whole(e.currentTarget.value) })} /></label>
              <label><span>2nd job at Lv</span><input class="number" type="number" min="2" value={rules.secondJobLevel} oninput={(e) => setRules({ secondJobLevel: whole(e.currentTarget.value) })} /></label>
            </div>
            <label class="check">
              <input type="checkbox" checked={rules.gearCounts} onchange={(e) => setRules({ gearCounts: e.currentTarget.checked })} />
              <span>Stats from other worn gear count toward requirements</span>
            </label>
            <label class="check">
              <input type="checkbox" checked={rules.secondSpBuysFirst} onchange={(e) => setRules({ secondSpBuysFirst: e.currentTarget.checked })} />
              <span>2nd-job SP can buy 1st-job skills</span>
            </label>
            {#if rulesChanged}
              <button class="plain" onclick={() => setRules(DEFAULT_RULES)}><RotateCcw size={13} />Back to the defaults</button>
            {/if}
          </section>

          <section>
            <h2 class="group">Guide</h2>
            {#if jobGuides.length}
              <div class="links">
                {#each jobGuides as guide (guide.url)}
                  <button class="plain" onclick={() => backend.openGuide(guide.url)}>
                    <ExternalLink size={13} />{guide.job}, {guide.site} ({guide.levels})
                  </button>
                {/each}
              </div>
            {/if}
            <div class="guide">
              <input type="url" placeholder="https://" aria-label="Guide link" bind:value={draft.guide} />
              <button class="plain" disabled={!draft.guide.trim()} onclick={() => draft && backend.openGuide(draft.guide)}>
                <ExternalLink size={13} />Open
              </button>
            </div>
            <p class="help">The guide you follow. It stays on this PC; plan codes leave it out.</p>
          </section>

          <section class="actions">
            <button class="plain" onclick={copyCode}><ClipboardCopy size={13} />{copied ? "Copied" : "Copy plan code"}</button>
            <button class="plain" onclick={duplicate}><Plus size={13} />Duplicate build</button>
            <button class="plain push" class:danger={confirmDelete} onclick={remove}>
              <Trash2 size={13} />{confirmDelete ? "Really delete?" : "Delete build"}
            </button>
          </section>
        {/if}

        <button class="credit" onclick={() => backend.openLink(OSMS_URL)}>{OSMS_CREDIT}</button>
      </div>
    </main>

    <Timeline {expansion} goals={draft.plan.skills} {level} />
  {/if}
</div>

<style>
  :global(body) {
    background: var(--bg);
  }

  .page {
    display: grid;
    grid-template-columns: 176px minmax(0, 1fr) 320px;
    height: 100vh;
    overflow: hidden;
  }

  .page.setup {
    grid-template-columns: 176px minmax(0, 1fr);
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
    background: var(--accent-soft);
    color: var(--text);
  }

  .build-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .build-who {
    color: var(--faint);
    font-size: var(--text-xs);
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
    font-size: var(--text-sm);
    cursor: pointer;
  }

  .side:hover:not(:disabled),
  .side.active {
    background: rgba(255, 255, 255, 0.05);
    color: var(--text);
  }

  .side.active {
    color: var(--accent-strong);
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
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
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
    font-size: var(--text-xl);
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

  .top {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 16px 22px 10px;
  }

  .titles {
    flex: 1;
    min-width: 0;
  }

  input.name {
    width: 100%;
    padding: 0 0 1px;
    border: 0;
    border-bottom: 1px solid transparent;
    border-radius: 0;
    background: transparent;
    color: var(--text);
    font-family: var(--font-display);
    font-size: var(--text-xl);
    font-weight: 600;
  }

  input.name:hover {
    border-bottom-color: var(--border);
  }

  input.name:focus-visible {
    border-bottom-color: var(--accent);
  }

  .path {
    margin: 2px 0 0;
    color: var(--muted);
    font-size: var(--text-sm);
  }

  .tabs {
    display: flex;
    padding: 0 22px;
    border-bottom: 1px solid var(--border);
  }

  .tabs button {
    padding: 8px 14px;
    border: 0;
    background: transparent;
    color: var(--muted);
    font: inherit;
    cursor: pointer;
  }

  .tabs button:hover {
    color: var(--text);
  }

  .tabs button[aria-selected="true"] {
    color: var(--text);
    box-shadow: inset 0 -2px 0 var(--accent);
  }

  .content {
    flex: 1;
    min-height: 0;
    padding: 12px 22px 28px;
    overflow-y: auto;
  }

  section {
    padding: 4px 0 14px;
    border-bottom: 1px solid var(--border);
  }

  section + section {
    padding-top: 12px;
  }

  h2.group {
    margin: 14px 0 6px;
    font-family: var(--font-display);
    font-size: var(--text-md);
    font-weight: 400;
    color: var(--muted);
  }

  section > h2.group:first-child {
    margin-top: 0;
  }

  .faint {
    color: var(--faint);
  }

  .help {
    margin: 0 0 8px;
    color: var(--faint);
    font-size: var(--text-sm);
  }

  .quiet {
    color: var(--faint);
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
    font-size: var(--text-sm);
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
    font-size: var(--text-md);
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
    font-size: var(--text-sm);
    cursor: pointer;
  }

  summary:hover {
    color: var(--text);
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
    border-top: 1px solid var(--border);
  }

  .goal-name {
    display: grid;
    flex: 1;
    min-width: 0;
  }

  small {
    color: var(--faint);
    font-size: var(--text-xs);
    overflow-wrap: anywhere;
  }

  .max {
    flex: none;
    color: var(--faint);
    font-size: var(--text-sm);
  }

  .reached {
    flex: none;
    min-width: 88px;
    color: var(--muted);
    font-size: var(--text-sm);
    text-align: right;
  }

  .reached.good {
    color: var(--good);
  }

  .reached.warn {
    color: var(--warn);
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

  .icon.small {
    width: 16px;
    height: 16px;
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
    margin-top: 10px;
  }

  .add select {
    max-width: 300px;
  }

  .split {
    display: flex;
    height: 16px;
    max-width: 520px;
    overflow: hidden;
  }

  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: 14px;
    margin: 6px 0 0;
    color: var(--muted);
    font-size: var(--text-sm);
  }

  .legend i {
    display: inline-block;
    width: 8px;
    height: 8px;
    margin-right: 5px;
  }

  .along {
    border-collapse: collapse;
    min-width: 420px;
    font-family: var(--font-display);
  }

  .along th {
    padding: 4px 12px;
    color: var(--faint);
    font-size: var(--text-xs);
    font-weight: 400;
    text-align: right;
  }

  .along td {
    padding: 5px 12px;
    border-top: 1px solid var(--border);
    text-align: right;
  }

  .along th:first-child,
  .along td:first-child {
    padding-left: 0;
    text-align: left;
  }

  .along tr.now td {
    color: var(--accent-strong);
  }

  .slots {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .slots > li {
    display: grid;
    grid-template-columns: 76px minmax(0, 1fr);
    gap: 4px 12px;
    padding: 8px 0;
    border-top: 1px solid var(--border);
  }

  .slot-name {
    padding-top: 3px;
    color: var(--muted);
    font-family: var(--font-display);
  }

  .path {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 2px 6px 2px 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface);
  }

  .chip.worn {
    padding-right: 8px;
    border-color: var(--accent);
  }

  .chip b {
    color: var(--faint);
    font-family: var(--font-display);
    font-size: var(--text-xs);
    font-weight: 400;
  }

  .chip b.warn {
    color: var(--warn);
  }

  .chip .ref {
    padding: 0;
    border: 0;
    background: transparent;
    font-size: var(--text-sm);
    cursor: pointer;
  }

  .chip.worn .ref {
    color: var(--text);
  }

  .chip .ref.upgraded {
    color: var(--accent-strong);
  }

  .add-chip {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    padding: 3px 8px;
    border: 1px dashed var(--border-strong);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--muted);
    font-size: var(--text-sm);
    cursor: pointer;
  }

  .add-chip:hover,
  .add-chip.on {
    border-color: var(--accent);
    color: var(--accent-strong);
  }

  .finder {
    grid-column: 2;
    padding: 10px;
    background: var(--surface-2);
  }

  .finder-head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 12px;
    margin-bottom: 8px;
  }

  .search {
    display: flex;
    flex: 1;
    align-items: center;
    gap: 6px;
    min-width: 180px;
    max-width: 320px;
  }

  .search input {
    flex: 1;
  }

  .found {
    display: grid;
    gap: 1px;
    margin: 0;
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
    color: var(--text);
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
    max-width: 600px;
  }

  .guide input {
    flex: 1;
    min-width: 0;
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

  .actions {
    display: flex;
    gap: 8px;
    border-bottom: 0;
  }

  .push {
    margin-left: auto;
  }

  .plain {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 9px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--muted);
    font-size: var(--text-sm);
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

  .error {
    margin: 10px 22px 0;
    padding: 6px 10px;
    background: var(--bad-soft);
    color: var(--bad);
    font-size: var(--text-sm);
  }

  .credit {
    margin-top: 16px;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--faint);
    font-size: var(--text-xs);
    cursor: pointer;
  }

  .credit:hover {
    color: var(--muted);
  }
</style>
