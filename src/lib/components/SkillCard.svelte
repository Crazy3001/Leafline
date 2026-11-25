<script lang="ts">
  // One skill: its picture, job and tier, what it does, the skills it needs first, and what each
  // level gives. Skills it needs are coloured names that open their own cards.
  import Ref from "$lib/components/Ref.svelte";
  import Section from "$lib/components/map/Section.svelte";
  import { skillById } from "$lib/skills.svelte";
  import type { SkillInfo } from "$lib/types";
  import Pic from "$lib/ui/Pic.svelte";

  let { skill }: { skill: SkillInfo } = $props();

  /** The description without the game's "[Master Level: N]" line, which the facts already say. */
  const about = $derived(
    (skill.description ?? "")
      .split("\n")
      .filter((line) => line.trim() && !/^\[Master Level/i.test(line.trim()))
      .join("\n"),
  );
</script>

<div class="head">
  <span class="pic"><Pic kind="skill" id={skill.id} name={skill.name} size={36} /></span>
  <div>
    <h3>{skill.name}</h3>
    <p class="facts">
      {skill.className}{skill.jobTier && skill.jobTier !== skill.className ? ` · ${skill.jobTier}` : ""} · max Lv {skill.maxLevel}{skill.passive
        ? " · passive"
        : ""}
    </p>
  </div>
</div>

{#if about}<p class="about">{about}</p>{/if}

{#if skill.requiredSkills.length}
  <p class="needs">
    Needs
    {#each skill.requiredSkills as need, i (need.skillId)}
      {i ? ", " : ""}<Ref kind="skill" id={need.skillId} name={skillById(need.skillId)?.name ?? `Skill ${need.skillId}`} /> Lv {need.level}
    {/each}
  </p>
{/if}

{#if skill.levelDescriptions.length}
  <Section title="Each level" count={skill.levelDescriptions.length} memory="skill-levels" open={false}>
    <ol class="levels">
      {#each skill.levelDescriptions as text, i (i)}
        <li><span class="lv tabular">Lv {i + 1}</span><span>{text}</span></li>
      {/each}
    </ol>
  </Section>
{/if}

<style>
  .head {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-bottom: var(--space-2);
  }

  .pic {
    display: grid;
    flex: none;
    place-items: center;
    width: 46px;
    height: 46px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
  }

  h3 {
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--text-lg);
    font-weight: var(--weight-strong);
  }

  .facts {
    margin: var(--space-0-5) 0 0;
    color: var(--muted);
    font-size: var(--text-md);
  }

  .about {
    margin: 0 0 var(--space-2);
    color: var(--text);
    font-size: var(--text-md);
    line-height: 1.5;
    white-space: pre-line;
  }

  .needs {
    margin: 0 0 var(--space-2);
    color: var(--muted);
    font-size: var(--text-md);
  }

  .levels {
    display: grid;
    gap: var(--space-0-5);
    margin: 0;
    padding: 0;
    list-style: none;
    font-size: var(--text-md);
  }

  .levels li {
    display: grid;
    grid-template-columns: 40px minmax(0, 1fr);
    gap: var(--space-1-5);
  }

  .lv {
    color: var(--faint);
  }
</style>
