<script lang="ts">
  // Lookup → Skills: every skill in the map data, by job, searchable by name or what it does.
  // A row opens the skill here; skill names elsewhere open its card.
  import Chips from "./Chips.svelte";
  import LookupRow from "./LookupRow.svelte";
  import LookupSplit from "./LookupSplit.svelte";
  import SearchBar from "./SearchBar.svelte";
  import SkillCard from "../SkillCard.svelte";
  import { loadSkills, skillById, skillData, skillView } from "$lib/skills.svelte";

  let { pinned = false }: { pinned?: boolean } = $props();

  let text = $state("");
  let job = $state<string | null>(null);

  $effect(() => {
    loadSkills();
  });

  /** Jobs in the data's own order of skill ids, Beginner first. */
  const jobs = $derived([...new Set([...skillData.skills].sort((a, b) => a.jobId - b.jobId).map((s) => s.className))]);
  const shown = $derived.by(() => {
    const words = text.trim().toLowerCase().split(/\s+/).filter(Boolean);
    return skillData.skills
      .filter((s) => !job || s.className === job)
      .filter((s) => words.every((w) => s.name.toLowerCase().includes(w) || (s.description ?? "").toLowerCase().includes(w)))
      .sort((a, b) => a.jobId - b.jobId || a.id - b.id);
  });
  const open = $derived(skillById(skillView.id));
</script>

<div class="controls" class:pinned>
  <SearchBar
    value={text}
    placeholder="Skill, or what it does"
    label="Search skills"
    filterable={false}
    clearable={text !== "" || job != null || open != null}
    oninput={(t) => {
      text = t;
      skillView.id = null;
    }}
    onclear={() => {
      text = "";
      job = null;
      skillView.id = null;
    }}
  />
  <Chips label="Job" options={jobs} value={job} onselect={(j) => (job = j)} />
</div>

<LookupSplit open={open != null} onback={() => (skillView.id = null)} {pinned}>
  {#snippet detail()}
    {#if open}<div class="list" class:pinned><SkillCard skill={open} /></div>{/if}
  {/snippet}
    {#snippet empty()}<p class="pick">Pick a skill to see it here.</p>{/snippet}
  {#snippet list()}
<div class="list" class:pinned>
  {#if !skillData.loaded}
    <p class="quiet">Loading skills…</p>
  {:else if !skillData.skills.length}
    <p class="quiet">Skills come with the map data. Download it in Settings → Data.</p>
  {:else if !shown.length}
    <p class="quiet">No skills match.</p>
  {:else}
    <p class="count tabular">{shown.length} skill{shown.length === 1 ? "" : "s"}</p>
    <ul class="rows">
      {#each shown as skill (skill.id)}
        <li>
          <LookupRow
            name={skill.name}
            picture={{ kind: "skill", id: skill.id }}
            line={`${skill.className}${skill.jobTier && skill.jobTier !== skill.className ? ` · ${skill.jobTier}` : ""}${skill.passive ? " · passive" : ""}`}
            aside={`max ${skill.maxLevel}`}
            selected={skillView.id === skill.id}
            onclick={() => (skillView.id = skill.id)}
          />
        </li>
      {/each}
    </ul>
  {/if}
</div>
  {/snippet}
</LookupSplit>

<style>
  .controls {
    padding: var(--space-2) var(--space-3) 0;
  }

  .list {
    padding: 0 var(--space-3) var(--space-2);
  }

  .controls.pinned,
  .list.pinned {
    padding: 0;
  }

  .rows {
    margin: 0;
    padding: 0;
    list-style: none;
  }


  .count,
  .quiet {
    margin: 0 0 var(--space-1);
    color: var(--faint);
    font-size: var(--text-md);
  }

  .pick {
    margin: 0;
    padding: var(--space-8) var(--space-3);
    color: var(--faint);
    font-size: var(--text-md);
    text-align: center;
  }
</style>
