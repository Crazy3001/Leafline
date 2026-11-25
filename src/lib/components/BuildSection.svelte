<script lang="ts">
  // The build plan's part of the Character tab: what the plan says at the character's level. It
  // moves on with the level, and catches up with what was really spent whenever the stat and
  // skill windows are read. The level itself is shown once, in the character's header above.
  import { can } from "$lib/platform";
  import { clipped, tip } from "$lib/tip.svelte";
  import Pic from "$lib/ui/Pic.svelte";
  import Ref from "$lib/components/Ref.svelte";
  import { Check, ExternalLink, Pencil, Plus, RefreshCw, Search, X } from "@lucide/svelte";
  import { untrack } from "svelte";

  import AdvancementChecklist from "./AdvancementChecklist.svelte";
  import CharacterStats from "./CharacterStats.svelte";
  import MapDataPanel from "./map/MapDataPanel.svelte";
  import { backend } from "$lib/api";
  import { STATS, buildState, statLabel, watchBuilds } from "$lib/builds.svelte";
  import { mapData, watchMapData } from "$lib/mapdata.svelte";
  import { gives } from "$lib/items.svelte";
  import { openPlanner } from "$lib/nav.svelte";
  import { display, keyHint } from "$lib/settings.svelte";
  import type { BuildCatalog, BuildPlan, Fit, GearItem, GearPiece, ItemRecord, LevelStep, Outfit } from "$lib/types";
  import Segmented from "$lib/ui/Segmented.svelte";

  let {
    level,
    character = null,
    glance = false,
    whole = false,
  }: {
    /** Read from the HUD or the session. */
    level: number | null;
    character?: string | null;
    /** A pinned window's short form: this level's to-do and the next advancement, nothing to browse. */
    glance?: boolean;
    /** Room for every part at once, as in the desk view: no chips, the parts side by side. */
    whole?: boolean;
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
  /** The Character Stat window has been read, so the plan's stats start from the real ones. */
  const statsKnown = $derived(plan?.build.start.stats != null);
  /** What gear added to each stat when the window was last read. */
  const gearBonus = $derived(plan?.build.start.gearBonus ?? null);
  const at = $derived(level ?? plan?.build.start.level ?? 0);
  const step = $derived(steps.findLast((s) => s.level <= at) ?? steps[0] ?? null);
  const hasWork = (s: LevelStep) => s.ap.length > 0 || s.sp.length > 0 || s.advance != null;
  const coming = $derived(steps.filter((s) => s.level > at && hasWork(s)).slice(0, COMING));
  const advance = $derived(steps.find((s) => s.level > at && s.advance));
  /** The next level that brings a gear goal within reach. */
  const nextGear = $derived(steps.find((s) => s.level > at && s.gear.length));
  /** The advancement the checklist is for: the next one, or this level's until the next level. */
  const nextAdvance = $derived(steps.find((s) => s.level >= at && s.advance));
  const warnings = $derived(plan?.expansion.problems.filter((p) => p.severity === "warning") ?? []);
  const past = $derived(level != null && steps.length > 0 && level > steps[steps.length - 1].level);

  // What to spend, the next job advancement and gear each get a view of their own, so the tab
  // stays short. The one last picked is remembered.
  type View = "plan" | "stats" | "advance" | "gear" | "skills";
  const VIEW_KEY = "leafline.build.view";
  let view = $state<View>(storedView());
  const canAdvance = $derived(!!nextAdvance?.advance && !!plan?.build.plan.job);
  const views = $derived([
    { id: "plan" as View, label: "Level up" },
    { id: "stats" as View, label: "Stats" },
    { id: "gear" as View, label: "Gear" },
    { id: "skills" as View, label: "Skills" },
    ...(canAdvance ? [{ id: "advance" as View, label: "Job advance" }] : []),
  ]);

  // Gear, slot by slot: what is worn, the plan's next goal and a better pick. What is worn comes
  // from an item check, or from the search here for gear named rather than checked.
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

  let outfit = $state<Outfit | null>(null);
  const outfitKey = $derived(plan ? JSON.stringify([plan.build.id, at, buildState.version, display.buildRules]) : null);

  // Follows the key alone, as the plan does.
  $effect(() => {
    const asked = outfitKey;
    const id = untrack(() => plan?.build.id);
    if (asked == null || id == null) {
      outfit = null;
      return;
    }
    untrack(() => backend.buildOutfit(id, at))
      .then((result) => {
        if (asked === outfitKey) outfit = result;
      })
      .catch(() => {});
  });

  const worn = $derived(new Set((plan?.build.start.pieces ?? []).map((p) => p.item)));

  /** What the worn pieces add up to, stat by stat, as their tooltips spelled them. */
  const gearTotals = $derived.by(() => {
    const totals = new Map<string, number>();
    for (const piece of plan?.build.start.pieces ?? []) {
      for (const stat of piece.stats) totals.set(stat.label, (totals.get(stat.label) ?? 0) + stat.value);
    }
    return [...totals].filter(([, value]) => value !== 0).map(([label, value]) => ({ label, value }));
  });

  /** Slot names as the game's equip window shows them. */
  const SLOT_NAMES: Record<string, string> = { Glove: "Gloves", Earring: "Earrings" };
  const slotName = (slot: string) => SLOT_NAMES[slot] ?? slot;

  async function wear(id: number) {
    try {
      await backend.wearItem(id, character);
      gearText = "";
      goalError = null;
    } catch (e) {
      goalError = String(e);
    }
  }

  /** Stats an equip's tooltip can show, as it spells them. */
  const STAT_LABELS = [
    "STR",
    "DEX",
    "INT",
    "LUK",
    "MaxHP",
    "MaxMP",
    "Weapon Attack",
    "Magic Attack",
    "Weapon Def.",
    "Magic Def.",
    "Accuracy",
    "Avoidability",
    "Speed",
    "Jump",
  ];

  /** A copy of the piece being put right; saved only on Save. */
  let editing = $state<GearPiece | null>(null);
  let editingName = "";

  function startEdit(piece: GearPiece) {
    editing = structuredClone($state.snapshot(piece));
    editingName = piece.name;
  }

  async function saveEdit() {
    if (!editing) return;
    const draft = $state.snapshot(editing);
    draft.name = draft.name.trim() || editingName;
    draft.stats = draft.stats.filter((s) => Number.isFinite(s.value));
    const whole = (n: number | null) => (n == null || !Number.isFinite(n) ? null : Math.max(0, Math.round(n)));
    draft.upgrades = whole(draft.upgrades);
    draft.slotsLeft = whole(draft.slotsLeft);
    try {
      // A new name is a different item: match it again, as an item check would.
      if (draft.name !== editingName) {
        const found = await backend.findItem(draft.name, draft.level);
        draft.item = found?.id ?? null;
        draft.slots = found?.upgradeSlots ?? null;
        if (found) draft.itemType = found.weaponType ?? found.slot ?? draft.itemType;
      }
      await backend.updatePiece(draft, character);
      editing = null;
      goalError = null;
    } catch (e) {
      goalError = String(e);
    }
  }

  async function removePiece(id: number) {
    try {
      await backend.removePiece(id, character);
      goalError = null;
    } catch (e) {
      goalError = String(e);
    }
  }

  async function setGoal(id: number, wanted: boolean) {
    try {
      await backend.setGearGoal(id, wanted, character);
      goalError = null;
    } catch (e) {
      goalError = String(e);
    }
  }
  // Skills: what the skill window last showed, what the plan wants by its end, and when the next point goes in.
  let catalog = $state<BuildCatalog | null>(null);
  $effect(() => {
    if (shown !== "skills" || catalog) return;
    backend.buildCatalog().then((c) => (catalog = c)).catch(() => {});
  });
  const skillRows = $derived.by(() => {
    const info = new Map((catalog?.skills ?? []).map((k) => [k.id, k]));
    const now = new Map((plan?.build.start.skills ?? []).map((k) => [k.skillId, k.level]));
    const goal = new Map((plan?.expansion.skills ?? []).map((k) => [k.skillId, k.level]));
    // Every skill of the job and the jobs before it, so skills not yet touched show too.
    const job = plan?.build.plan.job ?? 0;
    const path = new Set([0, Math.floor(job / 100) * 100, job]);
    const ids = new Set([...now.keys(), ...goal.keys(), ...(catalog?.skills ?? []).filter((k) => path.has(k.jobId)).map((k) => k.id)]);
    return [...ids]
      .map((id) => {
        const next = steps.find((st) => st.level > at && st.sp.some((g) => g.skillId === id));
        const gain = next?.sp.find((g) => g.skillId === id);
        const name = info.get(id)?.name ?? steps.flatMap((st) => st.sp).find((g) => g.skillId === id)?.name ?? `Skill ${id}`;
        return {
          id,
          name,
          level: now.get(id) ?? 0,
          goal: goal.get(id) ?? now.get(id) ?? 0,
          max: info.get(id)?.maxLevel ?? null,
          passive: info.get(id)?.passive ?? false,
          description: info.get(id)?.description ?? null,
          tier: info.get(id)?.jobTier ?? "",
          className: info.get(id)?.className ?? "",
          nextLevel: next?.level ?? null,
          nextGain: gain ? gain.to - gain.from : null,
        };
      })
      .sort((a, b) => a.id - b.id);
  });
  type SkillRow = (typeof skillRows)[number];
  const isMaxed = (k: SkillRow) => k.max != null && k.level >= k.max;
  const TIERS = ["2nd Job", "1st Job", "Beginner"];
  /** Where the 2nd job's points end: the 3rd job takes over. */
  const THIRD_JOB_LEVEL = 70;
  /** Skills by job, the latest job first, in the order the game's skill window lists them. */
  const skillGroups = $derived.by(() => {
    const tiers = [...new Set(skillRows.map((k) => k.tier))].sort((a, b) => {
      const rank = (t: string) => (TIERS.includes(t) ? TIERS.indexOf(t) : TIERS.length);
      return rank(a) - rank(b);
    });
    return tiers
      .map((tier) => {
        const rows = skillRows.filter((k) => k.tier === tier);
        const title = tier ? `${tier}${rows[0]?.className && tier !== "Beginner" ? ` · ${rows[0].className}` : ""}` : "Other";
        // The points a job gives over its whole span, or what its skills can take if that is less.
        const r = display.buildRules;
        const given =
          tier === "Beginner"
            ? (r.firstJobLevel - 1) * r.beginnerSpPerLevel
            : tier === "1st Job"
              ? (r.secondJobLevel - r.firstJobLevel) * r.spPerLevel + r.advancementSp
              : tier === "2nd Job"
                ? (THIRD_JOB_LEVEL - r.secondJobLevel) * r.spPerLevel + r.advancementSp
                : Infinity;
        const room = rows.reduce((n, k) => n + (k.max ?? k.level), 0);
        return { title, rows, spent: rows.reduce((n, k) => n + k.level, 0), total: Math.min(room, given) };
      })
      .filter((g) => g.rows.length);
  });
  const skillsRead = $derived((plan?.build.start.skills?.length ?? 0) > 0);
  const shown = $derived(view === "advance" && !canAdvance ? "plan" : view);

  function storedView(): View {
    try {
      const saved = localStorage.getItem(VIEW_KEY);
      return saved === "advance" || saved === "skills" || saved === "stats" ? saved : saved === "gear" || saved === "worn" ? "gear" : "plan";
    } catch {
      return "plan";
    }
  }

  /** What a gear line's level says on hover: what falls short, or that it can be worn. */
  function fitTitle(fit: Fit | null | undefined): string | undefined {
    if (!fit) return undefined;
    return fit.ok ? "You meet its requirements" : `Needs ${fit.short.join(", ")}`;
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

{#snippet wearing(row: Outfit["slots"][number])}
  {@const piece = row.wearing}
            <div class="line">
              <span class="label">Wearing</span>
              {#if piece}
                <Ref kind="item" id={piece.item} name={piece.name} class={piece.upgrades ? "upgraded" : ""}
                  >{piece.name}{#if piece.upgrades}{` (+${piece.upgrades})`}{/if}</Ref
                >
                <span class="end">
                  {#if piece.level != null}<span class="lv tabular" class:good={row.wearingFit?.ok} class:bad={row.wearingFit && !row.wearingFit.ok} use:tip={fitTitle(row.wearingFit)}
                      >{piece.level ? `Lv ${piece.level}` : "Any level"}</span
                    >{/if}
                  <button class="remove" use:tip={"Edit"} aria-label="Edit {piece.name}" onclick={() => startEdit(piece)}
                    ><Pencil size={11} /></button
                  >
                  <button class="remove" use:tip={"Take it off"} aria-label="Take off {piece.name}" onclick={() => removePiece(piece.id)}
                    ><X size={12} /></button
                  >
                </span>
              {:else}
                <span class="none">Nothing worn</span>
              {/if}
            </div>
            {#if piece && editing?.id === piece.id}
              <div class="edit">
                <label class="edit-field">
                  <span>Name</span>
                  <input bind:value={editing.name} />
                </label>
                {#each editing.stats as stat, k}
                  <div class="stat-edit">
                    <select bind:value={stat.label} aria-label="Stat">
                      {#each STAT_LABELS as label (label)}<option>{label}</option>{/each}
                      {#if !STAT_LABELS.includes(stat.label)}<option>{stat.label}</option>{/if}
                    </select>
                    <input type="number" bind:value={stat.value} aria-label="{stat.label} value" />
                    <button class="remove" aria-label="Remove {stat.label}" onclick={() => editing?.stats.splice(k, 1)}
                      ><X size={11} /></button
                    >
                  </div>
                {/each}
                <button class="link add-stat" onclick={() => editing?.stats.push({ label: "STR", value: 0 })}
                  ><Plus size={11} />Add a stat</button
                >
                <div class="edit-pair">
                  <label class="edit-field">
                    <span>Scrolled (+N)</span>
                    <input type="number" min="0" max="30" bind:value={editing.upgrades} />
                  </label>
                  <label class="edit-field">
                    <span>Slots left</span>
                    <input type="number" min="0" max="30" bind:value={editing.slotsLeft} />
                  </label>
                </div>
                <div class="edit-actions">
                  <button class="action small" onclick={saveEdit}>Save</button>
                  <button class="link" onclick={() => (editing = null)}>Cancel</button>
                </div>
              </div>
            {:else if piece && (piece.stats.length || piece.slotsLeft != null)}
              <p class="piece-line tabular">
                {#each piece.stats as s, k}{#if k}<span class="sep">{" · "}</span>{/if}<span class="nowrap"
                    >{s.label} {s.value > 0 ? "+" : ""}{s.value}</span
                  >{/each}{#if piece.slotsLeft != null}<span class="sep">{piece.stats.length ? " · " : ""}</span><span
                    class="nowrap faint">{piece.slotsLeft}{piece.slots != null ? ` of ${piece.slots}` : ""} slots left</span
                  >{/if}
              </p>
            {/if}
{/snippet}

{#snippet putOn()}
    {#if goalError}<p class="goal-error">{goalError}</p>{/if}

    <label class="find">
      <Search size={13} />
      <input type="search" placeholder="Put on gear by name" aria-label="Find equipment you wear" bind:value={gearText} />
    </label>
    {#if gearFound.length}
      <ul class="found">
        {#each gearFound as item (item.id)}
          {@const on = worn.has(item.id)}
          <li>
            <button disabled={on} onclick={() => wear(item.id)}>
              <span class="found-name">{item.name}</span>
              <small>{slotName(item.slot)}{item.reqLevel ? ` · Lv ${item.reqLevel}` : ""}</small>
              {#if on}<Check size={12} />{:else}<Plus size={12} />{/if}
            </button>
          </li>
        {/each}
      </ul>
    {:else if gearText.trim().length >= 2 && searched === gearText}
      <p class="quiet-line">No equipment by that name for this job.</p>
    {/if}
{/snippet}

{#snippet skillsView()}
  {#if !skillsRead}
    <p class="quiet-line">Not read yet. Open your Skill window in the game and Leafline reads it, or press Read stats.</p>
  {/if}
  {#each skillGroups as group (group.title)}
    <h4 class="sub skill-group">{group.title}<span class="count tabular">{skillsRead ? group.spent : "—"} / {group.total} points spent</span></h4>
    <ul class="skills">
      {#each group.rows as skill (skill.id)}
        {@const max = skill.max ?? Math.max(skill.goal, skill.level, 1)}
        {@const done = isMaxed(skill)}
        <li class:done use:tip={skill.description ?? undefined}>
          <span class="skill-name"><Ref kind="skill" id={skill.id} name={skill.name} /></span>
          <span class="skill-level tabular">
            {#if done}<span class="max">MAX</span>{:else}<b>{skillsRead ? skill.level : "—"}</b><span class="faint"> / {max}</span>{/if}
          </span>
          <span class="bar" aria-hidden="true">
            <span class="planned" style="width: {(Math.min(skill.goal, max) / max) * 100}%"></span>
            <span class="have" style="width: {(Math.min(skill.level, max) / max) * 100}%"></span>
          </span>
          {#if skill.nextLevel != null && skill.nextGain != null}
            <span class="skill-next tabular">
              <b>Lv {skill.nextLevel}</b> +{skill.nextGain}{#if skill.goal > skill.level}<span class="faint"> · plan to {skill.goal}</span>{/if}
            </span>
          {:else if skill.goal > skill.level}
            <span class="skill-next tabular faint">plan to {skill.goal}</span>
          {/if}
        </li>
      {/each}
    </ul>
  {/each}
  {#if !skillRows.length}
    <p class="quiet-line">No skills read yet.</p>
  {/if}
{/snippet}

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
            <span class="tag">{gain.to - gain.from} SP</span><Ref kind="skill" id={gain.skillId} name={gain.name} />
            <span class="tabular dim">{gain.from} → {gain.to}</span>
          </p>
        {/each}
        {#each step.ap as gain (gain.stat)}
          <p><span class="tag">{gain.amount} AP</span>{statLabel(gain.stat)} <span class="tabular dim">to {gain.total}</span></p>
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
        <span class="plan" use:clipped>{plan.build.name}</span>
      </header>
      {@render todo(step)}
      {#if advance}
        <p class="next">
          Next job advancement: <strong>{advance.advance}</strong> at Lv {advance.level}
        </p>
      {/if}
      {#if warnings.length}
        <button class="link warn-line" onclick={() => openPlanner()}>
          {warnings.length === 1 ? "1 thing to fix" : `${warnings.length} things to fix`} in this plan
        </button>
      {/if}
    </section>
  {/if}
{:else}
<section class="build full" aria-label="Build plan">
  <header class="head strip">
    <span class="title strip-title">Build</span>
    {#if plan && step}<span class="plan" use:clipped>{plan.build.name}</span>{/if}
    {#if plan && step}
      <span class="tools">
{#if can.liveReading}<button
          class="link"
          use:tip={`Read your Character Stat and Skill windows${keyHint('ReadCharacter')}`}
          onclick={() => backend.readCharacter()}><RefreshCw size={12} />Read stats</button
        >{/if}
        <button class="link" onclick={() => openPlanner()}><Pencil size={12} />Edit</button>
      </span>
    {/if}
  </header>

  {#if !ready}
    <p class="lead">Plan your AP and SP level by level. Builds use the skills and equipment in the map data.</p>
    <MapDataPanel compact />
    <h4 class="sub">Stats</h4>
    <CharacterStats {character} />
  {:else if error}
    <p class="lead">{error}</p>
  {:else if loaded && !plan}
    <p class="lead">
      No build yet. Choose a job and write your goals in order, from a guide or your own idea, and Leafline lays them out
      level by level.
    </p>
    <button class="action" onclick={() => openPlanner()}><Pencil size={13} />Plan a build</button>
    <h4 class="sub">Stats</h4>
    <CharacterStats {character} />
  {:else if plan && step}
    {#snippet planPart()}
    {@render todo(step)}

    <h4 class="sub">Your stats by the plan</h4>
    {#if !statsKnown}
      <p class="quiet-line">
        Not read yet, so the plan counts from zero. Open your Character Stat and Skill windows in the game and Leafline reads
        them, or press Read stats.
      </p>
    {/if}
    <dl class="stats">
      {#each STATS as stat (stat.id)}
        <div>
          <dt>{stat.label}</dt>
          <dd class="tabular">
            {#if statsKnown && gearBonus?.[stat.id]}
              <!-- The total the game shows, then what you put in and what your gear adds. -->
              {step.stats[stat.id] + gearBonus[stat.id]}
              <span class="gear" use:tip={"Points you put in, plus what your gear adds as the Character Stat window last showed it"}
                >({step.stats[stat.id]}+{gearBonus[stat.id]})</span
              >
            {:else}
              {statsKnown ? step.stats[stat.id] : "—"}
            {/if}
          </dd>
        </div>
      {/each}
    </dl>

    {#if coming.length}
      <h4 class="sub">Coming up</h4>
        <ul class="coming">
          {#each coming as s (s.level)}
            <li>
              <span class="when tabular">Lv {s.level}</span>
              <span class="what">
                {#if s.advance}<span class="advance">{s.advance}</span>{/if}
                <!-- What the level adds, then the total it makes: "Double Stab +3 (4)". -->
                {#each s.sp as gain (gain.skillId)}<span><Ref kind="skill" id={gain.skillId} name={gain.name} /> <span class="tabular">+{gain.to - gain.from}</span> <span class="total tabular">({gain.to})</span></span>{/each}
                {#each s.ap as gain (gain.stat)}<span>{statLabel(gain.stat)} <span class="tabular">+{gain.amount}</span> <span class="total tabular">({gain.total})</span></span>{/each}
                {#each s.gear as name (name)}<Ref kind="item" name={name} />{/each}
              </span>
            </li>
          {/each}
        </ul>
    {/if}

    {#if nextGear || advance}
      <h4 class="sub">Next</h4>
      <ul class="coming">
        {#if nextGear}
          <li>
            <span class="when tabular">Lv {nextGear.level}</span>
            <span class="what"
              ><span>New gear:</span>{#each nextGear.gear as name (name)}<Ref kind="item" {name} />{/each}</span
            >
          </li>
        {/if}
        {#if advance}
          <li>
            <span class="when tabular">Lv {advance.level}</span>
            <span class="what"><span class="advance">{advance.advance}</span></span>
          </li>
        {/if}
      </ul>
    {/if}

    {#if warnings.length}
      <h4 class="warn-title">{warnings.length === 1 ? "1 thing to fix" : `${warnings.length} things to fix`}</h4>
      <ul class="warnings">
        {#each warnings.slice(0, 3) as problem (problem.message)}<li>{problem.message}</li>{/each}
      </ul>
      {#if warnings.length > 3}<button class="link" onclick={() => openPlanner()}>See all in the editor</button>{/if}
    {/if}
    {/snippet}
    {#snippet advancePart()}
      {#if plan && nextAdvance?.advance && plan.build.plan.job}
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
      {/if}
    {/snippet}
    {#snippet statsPart()}
    <CharacterStats character={character ?? plan?.build.character ?? null} showRead={false} />
    {/snippet}
    {#snippet gearPart()}
    <!--
      Gear slot by slot, as the equip window has it: what you wear (its own stats, scrolls and all,
      which can be put right here), the plan's next goal for the slot, and the best you can wear
      now when it beats what you wear.
    -->
    {#if outfit}
      {#if !plan?.build.start.pieces?.length}
        <p class="quiet-line">
          Check what you wear in game{keyHint("ReadItem")} and press <strong>I'm wearing this</strong>, or put it on by name below.
        </p>
      {/if}
      <ul class="outfit">
        {#each outfit.slots as row (row.slot)}
          <li>
            <h5 class="slot-name">{slotName(row.slot)}</h5>
            {@render wearing(row)}

            {#if row.suggested}
              {@const pick = row.suggested}
              <div class="line">
                <span class="label">Suggested</span>
                <Ref kind="item" id={pick.id} name={pick.name} />
                <span class="end">
                  <span class="lv tabular" class:good={row.suggestedFit?.ok} class:bad={row.suggestedFit && !row.suggestedFit.ok} use:tip={fitTitle(row.suggestedFit)}
                    >{pick.reqLevel ? `Lv ${pick.reqLevel}` : "Any level"}</span
                  >
                  {#if row.suggestedPlanned}
                    <button
                      class="remove"
                      use:tip={"Remove goal"}
                      aria-label="Stop aiming for {pick.name}"
                      onclick={() => setGoal(pick.id, false)}><X size={12} /></button
                    >
                  {:else}
                    <button class="remove" use:tip={"Aim for this"} aria-label="Aim for {pick.name}" onclick={() => setGoal(pick.id, true)}
                      ><Plus size={12} /></button
                    >
                  {/if}
                </span>
              </div>
              <p class="piece-line">{gives(pick, 3)}</p>
            {/if}

            {#if row.next}
            {@const next = row.next}
            <div class="line">
              <span class="label">Next</span>
                <Ref kind="item" id={next.item.id} name={next.item.name} />
                <span class="end">
                  <span
                    class="lv tabular"
                    class:warn={next.level == null}
                    class:good={next.level != null && row.nextFit?.ok}
                    class:bad={next.level != null && row.nextFit && !row.nextFit.ok}
                    use:tip={fitTitle(row.nextFit)}
                  >
                    {#if next.level == null}Out of reach{:else}Lv {next.level}{/if}
                  </span>
                  {#if row.planned}
                    <button
                      class="remove"
                      use:tip={"Remove goal"}
                      aria-label="Stop aiming for {next.item.name}"
                      onclick={() => setGoal(next.item.id, false)}><X size={12} /></button
                    >
                  {:else}
                    <button class="remove" use:tip={"Aim for this"} aria-label="Aim for {next.item.name}" onclick={() => setGoal(next.item.id, true)}
                      ><Plus size={12} /></button
                    >
                  {/if}
                </span>
            </div>
            {/if}
            {#if row.next}
              <p class="piece-line">
                {gives(row.next.item as ItemRecord, 3)}{#if row.later}<span class="sep">{" · "}</span><button class="link later" onclick={() => openPlanner()}
                    >{row.later} more {row.later === 1 ? "step" : "steps"} after it</button
                  >{/if}
              </p>
            {/if}
          </li>
        {/each}
      </ul>
      {#if gearTotals.length}
        <p class="quiet-line totals tabular">
          <strong>Your gear adds</strong>
          {#each gearTotals as total, k}{#if k}<span class="sep">{" · "}</span>{/if}<span class="nowrap">{total.label} {total.value > 0 ? "+" : ""}{total.value}</span>{/each}
        </p>
      {/if}
    {/if}
    {@render putOn()}
    {/snippet}
    {#if whole}
      <!-- Room for every part at once, as in the desk view: side by side where it fits. -->
      <div class="parts">
        <div class="part"><h4 class="part-title">Level up</h4>{@render planPart()}</div>
        <div class="part"><h4 class="part-title">Gear</h4>{@render gearPart()}</div>
        <div class="part"><h4 class="part-title">Stats</h4>{@render statsPart()}</div>
        <div class="part"><h4 class="part-title">Skills</h4>{@render skillsView()}</div>
        {#if canAdvance && nextAdvance?.advance && plan.build.plan.job}<div class="part"><h4 class="part-title">Job advance</h4>{@render advancePart()}</div>{/if}
      </div>
    {:else}
    <div class="views"><Segmented label="Build view" options={views} value={shown} stretch size="sm" onchange={pickView} /></div>
    {#if shown === "plan"}
      {@render planPart()}
    {:else if shown === "advance" && nextAdvance?.advance && plan.build.plan.job}
      {@render advancePart()}
    {:else if shown === "skills"}
    {@render skillsView()}
    {:else if shown === "stats"}
      {@render statsPart()}
    {:else}
      {@render gearPart()}
    {/if}
    {/if}

    <footer>
      {#if plan.build.guide}
        <button class="credit" onclick={() => backend.openGuide(plan!.build.guide)}><ExternalLink size={11} />Open guide</button>
      {/if}
    </footer>
  {/if}
</section>
{/if}

<style>
  .parts {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
    gap: var(--space-4) var(--space-6);
    align-items: start;
  }

  .part {
    display: grid;
    gap: var(--space-1-5);
    min-width: 0;
    align-content: start;
  }

  /*
   * Gear with room to spare: each slot's name beside its rows, the stat details quieter and on one
   * line, and the edit and remove buttons only on the row you point at.
   */
  .parts .outfit li {
    display: grid;
    grid-template-columns: 72px minmax(0, 1fr);
    column-gap: var(--space-2);
    row-gap: 1px;
    padding: var(--space-2) 0;
  }

  .parts .outfit li > :global(*) {
    grid-column: 2;
  }

  .parts .outfit .slot-name {
    grid-column: 1;
    grid-row: 1 / span 12;
    margin: 1px 0 0;
    color: var(--faint);
    font-size: var(--text-xs);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .parts .outfit .line {
    grid-template-columns: 62px minmax(0, 1fr) auto;
  }

  .parts .outfit .line .label {
    color: var(--faint);
    font-size: var(--text-xs);
  }

  .parts .outfit .piece-line {
    margin: 0 0 var(--space-0-5) 68px;
    overflow: hidden;
    color: var(--faint);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .parts .outfit .remove {
    opacity: 0;
    transition: opacity 0.12s ease;
  }

  .parts .outfit li:hover .remove,
  .parts .outfit li:focus-within .remove {
    opacity: 1;
  }

  .part-title {
    margin: 0 0 var(--space-0-5);
    padding-bottom: var(--space-1-5);
    border-bottom: 1px solid var(--border);
    font-family: var(--font-display);
    font-size: var(--text-md);
    font-weight: var(--weight-strong);
  }

  .link.warn-line {
    margin-top: var(--space-2);
    color: var(--warn);
  }

  .views {
    display: flex;
    margin: var(--space-2) 0 var(--space-2);
  }

  .sub {
    margin: var(--space-3) 0 var(--space-1-5);
    font-family: var(--font-display);
    font-size: var(--text-md);
    font-weight: var(--weight-body);
    color: var(--muted);
  }

  .build {
    margin-top: var(--space-3);
    padding-top: var(--space-2);
    border-top: 1px solid var(--border);
  }

  /* In the tab, the build opens with a band like a section of its own, out to the card's edges. */
  .build.full {
    margin-top: var(--space-4);
    padding-top: 0;
    border-top: 0;
  }

  .head.strip {
    align-items: center;
    margin: 0 0 var(--space-2);
  }

  .head.strip .title {
    color: var(--text);
    font-weight: var(--weight-strong);
  }

  .head {
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
    margin-bottom: var(--space-2);
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
    font-weight: var(--weight-strong);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tools {
    display: inline-flex;
    flex: none;
    gap: var(--space-2);
    margin-left: auto;
  }

  .lead {
    margin: 0 0 var(--space-2);
    color: var(--muted);
    font-size: var(--text-md);
  }

  .action {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1-5);
    height: var(--control-h);
    padding: 0 var(--space-3);
    border: 1px solid var(--accent);
    border-radius: var(--radius-sm);
    background: var(--accent);
    color: var(--accent-ink);
    font-size: var(--text-md);
    font-weight: var(--weight-strong);
    cursor: pointer;
  }

  .action:hover {
    filter: brightness(1.2);
  }

  .link {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--muted);
    font-size: var(--text-md);
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
    gap: var(--space-2);
    align-items: baseline;
  }

  .at {
    flex: none;
    width: 42px;
    color: var(--muted);
    font-family: var(--font-display);
    font-size: var(--text-md);
    font-weight: var(--weight-strong);
  }

  .todo {
    flex: 1;
    min-width: 0;
  }

  .todo p {
    display: flex;
    align-items: baseline;
    gap: var(--space-1-5);
    margin: 0 0 var(--space-0-5);
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
    font-weight: var(--weight-strong);
  }

  .dim {
    color: var(--muted);
    white-space: nowrap;
  }

  .advance {
    color: var(--text);
    font-weight: var(--weight-strong);
  }

  .stats {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: var(--space-1-5);
    margin: var(--space-2) 0 0;
  }

  .stats div {
    padding: var(--space-1) var(--space-2);
    border-radius: var(--radius-sm);
    background: var(--hover);
  }

  dt {
    color: var(--faint);
    font-size: var(--text-xs);
  }

  dd {
    margin: 0;
    font-weight: var(--weight-strong);
  }

  /* What gear adds, beside the stat: shown, never part of the plan's count. */
  dd .gear {
    margin-left: var(--space-1);
    color: var(--muted);
    font-size: var(--text-xs);
    font-weight: var(--weight-body);
  }

  .next {
    margin: var(--space-2) 0 0;
    color: var(--muted);
    font-size: var(--text-md);
  }

  .next strong {
    color: var(--text);
    font-weight: var(--weight-strong);
  }

  h4 {
    margin: var(--space-3) 0 var(--space-1);
    font-family: var(--font-display);
    font-size: var(--text-md);
    font-weight: var(--weight-body);
  }

  .warn-title {
    color: var(--warn);
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
    font-size: var(--text-md);
  }

  .coming li {
    display: flex;
    gap: var(--space-2);
    padding: var(--space-0-5) 0;
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
    gap: var(--space-0-5) var(--space-3);
    min-width: 0;
  }

  .what .total {
    color: var(--muted);
  }

  .remove {
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

  .remove:hover,
  .remove:focus-visible {
    background: var(--hover);
    color: var(--text);
    text-decoration: none;
    outline: none;
  }

  .outfit li {
    padding: var(--space-1-5) 0 var(--space-2);
  }

  .outfit li + li {
    border-top: 1px solid var(--border);
  }

  .slot-name {
    margin: 0 0 var(--space-0-5);
    font-family: var(--font-display);
    font-size: var(--text-md);
    font-weight: var(--weight-body);
    color: var(--muted);
  }

  /* Wearing, Suggested and Next: a label, the item, then its level and buttons. */
  .line {
    display: grid;
    grid-template-columns: 64px minmax(0, 1fr) auto;
    align-items: baseline;
    gap: var(--space-1-5);
    padding: 1px 0;
  }

  .line .label {
    color: var(--faint);
    font-size: var(--text-xs);
  }

  .line :global(.ref) {
    min-width: 0;
    padding: 0;
    border: 0;
    background: transparent;
    font-size: var(--text-md);
    font-weight: var(--weight-strong);
    text-align: left;
    cursor: pointer;
  }

  /* Scrolled, in the game's amber, as the item check shows it. */
  .line :global(.ref.upgraded:not(.worn)) {
    color: var(--accent-text-strong);
  }

  .line .none {
    color: var(--faint);
    font-size: var(--text-md);
  }

  .line .end {
    display: flex;
    align-items: center;
    gap: var(--space-1-5);
  }

  .line .lv {
    color: var(--muted);
    font-size: var(--text-md);
  }

  .line .lv.good {
    color: var(--good);
  }

  .line .lv.warn {
    color: var(--warn);
  }

  .line .lv.bad {
    color: var(--bad);
  }

  /* Lines under an item line up with its name. */
  .outfit .piece-line,
  .outfit .edit {
    margin-left: 70px;
  }

  .link.later {
    font-size: var(--text-xs);
  }

  .edit {
    display: grid;
    gap: var(--space-1-5);
    margin-top: var(--space-1-5);
    padding: var(--space-2);
    background: var(--surface-2);
  }

  .edit-field {
    display: grid;
    gap: var(--space-0-5);
    font-size: var(--text-xs);
    color: var(--muted);
  }

  .edit input,
  .edit select {
    min-width: 0;
    height: 26px;
    padding: 0 var(--space-1-5);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--text);
    font-size: var(--text-md);
  }

  .stat-edit {
    display: grid;
    grid-template-columns: 1fr 72px 20px;
    align-items: center;
    gap: var(--space-1-5);
  }

  .add-stat {
    justify-self: start;
  }

  .edit-pair {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-1-5);
  }

  .edit-actions {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .action.small {
    height: 24px;
    padding: 0 var(--space-3);
  }

  .piece-line {
    margin: 1px 0 0;
    color: var(--muted);
    font-size: var(--text-xs);
  }

  .piece-line .sep,
  .piece-line .faint {
    color: var(--faint);
  }

  .nowrap {
    white-space: nowrap;
  }

  .quiet-line {
    margin: 0 0 var(--space-1);
    color: var(--faint);
    font-size: var(--text-md);
  }

  .quiet-line strong {
    color: var(--muted);
    font-weight: var(--weight-strong);
  }

  .goal-error {
    margin: var(--space-1) 0 0;
    color: var(--bad);
    font-size: var(--text-md);
  }

  .find {
    display: flex;
    align-items: center;
    gap: var(--space-1-5);
    height: 28px;
    margin-top: var(--space-2);
    padding: 0 var(--space-2);
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
    font-size: var(--text-md);
  }

  .found {
    margin-top: var(--space-1);
    border: 1px solid var(--border);
  }

  .found li + li {
    border-top: 1px solid var(--border);
  }

  .found button {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
    padding: var(--space-1) var(--space-2);
    border: 0;
    background: transparent;
    color: var(--text);
    font-size: var(--text-md);
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
    color: var(--accent-text);
  }

  .skill-group {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    margin: var(--space-3) 0 var(--space-1);
    padding-bottom: var(--space-0-5);
    border-bottom: 1px solid var(--border-strong);
    color: var(--text);
    font-size: var(--text-xs);
    font-weight: var(--weight-strong);
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  .skill-group .count {
    color: var(--faint);
    font-weight: var(--weight-body);
  }

  .skills li {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: var(--space-0-5) var(--space-2);
    padding: var(--space-1-5) 0;
  }

  .skills li + li {
    border-top: 1px solid var(--border);
  }

  .skill-name {
    display: flex;
    align-items: center;
    gap: var(--space-1-5);
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--text-md);
    font-weight: var(--weight-strong);
  }

  .skill-level {
    font-size: var(--text-md);
  }

  .skill-level b {
    font-weight: var(--weight-strong);
  }

  /* Level reached over the level the plan ends at, to the skill's maximum. */
  .bar {
    position: relative;
    grid-column: 1 / -1;
    height: 5px;
    background: var(--hover);
  }

  .bar span {
    position: absolute;
    inset: 0 auto 0 0;
  }

  .bar .planned {
    background: color-mix(in srgb, var(--accent) 35%, transparent);
  }

  .bar .have {
    background: var(--accent-strong);
  }

  .skill-next {
    grid-column: 1 / -1;
    color: var(--muted);
    font-size: var(--text-md);
  }

  .skill-next b {
    color: var(--accent-text-strong);
    font-weight: var(--weight-strong);
  }

  .skills .faint,
  .skill-next .faint {
    color: var(--faint);
  }

  .skills li.done .skill-name {
    color: var(--muted);
    font-weight: var(--weight-body);
  }

  .skills li.done .bar .have {
    background: var(--good);
  }

  .skills li.done {
    padding: var(--space-1) 0;
  }

  .max {
    color: var(--good);
    font-size: var(--text-md);
    font-weight: var(--weight-strong);
    letter-spacing: 0.06em;
  }

  .warnings li {
    padding: var(--space-0-5) 0 var(--space-0-5) var(--space-2);
    border-left: 2px solid var(--warn);
    color: var(--muted);
  }

  .warnings li + li {
    margin-top: var(--space-1);
  }

  footer {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1) var(--space-3);
    margin-top: var(--space-3);
  }

  .credit {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
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

  /* The picture of a skill, in line with its name. */
  :global(.skill-pic) {
    margin-right: var(--space-1);
  }
</style>
