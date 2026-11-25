<script lang="ts">
  // One crafting recipe: what it makes, the skill level it needs, and what goes into it, each
  // ingredient as its picture and count. Every item opens its own card.
  import Ref from "$lib/components/Ref.svelte";
  import { formatNumber } from "$lib/format";
  import { tip } from "$lib/tip.svelte";
  import type { Recipe } from "$lib/types";
  import Pic from "$lib/ui/Pic.svelte";

  let {
    recipe,
    /** The item the page is about, left unlinked. */
    current = null,
    /** Say the discipline too, where the list mixes them. */
    discipline = true,
  }: { recipe: Recipe; current?: number | null; discipline?: boolean } = $props();
</script>

<div class="recipe">
  <div class="top">
    <span class="makes">
      {#if recipe.output.id === current}
        <Pic kind="item" id={recipe.output.id} size={20} /><strong>{recipe.output.name}</strong>
      {:else}
        <Ref kind="item" id={recipe.output.id} name={recipe.output.name} />
      {/if}
      {#if recipe.output.count > 1}<span class="times tabular">×{formatNumber(recipe.output.count)}</span>{/if}
    </span>
    <span class="how tabular" use:tip={recipe.craftExp ? `${formatNumber(recipe.craftExp)} crafting EXP` : null}
      >{discipline ? `${recipe.discipline} ` : ""}Lv {recipe.skillLevel}</span
    >
  </div>
  <div class="from">
    {#each recipe.ingredients as g (g.id)}
      <span class="ing">
        {#if g.id === current}
          <Pic kind="item" id={g.id} size={18} /><strong>{g.name}</strong>
        {:else}
          <Ref kind="item" id={g.id} name={g.name} />
        {/if}
        <span class="times tabular">×{formatNumber(g.count)}</span>
      </span>
    {/each}
    {#if recipe.mesoCost > 0}<span class="cost tabular">{formatNumber(recipe.mesoCost)} mesos</span>{/if}
  </div>
</div>

<style>
  .recipe {
    display: grid;
    gap: var(--space-1);
    padding: var(--space-2) 0;
    border-top: 1px solid var(--border);
    font-size: var(--text-md);
  }

  .recipe:first-child {
    border-top: 0;
  }

  .top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
  }

  .makes {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    min-width: 0;
  }

  .makes strong,
  .ing strong {
    font-weight: var(--weight-strong);
  }

  .times {
    color: var(--muted);
    font-size: var(--text-xs);
  }

  .how {
    flex: none;
    color: var(--muted);
    font-size: var(--text-xs);
    white-space: nowrap;
  }

  /* Ingredients as small chips: picture, name, count. */
  .from {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-1);
    font-size: var(--text-xs);
  }

  .ing {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    padding: 1px var(--space-2) 1px var(--space-1);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface);
  }

  .cost {
    margin-left: var(--space-0-5);
    color: var(--mesos);
  }
</style>
