<script lang="ts">
  // What to do for the build's next job advancement: who to see, where they stand and the quests
  // involved, found in the downloaded quest and NPC data by the job's name. Nothing here is a
  // game rule Leafline knows: it is what the data lists, and the game's own NPCs come first.
  // Ticks are a convenience kept in this window's storage, per character and advancement.
  import { Check } from "@lucide/svelte";
  import { untrack } from "svelte";

  import { backend } from "$lib/api";
  import { live } from "$lib/readings.svelte";
  import type { QuestRow } from "$lib/types";

  let {
    job,
    tier,
    at,
    level,
    character = null,
  }: {
    /** The build's job: a first or second job id. */
    job: number;
    /** 1 for the first job advancement, 2 for the second. */
    tier: 1 | 2;
    /** The level the plan advances at. */
    at: number;
    level: number | null;
    character?: string | null;
  } = $props();

  interface Place {
    npcId: number;
    name: string;
    map: string | null;
    hops: number | null;
  }

  interface Found {
    /** The first job's name, as the data spells it, which the quests are named after. */
    className: string;
    nextName: string | null;
    instructor: Place | null;
    quests: QuestRow[];
    /** The data's "<class> Job Instructor", whom the quests are likely to send the player to. */
    tester: Place | null;
  }

  /** Branch names by the job id's hundreds, as `Branch::name` in src-tauri/src/build/rules.rs.
   *  The advancement quests in the data are named after these, in case the skill data's own
   *  class name for the first job differs. */
  const BRANCHES: Record<number, string> = { 1: "Warrior", 2: "Magician", 3: "Bowman", 4: "Thief" };

  let found = $state<Found | null>(null);
  let loading = $state(true);
  let ticks = $state<Record<string, boolean>>({});

  const mapKey = $derived(JSON.stringify([live.analysis?.map?.name, live.analysis?.map?.street, live.analysis?.map?.twin]));
  const storeKey = $derived(`leafline.advance.${character ?? "any"}.${job}.${tier}`);

  // Follows its inputs alone: every live reading brings new objects with the same content.
  $effect(() => {
    const key = JSON.stringify([job, tier, mapKey]);
    loading = true;
    untrack(() => look(job, tier)).then((result) => {
      if (key === JSON.stringify([job, tier, mapKey])) {
        found = result;
        loading = false;
      }
    });
  });

  $effect(() => {
    const key = storeKey;
    try {
      ticks = JSON.parse(localStorage.getItem(key) ?? "{}");
    } catch {
      ticks = {};
    }
  });

  async function look(job: number, tier: 1 | 2): Promise<Found | null> {
    try {
      const catalog = await backend.buildCatalog();
      const chosen = catalog?.jobs.find((j) => j.id === job);
      if (!catalog || !chosen) return null;
      const first = chosen.tier === 1 ? chosen : catalog.jobs.find((j) => j.id === chosen.parent);
      if (!first) return null;
      const second = tier === 2 && chosen.tier === 2 ? chosen.name : null;
      const page = await backend.searchQuests({ text: "Next Journey", region: null, repeat: null, atLevel: null, offset: 0 });
      const names = [first.name, BRANCHES[Math.floor(job / 100)]].filter(Boolean);
      const named = (name: string) => page.quests.find((q) => q.name.toLowerCase() === `the ${name.toLowerCase()}'s next journey`);
      const className = names.find((name) => named(name)) ?? first.name;
      const journey = named(className) ?? null;
      const quests: QuestRow[] = [];
      if (journey) {
        quests.push(journey);
        const more = await backend.searchQuests({ text: "Finding the Instructor", region: null, repeat: null, atLevel: null, offset: 0 });
        quests.push(...more.quests.filter((q) => q.giver === journey.giver && q.id !== journey.id));
      }
      const instructor = journey?.giver ? await place(journey.giver) : null;
      let tester: Place | null = null;
      if (tier === 2) {
        const npcs = await backend.searchNpcs({ text: `${className} Job Instructor`, kind: null, region: null, area: null, offset: 0 });
        const row = npcs.closest ? null : [...npcs.npcs].sort((a, b) => a.id - b.id)[0];
        if (row) tester = await place(row.name, row.id);
      }
      return { className, nextName: second, instructor, quests, tester };
    } catch {
      return null;
    }
  }

  /** Where an NPC stands, nearest first, from where the minimap says the player is. */
  async function place(name: string, id?: number): Promise<Place | null> {
    const npcId = id ?? (await backend.findNpc(name));
    if (npcId == null) return null;
    const map = live.analysis?.map ?? null;
    const where = await backend.npcSheet(npcId, map?.name ?? null, map?.street ?? null, map?.twin ?? null);
    const nearest = where?.sheet.maps[0] ?? null;
    return { npcId, name, map: nearest?.map.name ?? null, hops: nearest?.travel?.hops ?? nearest?.hops ?? null };
  }

  function toggle(step: string) {
    ticks = { ...ticks, [step]: !ticks[step] };
    try {
      localStorage.setItem(storeKey, JSON.stringify(ticks));
    } catch {
      // A convenience only.
    }
  }

  const reached = $derived(level != null && level >= at);
  const away = (hops: number | null) => (hops == null ? "" : hops === 0 ? " · here" : ` · ${hops} portal${hops === 1 ? "" : "s"}`);
</script>

{#snippet tick(step: string, done: boolean, auto = false)}
  <button
    class="box"
    class:done
    role="checkbox"
    aria-checked={done}
    disabled={auto}
    title={auto ? "Ticks itself when the HUD shows the level" : done ? "Not done yet" : "Done"}
    onclick={() => toggle(step)}
  >
    {#if done}<Check size={10} />{/if}
  </button>
{/snippet}

{#if loading && !found}
  <p class="quiet">Looking in the quest data…</p>
{:else}
  <ol class="steps">
    <li>
      {@render tick("level", reached, true)}
      <span class="what">
        Reach Lv {at}
        {#if level != null && !reached}<span class="dim tabular">· {at - level} to go</span>{/if}
      </span>
    </li>

    {#if found?.instructor}
      <li>
        {@render tick("instructor", !!ticks.instructor)}
        <span class="what">
          {tier === 1 ? "See" : "Talk to"}
          <button class="link" title="Show them in the Lookup tab" onclick={() => backend.showPlace("npcs", found!.instructor!.npcId)}
            >{found.instructor.name}</button
          >
          {#if found.instructor.map}<span class="dim">in {found.instructor.map}{away(found.instructor.hops)}</span>{/if}
        </span>
      </li>
    {/if}

    {#if tier === 2}
      {#each found?.quests ?? [] as quest (quest.id)}
        <li>
          {@render tick(`quest-${quest.id}`, !!ticks[`quest-${quest.id}`])}
          <span class="what">
            <button class="link" title="Open its steps in the Quests tab" onclick={() => backend.showQuest(quest.id)}>{quest.name}</button>
            <span class="dim">quest{quest.levelMin ? `, Lv ${quest.levelMin}` : ""}</span>
          </span>
        </li>
      {/each}
      {#if found?.tester}
        <li>
          {@render tick("tester", !!ticks.tester)}
          <span class="what">
            <button class="link" title="Show them in the Lookup tab" onclick={() => backend.showPlace("npcs", found!.tester!.npcId)}
              >{found.tester.name}</button
            >
            {#if found.tester.map}<span class="dim">in {found.tester.map}{away(found.tester.hops)}</span>{/if}
          </span>
        </li>
      {/if}
      <li>
        {@render tick("advanced", !!ticks.advanced)}
        <span class="what">Advance{found?.nextName ? ` to ${found.nextName}` : ""}, then spend the SP the plan lists at Lv {at}</span>
      </li>
    {:else}
      <li>
        {@render tick("advanced", !!ticks.advanced)}
        <span class="what">Become a {found?.className ?? "new job"}, then spend the SP the plan lists at Lv {at}</span>
      </li>
    {/if}
  </ol>

  <p class="hedge">
    {#if !found}
      The map data doesn't name anyone for this advancement. The game's NPCs, or a guide, will say where to go.
    {:else if tier === 1 && found.instructor}
      The data has no quest for the 1st job. {found.instructor.name} gives the {found.className}'s later job quests, so is
      the likely one to see; what they say in game comes first.
    {:else if tier === 1}
      The data has no quest for the 1st job and names no {found.className} instructor. The game's NPCs, or a guide, will
      say where to go.
    {:else if !found.quests.length}
      The data has no "{found.className}'s Next Journey" quest. The game's NPCs, or a guide, will say what to do.
    {:else}
      From the quest and NPC data. Classic may run the advancement differently; the NPCs' own words in game come first,
      and each quest's steps are in the Quests tab.
    {/if}
  </p>
{/if}

<style>
  .steps {
    display: grid;
    gap: 4px;
    margin: 0;
    padding: 0;
    list-style: none;
    font-size: var(--text-sm);
  }

  .steps li {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }

  .box {
    display: grid;
    flex: none;
    place-items: center;
    width: 13px;
    height: 13px;
    padding: 0;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--bg);
    transform: translateY(2px);
    cursor: pointer;
  }

  .box.done {
    border-color: var(--accent);
    background: var(--accent);
  }

  .box:disabled {
    cursor: default;
  }

  .box:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  .what {
    min-width: 0;
  }

  .link {
    padding: 0;
    border: 0;
    background: transparent;
    font-size: var(--text-sm);
    font-weight: 600;
    text-align: left;
    cursor: pointer;
  }

  .link:hover,
  .link:focus-visible {
    color: var(--accent-strong);
    text-decoration: underline;
    outline: none;
  }

  .dim,
  .quiet {
    color: var(--faint);
  }

  .quiet {
    margin: 0;
    font-size: var(--text-sm);
  }

  .hedge {
    margin: 8px 0 0;
    color: var(--faint);
    font-size: var(--text-xs);
    line-height: 1.4;
  }
</style>
