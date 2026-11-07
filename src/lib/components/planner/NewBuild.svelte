<script lang="ts">
  // A new build in three choices: the second job, a ready-made plan or a blank one, and a name.
  // Everything can be changed later in the planner's tabs.
  import { backend } from "$lib/api";
  import type { BuildCatalog, BuildPlan, StarterPlan } from "$lib/types";

  let {
    catalog,
    starters,
    character,
    level,
    hudJob,
    canCancel,
    onCreated,
    onCancel,
  }: {
    catalog: BuildCatalog;
    starters: StarterPlan[];
    character: string | null;
    level: number | null;
    /** The job the HUD shows, to start on it. */
    hudJob: string | null;
    canCancel: boolean;
    onCreated: (plan: BuildPlan) => void;
    onCancel: () => void;
  } = $props();

  const firstJobs = $derived(catalog.jobs.filter((j) => j.tier === 1));
  const secondOf = (first: number) => catalog.jobs.filter((j) => j.parent === first);

  const seen = $derived(hudJob ? catalog.jobs.find((j) => j.name.toLowerCase() === hudJob.toLowerCase()) : undefined);
  // The job read in game: a second job is picked for you, a first job only narrows the choice.
  let job = $state(0);
  let touched = $state(false);
  $effect(() => {
    if (!touched && seen?.tier === 2) job = seen.id;
  });

  const plans = $derived(starters.filter((s) => s.job === job));
  /** "blank" or a starter's id. */
  let from = $state<string>("blank");
  $effect(() => {
    // A job with a ready-made plan starts on it.
    from = plans[0]?.id ?? "blank";
  });
  const starter = $derived(plans.find((p) => p.id === from) ?? null);

  let name = $state("");
  let named = $state(false);
  const suggested = $derived(starter?.name ?? (character ? `${character}'s build` : "New build"));
  const shownName = $derived(named ? name : suggested);
  let who = $state<string | null>(null);
  const forWho = $derived(who ?? character ?? "");
  let error = $state<string | null>(null);
  let busy = $state(false);

  const count = (n: number, one: string, many = `${one}s`) => `${n} ${n === 1 ? one : many}`;

  function pickJob(id: number) {
    touched = true;
    job = id;
  }

  async function create() {
    busy = true;
    try {
      const primary = catalog.jobs.find((j) => j.id === job)?.primary ?? null;
      const plan = starter
        ? JSON.parse(JSON.stringify(starter.plan))
        : { job, skills: [], stats: primary ? [{ stat: primary, target: null, fromLevel: null }] : [], gear: [], planTo: 70 };
      const made = await backend.saveBuild({
        id: 0,
        name: shownName.trim() || suggested,
        character: forWho.trim() || null,
        guide: "",
        start: { level: level ?? 1, stats: null, ap: 0, sp: 0, skills: [] },
        plan,
      });
      onCreated(made);
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<main class="setup">
  <h1>New build</h1>
  <p class="lead">Three choices, then the plan opens. Everything can be changed later.</p>

  <section>
    <h2>
      1 · Second job
      {#if seen}<span class="seen">· Leafline read {seen.name}{character ? ` on ${character}` : ""}</span>{/if}
    </h2>
    <div class="jobs">
      {#each firstJobs as first (first.id)}
        <div class="column">
          <span class="class-name" class:seen-first={seen?.id === first.id}>{first.name}</span>
          {#each secondOf(first.id) as second (second.id)}
            <button class="choice" class:on={job === second.id} aria-pressed={job === second.id} onclick={() => pickJob(second.id)}>
              {second.name}
            </button>
          {/each}
        </div>
      {/each}
    </div>
    <p class="help">Not sure yet? Pick the one you lean towards; the job can be changed in Settings.</p>
  </section>

  <section>
    <h2>2 · Start from</h2>
    <div class="starts">
      {#each plans as plan (plan.id)}
        <button class="choice start" class:on={from === plan.id} aria-pressed={from === plan.id} onclick={() => (from = plan.id)}>
          <span class="start-name">{plan.name} <span class="faint">· ready-made, to Lv {plan.plan.planTo}</span></span>
          <span class="muted">{plan.summary}</span>
          <span class="muted">
            {count(plan.plan.skills.length, "skill goal")}, {count(plan.plan.stats.length, "stat goal")},
            {count(plan.plan.gear.length, "piece", "pieces")} of gear.
          </span>
          {#if plan.missing.length}<span class="warn">Not in this data: {plan.missing.join(", ")}</span>{/if}
          <span class="warn">Untested in game: a starting point to change.</span>
        </button>
      {/each}
      <button class="choice start" class:on={from === "blank"} aria-pressed={from === "blank"} onclick={() => (from = "blank")}>
        <span class="start-name">Blank plan</span>
        <span class="muted">No goals yet. Add skills, stats and gear yourself, or from a guide.</span>
      </button>
    </div>
  </section>

  <section>
    <h2>3 · Name it</h2>
    <div class="names">
      <label class="grow">
        <span>Build name</span>
        <input
          maxlength="80"
          value={shownName}
          oninput={(e) => {
            named = true;
            name = e.currentTarget.value;
          }}
        />
      </label>
      <label>
        <span>For character</span>
        <input maxlength="30" placeholder="Any" value={forWho} oninput={(e) => (who = e.currentTarget.value)} />
      </label>
      <label>
        <span>Starts at</span>
        <input class="number" readonly value={`Lv ${level ?? 1}`} />
      </label>
    </div>
    <p class="help">
      {#if character}Character and level are read from the game.{:else}No character read yet: the build follows whoever you name.{/if}
      Stats and skills fill in the next time Leafline reads your Character Stat and Skill windows.
    </p>
  </section>

  {#if error}<p class="error" role="alert">{error}</p>{/if}
  <div class="actions">
    <button class="create" disabled={!job || busy} onclick={create}>Create build</button>
    {#if canCancel}<button class="cancel" onclick={onCancel}>Cancel</button>{/if}
    {#if !job}<span class="faint">Pick a second job first.</span>{/if}
  </div>
</main>

<style>
  .setup {
    grid-column: 2 / -1;
    display: flex;
    flex-direction: column;
    gap: 20px;
    min-width: 0;
    max-width: 860px;
    padding: 24px 36px 32px;
    overflow-y: auto;
  }

  h1 {
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--text-2xl);
    font-weight: 600;
  }

  .lead {
    margin: 4px 0 0;
    color: var(--muted);
  }

  section {
    display: grid;
    gap: 8px;
  }

  h2 {
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--text-md);
    font-weight: 400;
    color: var(--muted);
  }

  .seen {
    color: var(--faint);
  }

  .jobs {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 14px;
  }

  .column {
    display: grid;
    align-content: start;
    gap: 6px;
  }

  .class-name {
    color: var(--faint);
    font-size: var(--text-xs);
  }

  .class-name.seen-first {
    color: var(--accent-strong);
  }

  .choice {
    padding: 7px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .choice:hover {
    border-color: var(--border-strong);
  }

  .choice.on {
    border-color: var(--accent);
    background: var(--accent-soft);
    color: var(--accent-strong);
  }

  .starts {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 10px;
  }

  .start {
    display: grid;
    align-content: start;
    gap: 4px;
    padding: 12px 14px;
  }

  .start-name {
    font-weight: 600;
  }

  .start.on .muted {
    color: var(--text);
  }

  .muted {
    color: var(--muted);
  }

  .faint {
    color: var(--faint);
    font-weight: 400;
  }

  .warn {
    color: var(--warn);
    font-size: var(--text-sm);
  }

  .names {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    max-width: 640px;
  }

  label {
    display: grid;
    gap: 3px;
    color: var(--muted);
    font-size: var(--text-sm);
  }

  .grow {
    flex: 1;
    min-width: 200px;
  }

  input {
    padding: 4px 7px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    color: var(--text);
    font: inherit;
    font-size: var(--text-md);
  }

  input:focus-visible {
    border-color: var(--accent);
    outline: none;
  }

  .number {
    width: 70px;
  }

  .help {
    margin: 0;
    color: var(--faint);
    font-size: var(--text-sm);
  }

  .error {
    margin: 0;
    padding: 6px 10px;
    background: var(--bad-soft);
    color: var(--bad);
    font-size: var(--text-sm);
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .create {
    height: 32px;
    padding: 0 18px;
    border: 0;
    border-radius: var(--radius-sm);
    background: var(--accent);
    color: var(--bg);
    font: inherit;
    font-weight: 600;
    cursor: pointer;
  }

  .create:disabled {
    opacity: 0.45;
    cursor: default;
  }

  .cancel {
    padding: 0 8px;
    border: 0;
    background: transparent;
    color: var(--muted);
    font: inherit;
    cursor: pointer;
  }
</style>
