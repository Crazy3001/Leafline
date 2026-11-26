<script lang="ts">
  // A new build in three choices: the second job, a ready-made plan or a blank one, and a name.
  // Everything can be changed later in the planner's tabs.
  import { backend } from "$lib/api";
  import StarterCard from "$lib/components/planner/StarterCard.svelte";
  import Segmented from "$lib/ui/Segmented.svelte";
  import { tip } from "$lib/tip.svelte";
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
  /** Who the gear is made for: "" for either, until the character's worn gear says. */
  let gender = $state("");
  const GENDERS = [
    { id: "", label: "Either" },
    { id: "male", label: "Male" },
    { id: "female", label: "Female" },
  ];
  let error = $state<string | null>(null);
  let busy = $state(false);

  const jobName = $derived(catalog.jobs.find((j) => j.id === job)?.name ?? "");
  /** A new character starts from the plan's roll; one already levelled enters their own. */
  const fresh = $derived((level ?? 1) <= 1);

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
        guide: starter?.source?.url ?? "",
        start: {
          level: level ?? 1,
          stats: starter && fresh ? { ...starter.start } : null,
          ap: 0,
          sp: 0,
          skills: [],
          gender: gender || null,
        },
        plan,
      });
      // Slots the plan leaves empty get an upgrade path from the data, as Fill empty slots does.
      const added = await backend.fillGear(made.build.id, level ?? 1).catch(() => 0);
      onCreated((added && (await backend.buildPlan(made.build.id))) || made);
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
    {#if plans.length}
      <p class="help">
        Plans run to Lv 70 and nobody has played them yet.{#if fresh}
          Each counts from the Lv 1 roll shown; if yours differs, change it in the plan.{/if}
      </p>
    {/if}
    <div class="starts">
      {#each plans as plan (plan.id)}
        <StarterCard {plan} {jobName} selected={from === plan.id} onpick={() => (from = plan.id)} />
      {/each}
      <button class="choice blank" class:on={from === "blank"} aria-pressed={from === "blank"} onclick={() => (from = "blank")}>
        <span class="start-name">Blank plan</span>
        <span class="muted">No goals yet. Add your own skills, stats and gear.</span>
      </button>
    </div>
  </section>

  {#if error}<p class="error" role="alert">{error}</p>{/if}
  <!-- Always in view at the bottom: the build's name, to change right there, and Create. -->
  <div class="actions">
    <label class="bar-name">
      <span>Name</span>
      <input
        maxlength="80"
        aria-label="Build name"
        value={shownName}
        oninput={(e) => {
          named = true;
          name = e.currentTarget.value;
        }}
      />
    </label>
    <label class="bar-field" use:tip={character ? "Read from the game. The build is used whenever Leafline reads this name." : "The build is used whenever Leafline reads this name; empty for anyone."}>
      <span>For</span>
      <input class="who" maxlength="30" placeholder="Any character" value={forWho} oninput={(e) => (who = e.currentTarget.value)} />
    </label>
    <span class="bar-field" use:tip={"The level the plan starts from: the character's, when Leafline has read it"}>
      <span>Starts at</span><b class="tabular">Lv {level ?? 1}</b>
    </span>
    <span class="bar-field" use:tip={"Some hats, tops and shoes are made for one. Change it later in the Gear tab."}>
      <span>Gear for</span>
      <Segmented label="Gear made for" size="sm" options={GENDERS} value={gender} onchange={(g) => (gender = g)} />
    </span>
    <span class="push"></span>
    <button class="create" disabled={!job || busy} onclick={create}>Create build</button>
    {#if canCancel}<button class="cancel" onclick={onCancel}>Cancel</button>{/if}
    {#if !job}
      <span class="faint">Pick a second job first.</span>
    {/if}
  </div>
</main>

<style>
  .setup {
    grid-column: 2 / -1;
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
    min-width: 0;
    max-width: 960px;
    padding: var(--space-6) var(--space-10) var(--space-8);
    overflow-y: auto;
  }

  h1 {
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--text-2xl);
    font-weight: var(--weight-strong);
  }

  .lead {
    margin: var(--space-1) 0 0;
    color: var(--muted);
  }

  section {
    display: grid;
    gap: var(--space-2);
  }

  h2 {
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--text-md);
    font-weight: var(--weight-body);
    color: var(--muted);
  }

  .seen {
    color: var(--faint);
  }

  .jobs {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: var(--space-3);
  }

  .column {
    display: grid;
    align-content: start;
    gap: var(--space-1-5);
  }

  .class-name {
    color: var(--faint);
    font-size: var(--text-xs);
  }

  .class-name.seen-first {
    color: var(--accent-text-strong);
  }

  .choice {
    padding: var(--space-1-5) var(--space-3);
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
    color: var(--accent-text-strong);
  }

  .starts {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
    gap: var(--space-3);
  }

  .blank {
    display: grid;
    align-content: start;
    gap: var(--space-1);
    padding: var(--space-3) var(--space-4);
    border-style: dashed;
    border-radius: var(--radius);
  }

  .blank.on {
    border-style: solid;
  }

  .start-name {
    font-weight: var(--weight-strong);
  }

  .blank.on .muted {
    color: var(--text);
  }

  .muted {
    color: var(--muted);
  }

  .faint {
    color: var(--faint);
    font-weight: var(--weight-body);
  }


  label {
    display: grid;
    gap: var(--space-0-5);
    color: var(--muted);
    font-size: var(--text-md);
  }


  input {
    padding: var(--space-1) var(--space-2);
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


  .help {
    margin: 0;
    color: var(--faint);
    font-size: var(--text-md);
  }

  .error {
    margin: 0;
    padding: var(--space-1-5) var(--space-3);
    background: var(--bad-soft);
    color: var(--bad);
    font-size: var(--text-md);
  }

  .actions {
    flex-wrap: wrap;
    row-gap: var(--space-2);
    position: sticky;
    bottom: calc(-1 * var(--space-8));
    z-index: 2;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    margin: 0 calc(-1 * var(--space-10)) calc(-1 * var(--space-8));
    padding: var(--space-3) var(--space-10);
    border-top: 1px solid var(--border);
    background: var(--surface-raised);
  }

  .bar-name,
  .bar-field {
    display: flex;
    align-items: center;
    gap: var(--space-1-5);
    min-width: 0;
    color: var(--muted);
    font-size: var(--text-md);
  }

  .bar-name {
    flex: 1 1 240px;
    max-width: 380px;
  }

  .bar-field b {
    color: var(--text);
    font-weight: var(--weight-strong);
  }

  .who {
    width: 130px;
  }

  .push {
    flex: 1;
  }

  .bar-name input {
    flex: 1;
    min-width: 0;
  }



  .create {
    height: 32px;
    padding: 0 var(--space-5);
    border: 0;
    border-radius: var(--radius-sm);
    background: var(--accent);
    color: var(--accent-ink);
    font: inherit;
    font-weight: var(--weight-strong);
    cursor: pointer;
  }

  .create:disabled {
    opacity: 0.45;
    cursor: default;
  }

  .cancel {
    padding: 0 var(--space-2);
    border: 0;
    background: transparent;
    color: var(--muted);
    font: inherit;
    cursor: pointer;
  }
</style>
