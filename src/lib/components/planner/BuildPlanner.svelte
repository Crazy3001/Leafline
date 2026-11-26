<script lang="ts">
  // The build planner. A build is ordered goals, the way guides put them, whether copied from one
  // or the player's own. A new build starts with three choices; after that the plan is split into
  // Plan (the chart), Skills, Stats, Gear and Settings, with the levels beside every tab. Changes
  // save themselves.
  // It is its own window, and a column of the desk view (embedded), where a menu stands in for
  // the list of builds.
  import { tip } from "$lib/tip.svelte";
  import Pic from "$lib/ui/Pic.svelte";
  import Ref from "$lib/components/Ref.svelte";
  import {
    ArrowDown,
    ArrowUp,
    ClipboardCopy,
    ClipboardPaste,
    ExternalLink,
    GripVertical,
    Milestone,
    Plus,
    RotateCcw,
    Search,
    Trash2,
    TriangleAlert,
    X,
  } from "@lucide/svelte";
  import { onMount, tick, untrack } from "svelte";

  import { backend } from "$lib/api";
  import { STATS, buildState, statLabel, watchBuilds } from "$lib/builds.svelte";
  import GearPicks from "$lib/components/GearPicks.svelte";
  import MapDataPanel from "$lib/components/map/MapDataPanel.svelte";
  import LevelInspector from "$lib/components/planner/LevelInspector.svelte";
  import NewBuild from "$lib/components/planner/NewBuild.svelte";
  import PlanChart from "$lib/components/planner/PlanChart.svelte";
  import StarterCard from "$lib/components/planner/StarterCard.svelte";
  import Timeline from "$lib/components/planner/Timeline.svelte";
  import Segmented from "$lib/ui/Segmented.svelte";
  import { guideLinks } from "$lib/guides";
  import { gives } from "$lib/items.svelte";
  import { mapData, watchMapData } from "$lib/mapdata.svelte";
  import {
    STAT_COLOURS,
    apRuns,
    apSplit,
    goalLanes,
    laneState,
    skillReached,
    statReached,
    stepAt,
    type GoalLane,
    type LaneState,
    type Reached,
  } from "$lib/planner";
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

  let { embedded = false }: { embedded?: boolean } = $props();

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

  // Clicks leave the game focused; typing in a box takes focus while it lasts. The desk view
  // follows its own window's focus.
  onMount(() => (embedded ? undefined : watchTyping()));

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
    picked = null;
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
  type Tab = "plan" | "skills" | "stats" | "gear" | "settings";
  const TAB_KEY = "leafline.planner.tab";
  const TABS: { id: Tab; label: string }[] = [
    { id: "plan", label: "Plan" },
    { id: "skills", label: "Skills" },
    { id: "stats", label: "Stats" },
    { id: "gear", label: "Gear" },
    { id: "settings", label: "Settings" },
  ];
  let tab = $state<Tab>(storedTab());

  function storedTab(): Tab {
    try {
      const saved = localStorage.getItem(TAB_KEY);
      return TABS.some((t) => t.id === saved) ? (saved as Tab) : "plan";
    } catch {
      return "plan";
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
    // A plan that follows a guide brings its link, unless the build already has one.
    if (starter.source && !draft.guide.trim()) draft.guide = starter.source.url;
    // A Lv 1 character with no stats entered yet starts from the plan's roll.
    if (!draft.start.stats && draft.start.level <= 1) draft.start.stats = { ...starter.start };
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
  function moveSkill(index: number, by: number): number | null {
    if (!draft) return null;
    const list = draft.plan.skills;
    const tier = tierOf(list[index].skillId);
    let to = index + by;
    while (to >= 0 && to < list.length && tierOf(list[to].skillId) !== tier) to += by;
    if (to < 0 || to >= list.length) return null;
    [list[index], list[to]] = [list[to], list[index]];
    return to;
  }

  // Dragging a goal by its grip, among the goals of its own tier. Pointer events, since the app
  // window takes the browser's own drag and drop for files.
  let drag = $state<{ from: number; over: number } | null>(null);

  function grab(e: PointerEvent, index: number) {
    if (e.button !== 0) return;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    drag = { from: index, over: index };
  }

  function dragMove(e: PointerEvent) {
    if (!drag || !draft) return;
    const row = document.elementFromPoint(e.clientX, e.clientY)?.closest<HTMLElement>("[data-goal]");
    if (!row) return;
    const i = Number(row.dataset.goal);
    if (draft.plan.skills[i] && tierOf(draft.plan.skills[i].skillId) === tierOf(draft.plan.skills[drag.from].skillId)) drag.over = i;
  }

  function dragEnd() {
    if (drag && draft && drag.over !== drag.from) {
      const [goal] = draft.plan.skills.splice(drag.from, 1);
      draft.plan.skills.splice(drag.over, 0, goal);
    }
    drag = null;
  }

  async function gripKey(e: KeyboardEvent, index: number) {
    const by = e.key === "ArrowUp" ? -1 : e.key === "ArrowDown" ? 1 : 0;
    if (!by) return;
    e.preventDefault();
    const to = moveSkill(index, by);
    if (to == null) return;
    await tick();
    document.querySelector<HTMLElement>(`[data-grip="${to}"]`)?.focus();
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

  // The Plan tab: the chart, the goals in order, and the level picked on either.
  const planStart = $derived(steps[0]?.level ?? draft?.start.level ?? 1);
  let picked = $state<number | null>(null);
  /** The level shown beside the chart: the one picked, or the character's next. */
  const selected = $derived(Math.max(planStart, Math.min(endLevel, picked ?? (level != null ? level + 1 : planStart + 1))));
  const pick = (at: number) => (picked = at);
  const lanes = $derived(draft ? goalLanes(draft.plan.skills, draft.start.skills, steps, (id) => skillOf(id)?.name) : []);
  const apRows = $derived(apRuns(steps));
  const bands = $derived(
    [
      { tier: 0, from: 1, to: rules.firstJobLevel - 1, label: "Beginner" },
      { tier: 1, from: rules.firstJobLevel, to: rules.secondJobLevel - 1, label: `1st job · ${jobName(firstJob) || "not chosen"}` },
      { tier: 2, from: rules.secondJobLevel, to: endLevel, label: `2nd job · ${jobName(secondJob) || "not chosen"}` },
    ]
      .map((b) => ({ ...b, from: Math.max(b.from, planStart), to: Math.min(b.to, endLevel) }))
      .filter((b) => b.from <= b.to),
  );
  const planProblems = $derived([
    ...(expansion?.problems.filter((p) => p.severity === "warning") ?? []),
    ...(expansion?.problems.filter((p) => p.severity === "note") ?? []),
  ]);
  /** The goal that starts taking SP after the ones going now. */
  const nextLane = $derived(
    lanes
      .filter((l) => laneState(l, level) === "later" && l.spends.length)
      .sort((a, b) => a.spends[0].level - b.spends[0].level)[0]?.index ?? null,
  );

  function laneTag(lane: GoalLane, state: LaneState): string {
    if (state === "done") return "Done";
    if (state === "now") return "Now";
    if (state === "short") return "Short";
    return lane.index === nextLane ? "Next" : "";
  }

  function laneSpan(lane: GoalLane): string {
    const first = lane.spends[0]?.level;
    const last = lane.spends.at(-1)?.level;
    if (lane.reached.kind === "already") return "had it";
    if (first == null) return lane.reached.kind === "at" ? "covered" : "no SP";
    const span = first === last ? `Lv ${first}` : `Lv ${first}–${last}`;
    return lane.reached.kind === "never" ? `${span}, not all` : span;
  }

  function reachedText(reached: Reached, verb = "by"): { text: string; tone: "good" | "normal" | "warn" } {
    if (reached.kind === "already") return { text: "done", tone: "good" };
    if (reached.kind === "at") return { text: `${verb} Lv ${reached.level}`, tone: level != null && reached.level <= level ? "good" : "normal" };
    return { text: `not by Lv ${reached.end}`, tone: "warn" };
  }

  // Stats.
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

  /** Goals the plan can't use: for another job, or out of reach by its end. Shown after the path. */
  const stuckBySlot = $derived.by(() => {
    const by = new Map<string, GearCheck[]>();
    for (const check of expansion?.gear ?? []) {
      if (check.usable && check.level != null) continue;
      by.set(check.item.slot, [...(by.get(check.item.slot) ?? []), check]);
    }
    return by;
  });
  /** The outfit's slots, then any slot with goals it left out. */
  const gearSlots = $derived.by(() => {
    const slots = (outfit?.slots ?? []).map((s) => s.slot);
    for (const slot of stuckBySlot.keys()) if (!slots.includes(slot)) slots.push(slot);
    return slots;
  });
  const wearing = (slot: string) => outfit?.slots.find((s) => s.slot === slot)?.wearing ?? null;
  const pathOf = (slot: string) => outfit?.paths.find((p) => p.slot === slot) ?? null;
  /** The level the gear paths start from. */
  const gearFrom = $derived(outfit?.level ?? level ?? draft?.start.level ?? 1);

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

  /** Slots whose passed-over goals are shown; folded into a count otherwise. */
  let openPassed = $state<string[]>([]);
  const GENDERS = [
    { id: "", label: "Either" },
    { id: "male", label: "Male" },
    { id: "female", label: "Female" },
  ];
  /** The character's gender as the plan has it, or as the gear worn tells. */
  const gender = $derived(draft?.start.gender ?? outfit?.wornGender ?? null);
  const genderWord = $derived(gender === "male" ? "Male" : gender === "female" ? "Female" : null);
  const rulesChanged = $derived(JSON.stringify(display.buildRules) !== JSON.stringify(DEFAULT_RULES));
  const pathSummary = $derived(
    draft
      ? `${jobPath} · Lv ${draft.start.level}${genderWord ? ` · ${genderWord}` : ""} · planned to Lv ${draft.plan.planTo}`
      : "",
  );
</script>

{#snippet moveButtons(onmove: (by: number) => void, first: boolean, last: boolean)}
  <span class="moves">
    <button class="icon-btn xs icon" use:tip={"Move up"} aria-label="Move up" disabled={first} onclick={() => onmove(-1)}><ArrowUp size={12} /></button>
    <button class="icon-btn xs icon" use:tip={"Move down"} aria-label="Move down" disabled={last} onclick={() => onmove(1)}><ArrowDown size={12} /></button>
  </span>
{/snippet}

{#snippet inspector()}
  <LevelInspector
    {steps}
    {selected}
    start={planStart}
    end={endLevel}
    {level}
    startStats={draft?.start.stats ?? null}
    problems={expansion?.problems ?? []}
    {skillOf}
    onselect={pick}
  />
{/snippet}

{#snippet reachedBadge(reached: Reached, verb?: string)}
  {@const r = reachedText(reached, verb)}
  <span class="reached tabular {r.tone}">{r.text}</span>
{/snippet}

<div class="host" class:embedded>
<div class="page" class:setup={creating || !draft} class:embedded>
  {#if !embedded}
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
  {/if}

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
          {#if embedded}
            <select
              class="switch"
              aria-label="Build"
              value={draft.id}
              onchange={(e) => {
                const id = Number(e.currentTarget.value);
                if (id === 0) creating = true;
                else open(id);
              }}
            >
              {#each summaries as summary (summary.id)}<option value={summary.id}>{summary.name}</option>{/each}
              <option value={0}>+ New build</option>
            </select>
          {/if}
          <input class="name" maxlength="80" aria-label="Build name" bind:value={draft.name} />
          <p class="path">{pathSummary}</p>
          <!-- Which character the build follows: Leafline uses it whenever it reads that name. -->
          <p class="who">
            {#if draft.character}
              <span>Used when Leafline reads <b>{draft.character}</b></span>
            {:else}
              <span>For any character without a build of their own</span>
            {/if}
            {#if character && draft.character?.toLowerCase() !== character.toLowerCase()}
              <button class="btn sm" onclick={() => draft && (draft.character = character)}>Use for {character}</button>
            {/if}
            {#if draft.character}
              <button class="btn sm ghost" onclick={() => draft && (draft.character = null)}>Use for anyone</button>
            {/if}
          </p>
        </div>
        <div class="top-actions">
          <button class="btn" onclick={copyCode}>
            <ClipboardCopy size={13} />{copied ? "Copied" : "Copy plan code"}
          </button>
          <button class="icon-btn sm" aria-label="Duplicate build" use:tip={"Duplicate build"} onclick={duplicate}><Plus size={14} /></button>
          {#if confirmDelete}
            <button class="btn danger" onclick={remove}><Trash2 size={13} />Really delete?</button>
            <button class="btn ghost" onclick={() => (confirmDelete = false)}>Keep</button>
          {:else}
            <button class="icon-btn sm" aria-label="Delete build" use:tip={"Delete build"} onclick={remove}><Trash2 size={14} /></button>
          {/if}
        </div>
      </header>

      <div class="tabs" role="tablist" aria-label="Plan">
        {#each TABS as t (t.id)}
          <button role="tab" aria-selected={tab === t.id} onclick={() => pickTab(t.id)}>{t.label}</button>
        {/each}
      </div>

      {#if error}<p class="error" role="alert">{error}</p>{/if}

      <div class="content">
        {#if tab === "plan"}
          {#if planProblems.length}
            <ul class="plan-problems">
              {#each planProblems as problem (problem.message)}
                <li class={problem.severity}>
                  {#if problem.severity === "warning"}<TriangleAlert size={13} />{/if}
                  <span>{problem.message}</span>
                  {#if problem.level != null}
                    <button class="btn sm" onclick={() => pick(problem.level!)}>Show Lv {problem.level}</button>
                  {/if}
                </li>
              {/each}
            </ul>
          {/if}

          <div class="card plan-card">
            <PlanChart
              {lanes}
              ap={apRows}
              {steps}
              start={planStart}
              end={endLevel}
              {level}
              {bands}
              problems={expansion?.problems ?? []}
              {selected}
              onselect={pick}
            />
          </div>

          <div class="card plan-card">
            <div class="order-head">
              <h2>Skill goals, in order</h2>
              <span class="help">SP goes to the first goal that can take it. Drag a goal by its grip to change the order.</span>
            </div>
            {#each tiers as group (group.tier)}
              <h3 class="order-group">{group.name} <span class="faint">· {group.span}</span></h3>
              <ol class="order">
                {#each group.goals as { goal, index }, k (index)}
                  {@const lane = lanes[index]}
                  {@const skill = skillOf(goal.skillId)}
                  {@const state = lane ? laneState(lane, level) : "later"}
                  {@const had = haveLevel(goal.skillId)}
                  {@const first = lane?.spends[0]?.level ?? null}
                  <li
                    data-goal={index}
                    class={state}
                    class:dragging={drag?.from === index}
                    class:drop-before={drag != null && drag.over === index && drag.over < drag.from}
                    class:drop-after={drag != null && drag.over === index && drag.over > drag.from}
                  >
                    <button
                      class="grip"
                      data-grip={index}
                      aria-label="Move {skill?.name ?? 'this goal'}: drag it, or press the up and down arrow keys"
                      use:tip={"Drag to change the order"}
                      onpointerdown={(e) => grab(e, index)}
                      onpointermove={dragMove}
                      onpointerup={dragEnd}
                      onpointercancel={() => (drag = null)}
                      onkeydown={(e) => gripKey(e, index)}><GripVertical size={14} /></button
                    >
                    <span class="num tabular">{k + 1}</span>
                    <span class="goal-name">
                      <span class="with-icon"
                        >{#if skill}<Ref kind="skill" id={goal.skillId} name={skill.name} />{:else}Unknown skill {goal.skillId}{/if}{#if had && state !== "done"}<small
                            >you have {had}</small
                          >{/if}</span
                      >
                    </span>
                    <span class="to tabular">to {goal.level} of {skill?.maxLevel ?? "?"}</span>
                    <button class="when tabular" disabled={first == null} use:tip={first == null ? undefined : `Show Lv ${first}`} onclick={() => first != null && pick(first)}
                      >{lane ? laneSpan(lane) : ""}</button
                    >
                    <span class="tag {state}">{lane ? laneTag(lane, state) : ""}</span>
                  </li>
                {/each}
              </ol>
            {:else}
              <p class="quiet">No skill goals yet.</p>
            {/each}
            <p class="order-foot">
              <button class="btn" onclick={() => pickTab("skills")}><Plus size={13} />Add or change goals in Skills</button>
              <button class="btn" onclick={() => pickTab("stats")}>Stat goals in Stats</button>
            </p>
          </div>
        {:else if tab === "skills"}
          <p class="help">SP goes to the first goal that can take it. A skill can come back later in the list.</p>
          {#each tiers as group (group.tier)}
            <h2 class="group">{group.name} <span class="faint">· {group.span}</span></h2>
            <ol class="goals">
              {#each group.goals as { goal, index }, k (index)}
                {@const skill = skillOf(goal.skillId)}
                <li>
                  {@render moveButtons((by) => moveSkill(index, by), k === 0, k === group.goals.length - 1)}
                  <span class="goal-name">
                    <span class="with-icon"
                      >{#if skill}<Ref kind="skill" id={goal.skillId} name={skill.name} />{:else}Unknown skill {goal.skillId}{/if}</span
                    >
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
                  <button class="icon-btn xs icon" use:tip={"Remove"} aria-label="Remove {skill?.name ?? 'skill'}" onclick={() => draft?.plan.skills.splice(index, 1)}
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
          <p class="help">
            Each level's AP goes to the first goal that can take it. End with one goal that takes the rest. A goal "with gear"
            counts what your worn gear and the plan's gear add, so no AP goes where gear already covers it.
          </p>
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
                  {#if goal.target != null}
                    <label class="inline" use:tip={"Count what worn gear adds, as the game does for requirements"}>
                      <input type="checkbox" checked={goal.withGear ?? false} onchange={(e) => (goal.withGear = e.currentTarget.checked)} />
                      <span>with gear</span>
                    </label>
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
                  <button class="icon-btn xs icon" use:tip={"Remove"} aria-label="Remove {statLabel(goal.stat)} goal" onclick={() => draft?.plan.stats.splice(i, 1)}
                    ><X size={12} /></button
                  >
                </li>
              {/each}
            </ol>
          {/if}
          <button
            class="btn"
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
            Each slot from what you wear to Lv {draft.plan.planTo}: the best piece you can wear now, then one piece for each
            level tier the data has for this plan, each better than the last. Add a piece to pick it over the data's choice
            for its tier. The first one ahead shows as Next in Character → Gear.
          </p>
          <div class="gender">
            <span class="faint">Gear made for</span>
            <Segmented
              label="Gear made for"
              size="sm"
              options={GENDERS}
              value={draft.start.gender ?? ""}
              onchange={(g) => draft && (draft.start.gender = g || null)}
            />
            {#if !draft.start.gender && outfit?.wornGender}
              <span class="faint">Your worn gear is {outfit.wornGender}-only, so the path keeps to {outfit.wornGender} gear.</span>
            {/if}
          </div>
          <ul class="slots">
            {#each gearSlots as slot (slot)}
              {@const worn = wearing(slot)}
              {@const path = pathOf(slot)}
              <li>
                <span class="slot-name">{slotName(slot)}</span>
                <div class="path">
                  {#if worn}
                    <span class="chip worn" use:tip={"What you wear"}>
                      <b>worn</b>
                      <Ref kind="item" id={worn.item} name={worn.name} class={worn.upgrades ? "upgraded" : ""}
                        >{worn.name}{#if worn.upgrades}{` (+${worn.upgrades})`}{/if}</Ref
                      >
                    </span>
                  {:else}
                    <span class="faint">Nothing worn</span>
                  {/if}
                  {#each [...(path?.now ? [path.now] : []), ...(path?.steps ?? [])] as step, i (step.item.id)}
                    {@const now = i === 0 && path?.now != null}
                    <span
                      class="chip"
                      class:data={!step.planned}
                      class:now
                      use:tip={step.planned ? "One of your goals" : "The data's pick for this tier. Add another piece of the tier to pick it instead."}
                    >
                      <b>{now ? "now" : step.level}</b>
                      <Ref kind="item" id={step.item.id} name={step.item.name} />
                      {#if step.planned}
                        <button class="icon-btn xs icon small" aria-label="Stop aiming for {step.item.name}" onclick={() => dropGoal(step.item.id)}><X size={11} /></button>
                      {/if}
                    </span>
                  {/each}
                  {#if path?.passed.length && !openPassed.includes(slot)}
                    <button
                      class="chip more"
                      use:tip={"Goals the path skips: outgrown, or a better piece comes as soon"}
                      onclick={() => (openPassed = [...openPassed, slot])}
                    >
                      {path.passed.length} passed over
                    </button>
                  {:else}
                    {#each path?.passed ?? [] as item (item.id)}
                      {@const below = item.reqLevel <= gearFrom}
                      <span class="chip passed" use:tip={below ? `Needs Lv ${item.reqLevel}: you're past it, and the path has a better piece` : "A better piece of this tier or a higher one comes as soon"}>
                        <b class="warn">{below ? `below Lv ${gearFrom}` : "passed over"}</b>
                        <Ref kind="item" id={item.id} name={item.name} />
                        <button class="icon-btn xs icon small" aria-label="Stop aiming for {item.name}" onclick={() => dropGoal(item.id)}><X size={11} /></button>
                      </span>
                    {/each}
                  {/if}
                  {#each stuckBySlot.get(slot) ?? [] as check (check.item.id)}
                    <span class="chip passed">
                      <b class="warn">{check.usable ? "out of reach" : "wrong job"}</b>
                      <Ref kind="item" id={check.item.id} name={check.item.name} />
                      <button class="icon-btn xs icon small" aria-label="Stop aiming for {check.item.name}" onclick={() => dropGoal(check.item.id)}><X size={11} /></button>
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
                      <span class="with-icon"><Pic kind="skill" id={skill.id} size={18} />{skill.name}</span>
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
              <p class="help">Using one replaces your goals; your stats stay.</p>
              <div class="plans">
                {#each jobPlans as starter (starter.id)}
                  <StarterCard plan={starter} jobName={jobName(starter.job)}>
                    {#snippet action()}
                      <button class="btn sm" class:danger={confirmStarter === starter.id} onclick={() => useStarter(starter)}>
                        {confirmStarter === starter.id ? "Replace my goals?" : "Use this plan"}
                      </button>
                    {/snippet}
                  </StarterCard>
                {/each}
              </div>
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
            {#if rulesChanged}
              <button class="btn" onclick={() => setRules(DEFAULT_RULES)}><RotateCcw size={13} />Back to the defaults</button>
            {/if}
          </section>

          <section>
            <h2 class="group">Guide</h2>
            {#if jobGuides.length}
              <div class="links">
                {#each jobGuides as guide (guide.url)}
                  <button class="btn" onclick={() => backend.openGuide(guide.url)}>
                    <ExternalLink size={13} />{guide.job}, {guide.site} ({guide.levels})
                  </button>
                {/each}
              </div>
            {/if}
            <div class="guide">
              <input type="url" placeholder="https://" aria-label="Guide link" bind:value={draft.guide} />
              <button class="btn" disabled={!draft.guide.trim()} onclick={() => draft && backend.openGuide(draft.guide)}>
                <ExternalLink size={13} />Open
              </button>
            </div>
            <p class="help">The guide you follow. It stays on this PC; plan codes leave it out.</p>
          </section>

          <section class="actions">
            <button class="btn" onclick={copyCode}><ClipboardCopy size={13} />{copied ? "Copied" : "Copy plan code"}</button>
            <button class="btn" onclick={duplicate}><Plus size={13} />Duplicate build</button>
            <button class="btn push" class:danger={confirmDelete} onclick={remove}>
              <Trash2 size={13} />{confirmDelete ? "Really delete?" : "Delete build"}
            </button>
          </section>
        {/if}

      </div>
    </main>

    <Timeline
      {expansion}
      goals={draft.plan.skills}
      {level}
      selected={tab === "plan" ? selected : null}
      onselect={(at) => {
        pick(at);
        pickTab("plan");
      }}
      top={tab === "plan" ? inspector : undefined}
      showProblems={tab !== "plan"}
    />
  {/if}
</div>
</div>

<style>
  .page {
    display: grid;
    grid-template-columns: 176px minmax(0, 1fr) 320px;
    height: calc(100vh / var(--page-zoom, 1));
    overflow: hidden;
  }

  .page.setup {
    grid-template-columns: 176px minmax(0, 1fr);
  }

  .host.embedded {
    height: 100%;
    container: planner / inline-size;
  }

  /* A desk view column: its own height, no list of builds, a narrower timeline. */
  .page.embedded {
    grid-template-columns: minmax(0, 1fr) 280px;
    height: 100%;
  }

  .page.embedded.setup {
    grid-template-columns: minmax(0, 1fr);
  }

  .page.embedded :global(.setup),
  .page.embedded main.center {
    grid-column: 1 / -1;
  }

  /* A narrow desk column: the timeline goes under the tabs, and the whole column scrolls. */
  @container planner (max-width: 640px) {
    .page.embedded {
      display: block;
      overflow-y: auto;
    }

    .page.embedded .content {
      overflow: visible;
    }

    .page.embedded :global(aside) {
      border-top: 1px solid var(--border);
      border-left: 0;
      overflow: visible;
    }
  }

  select.switch {
    margin-bottom: var(--space-1);
    max-width: 100%;
  }

  nav {
    display: flex;
    flex-direction: column;
    gap: var(--space-0-5);
    min-height: 0;
    padding: var(--space-4) var(--space-3);
    border-right: 1px solid var(--border);
    background: var(--surface);
  }

  .brand {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin: 0 var(--space-2) var(--space-3);
    font-family: var(--font-display);
    font-weight: var(--weight-strong);
  }

  .list {
    display: grid;
    align-content: start;
    gap: 1px;
    min-height: 0;
    margin-bottom: var(--space-2);
    overflow-y: auto;
  }

  .build {
    display: grid;
    gap: 0;
    width: 100%;
    padding: var(--space-1-5) var(--space-3);
    border: 0;
    border-left: 2px solid transparent;
    border-radius: 0;
    background: transparent;
    color: var(--muted);
    text-align: left;
    cursor: pointer;
  }

  .build:hover {
    background: var(--hover);
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
    gap: var(--space-1-5);
    padding: var(--space-1-5) var(--space-3);
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--muted);
    font-size: var(--text-md);
    cursor: pointer;
  }

  .side:hover:not(:disabled),
  .side.active {
    background: var(--hover);
    color: var(--text);
  }

  .side.active {
    color: var(--accent-text-strong);
  }

  .side:disabled {
    color: var(--faint);
    cursor: default;
  }

  textarea {
    width: 100%;
    margin: var(--space-1) 0;
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
    padding: var(--space-6);
  }

  h1 {
    margin: 0 0 var(--space-1-5);
    font-family: var(--font-display);
    font-size: var(--text-xl);
    font-weight: var(--weight-strong);
  }

  .lead {
    margin: 0 0 var(--space-3);
    color: var(--muted);
  }

  .box {
    padding: var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
  }

  .top {
    display: flex;
    align-items: flex-start;
    gap: var(--space-3);
    padding: var(--space-4) var(--space-6) var(--space-2);
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
    font-weight: var(--weight-strong);
  }

  input.name:hover {
    border-bottom-color: var(--border);
  }

  input.name:focus-visible {
    border-bottom-color: var(--accent);
  }

  .path {
    margin: var(--space-0-5) 0 0;
    color: var(--muted);
    font-size: var(--text-md);
  }

  .who {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-1-5) var(--space-2);
    margin: var(--space-1) 0 0;
    color: var(--faint);
    font-size: var(--text-md);
  }

  .who b {
    color: var(--text);
    font-weight: var(--weight-strong);
  }

  .top-actions {
    display: flex;
    flex: none;
    align-items: center;
    gap: var(--space-1-5);
  }

  .tabs {
    display: flex;
    padding: 0 var(--space-6);
    border-bottom: 1px solid var(--border);
  }

  .tabs button {
    padding: var(--space-2) var(--space-4);
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
    padding: var(--space-3) var(--space-6) var(--space-6);
    overflow-y: auto;
  }

  section {
    padding: var(--space-1) 0 var(--space-3);
    border-bottom: 1px solid var(--border);
  }

  section + section {
    padding-top: var(--space-3);
  }

  h2.group {
    margin: var(--space-3) 0 var(--space-1-5);
    font-family: var(--font-display);
    font-size: var(--text-md);
    font-weight: var(--weight-body);
    color: var(--muted);
  }

  section > h2.group:first-child {
    margin-top: 0;
  }

  .faint {
    color: var(--faint);
  }

  .help {
    margin: 0 0 var(--space-2);
    color: var(--faint);
    font-size: var(--text-md);
  }

  .quiet {
    color: var(--faint);
  }

  .fields {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2) var(--space-3);
    margin-top: var(--space-1-5);
  }

  label {
    display: grid;
    gap: var(--space-0-5);
    color: var(--muted);
    font-size: var(--text-md);
  }

  label.inline {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    flex: none;
  }

  input,
  select,
  textarea {
    padding: var(--space-1) var(--space-2);
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
    margin-top: var(--space-2);
  }

  summary {
    color: var(--muted);
    font-size: var(--text-md);
    cursor: pointer;
  }

  summary:hover {
    color: var(--text);
  }

  .have {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(170px, 1fr));
    gap: var(--space-1-5) var(--space-3);
    margin-top: var(--space-2);
  }

  .have label {
    grid-template-columns: 1fr auto;
    align-items: center;
  }

  .goals {
    display: grid;
    margin: 0 0 var(--space-2);
    padding: 0;
    list-style: none;
  }

  .goals li {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-1) 0;
    border-top: 1px solid var(--border);
  }

  .goal-name {
    display: grid;
    flex: 1;
    min-width: 0;
  }

  .with-icon {
    display: flex;
    align-items: center;
    gap: var(--space-1-5);
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
    font-size: var(--text-md);
  }

  .reached {
    flex: none;
    min-width: 88px;
    color: var(--muted);
    font-size: var(--text-md);
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

  .icon.small {
    width: 16px;
    height: 16px;
  }

  .add {
    margin-top: var(--space-2);
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
    gap: var(--space-3);
    margin: var(--space-1-5) 0 0;
    color: var(--muted);
    font-size: var(--text-md);
  }

  .legend i {
    display: inline-block;
    width: 8px;
    height: 8px;
    margin-right: var(--space-1-5);
  }

  .along {
    border-collapse: collapse;
    min-width: 420px;
    font-family: var(--font-display);
  }

  .along th {
    padding: var(--space-1) var(--space-3);
    color: var(--faint);
    font-size: var(--text-xs);
    font-weight: var(--weight-body);
    text-align: right;
  }

  .along td {
    padding: var(--space-1) var(--space-3);
    border-top: 1px solid var(--border);
    text-align: right;
  }

  .along th:first-child,
  .along td:first-child {
    padding-left: 0;
    text-align: left;
  }

  .along tr.now td {
    color: var(--accent-text-strong);
  }

  .slots {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .slots > li {
    display: grid;
    grid-template-columns: 76px minmax(0, 1fr);
    gap: var(--space-1) var(--space-3);
    padding: var(--space-2) 0;
    border-top: 1px solid var(--border);
  }

  .slot-name {
    padding-top: var(--space-0-5);
    color: var(--muted);
    font-family: var(--font-display);
  }

  .path {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-1-5);
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1-5);
    padding: var(--space-0-5) var(--space-1-5) var(--space-0-5) var(--space-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface);
  }

  .gender {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-1-5) var(--space-2);
    margin: 0 0 var(--space-2);
    font-size: var(--text-md);
  }

  .chip.worn {
    padding-right: var(--space-2);
    border-color: var(--border-strong);
    background: var(--surface-2);
  }

  .chip.now {
    border-color: var(--accent);
  }

  /* The data's pick for a tier, not one of the plan's goals: no remove button, a dashed edge. */
  .chip.data {
    padding-right: var(--space-2);
    border-style: dashed;
  }

  .chip.passed {
    opacity: 0.75;
  }

  .chip.more {
    padding-right: var(--space-2);
    border-style: dashed;
    color: var(--faint);
    font: inherit;
    font-size: var(--text-xs);
    cursor: pointer;
  }

  .chip.more:hover {
    color: var(--text);
  }

  .chip b {
    color: var(--faint);
    font-family: var(--font-display);
    font-size: var(--text-xs);
    font-weight: var(--weight-body);
  }

  .chip b.warn {
    color: var(--warn);
  }

  .chip :global(.ref) {
    padding: 0;
    border: 0;
    background: transparent;
    font-size: var(--text-md);
    cursor: pointer;
  }

  .chip.worn :global(.ref) {
    color: var(--text);
  }

  .chip :global(.ref.upgraded:not(.worn)) {
    color: var(--accent-text-strong);
  }

  .add-chip {
    display: inline-flex;
    align-items: center;
    gap: var(--space-0-5);
    padding: var(--space-0-5) var(--space-2);
    border: 1px dashed var(--border-strong);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--muted);
    font-size: var(--text-md);
    cursor: pointer;
  }

  .add-chip:hover,
  .add-chip.on {
    border-color: var(--accent);
    color: var(--accent-text-strong);
  }


  .finder {
    grid-column: 2;
    padding: var(--space-2);
    background: var(--surface-2);
  }

  .finder-head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-3);
    margin-bottom: var(--space-2);
  }

  .search {
    display: flex;
    flex: 1;
    align-items: center;
    gap: var(--space-1-5);
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
    padding: var(--space-1) var(--space-2);
    border: 0;
    border-radius: var(--radius-sm);
    background: var(--hover);
    color: var(--text);
    text-align: left;
    cursor: pointer;
  }

  .found button:hover:not(:disabled) {
    background: var(--hover);
  }

  .found button:disabled {
    opacity: 0.45;
    cursor: default;
  }

  .guide {
    display: flex;
    gap: var(--space-1-5);
    max-width: 600px;
  }

  .guide input {
    flex: 1;
    min-width: 0;
  }

  .links {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1-5);
    margin-bottom: var(--space-2);
  }

  .plans {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
    gap: var(--space-3);
  }

  .actions {
    display: flex;
    gap: var(--space-2);
    border-bottom: 0;
  }

  .push {
    margin-left: auto;
  }

  .error {
    margin: var(--space-2) var(--space-6) 0;
    padding: var(--space-1-5) var(--space-3);
    background: var(--bad-soft);
    color: var(--bad);
    font-size: var(--text-md);
  }

  /* The Plan tab. */
  .plan-problems {
    display: grid;
    gap: var(--space-1-5);
    margin: 0 0 var(--space-3);
    padding: 0;
    list-style: none;
    font-size: var(--text-md);
  }

  .plan-problems li {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-1-5) var(--space-3);
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    color: var(--muted);
  }

  .plan-problems li.warning {
    background: var(--warn-soft);
    color: var(--warn);
  }

  .plan-problems li > span {
    flex: 1;
  }

  .plan-problems :global(svg) {
    flex: none;
  }

  .plan-card {
    margin-bottom: var(--space-3);
    padding: var(--space-3) var(--space-4) var(--space-3);
  }

  .order-head {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--space-1) var(--space-3);
    margin-bottom: var(--space-1);
  }

  .order-head h2 {
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--text-md);
    font-weight: var(--weight-strong);
  }

  .order-head .help {
    margin: 0;
    font-size: var(--text-xs);
  }

  .order-group {
    margin: var(--space-2) 0 var(--space-0-5);
    color: var(--muted);
    font-family: var(--font-display);
    font-size: var(--text-md);
    font-weight: var(--weight-body);
  }

  .order {
    display: grid;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .order li {
    display: grid;
    grid-template-columns: 22px 20px minmax(0, 1fr) auto 84px 56px;
    align-items: center;
    gap: var(--space-2);
    min-height: 34px;
    padding: 0 var(--space-1-5);
    border-top: 1px solid var(--border);
  }

  .order li.now {
    background: var(--accent-soft);
  }

  .order li.done .goal-name,
  .order li.done .num {
    opacity: 0.7;
  }

  .order li.dragging {
    opacity: 0.45;
  }

  .order li.drop-before {
    box-shadow: inset 0 2px 0 var(--accent);
  }

  .order li.drop-after {
    box-shadow: inset 0 -2px 0 var(--accent);
  }

  .grip {
    display: grid;
    place-items: center;
    width: 22px;
    height: 26px;
    padding: 0;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--faint);
    cursor: grab;
    touch-action: none;
  }

  .grip:hover,
  .grip:focus-visible {
    background: var(--hover);
    color: var(--text);
  }

  .dragging .grip {
    cursor: grabbing;
  }

  .num {
    color: var(--faint);
    font-family: var(--font-display);
    text-align: right;
  }

  .goal-name small {
    margin-left: var(--space-1-5);
  }

  .to {
    color: var(--muted);
    font-size: var(--text-md);
    white-space: nowrap;
  }

  .when {
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--muted);
    font-family: var(--font-display);
    font-size: var(--text-md);
    text-align: right;
    white-space: nowrap;
    cursor: pointer;
  }

  .when:hover:not(:disabled) {
    color: var(--accent-text);
    text-decoration: underline;
  }

  .when:disabled {
    color: var(--faint);
    cursor: default;
  }

  .tag {
    justify-self: end;
    padding: 1px var(--space-2);
    border-radius: 6px;
    font-size: var(--text-xs);
    font-weight: var(--weight-strong);
  }

  .tag:empty {
    display: none;
  }

  .tag.done {
    background: var(--good-soft);
    color: var(--good);
  }

  .tag.now {
    background: var(--accent);
    color: var(--accent-ink);
  }

  .tag.later {
    background: var(--surface-2);
    color: var(--muted);
  }

  .tag.short {
    background: var(--warn-soft);
    color: var(--warn);
  }

  .order-foot {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    margin: var(--space-3) 0 0;
  }

  @container planner (max-width: 640px) {
    .order li {
      grid-template-columns: 22px minmax(0, 1fr) auto 50px;
    }

    .order .num,
    .order .to {
      display: none;
    }
  }

  .credit {
    margin-top: var(--space-4);
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
