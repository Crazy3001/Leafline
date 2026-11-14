<script lang="ts">
  import SkillCard from "../SkillCard.svelte";
  import type { PeekTarget } from "$lib/peek.svelte";
  import { loadSkills, skillById, skillData, skillNamed } from "$lib/skills.svelte";

  let { target }: { target: PeekTarget } = $props();

  $effect(() => {
    loadSkills();
  });

  const skill = $derived(skillById(target.id) ?? skillNamed(target.name));
</script>

{#if skill}
  <SkillCard {skill} />
{:else if skillData.loaded}
  <p class="note">No skill {target.name ? `called ${target.name}` : ""} in the map data.</p>
{:else}
  <p class="note">Loading…</p>
{/if}

<style>
  .note {
    margin: 0;
    color: var(--faint);
    font-size: var(--text-sm);
  }
</style>
