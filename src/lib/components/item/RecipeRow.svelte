<script lang="ts">
  // One crafting recipe: what it makes, the discipline and skill level it needs, and what goes
  // into it. Every item opens its own page.
  import Ref from "$lib/components/Ref.svelte";
  import { backend } from "$lib/api";
  import { formatNumber } from "$lib/format";
  import type { Recipe } from "$lib/types";

  let {
    recipe,
    /** The item the page is about, left unlinked. */
    current = null,
  }: { recipe: Recipe; current?: number | null } = $props();

  const amount = (count: number) => (count > 1 ? `${formatNumber(count)} × ` : "");
</script>

<div class="recipe">
  <div class="top">
    <span class="makes">
      {amount(recipe.output.count)}{#if recipe.output.id === current}<strong>{recipe.output.name}</strong>{:else}<Ref kind="item" id={recipe.output.id} name={recipe.output.name} />{/if}
    </span>
    <span class="how">{recipe.discipline} {recipe.skillLevel}</span>
  </div>
  <div class="from">
    {#each recipe.ingredients as g, i (g.id)}
      {i ? ", " : ""}{amount(g.count) || "1 × "}{#if g.id === current}<strong>{g.name}</strong>{:else}<Ref kind="item" id={g.id} name={g.name} />{/if}
    {/each}
    {#if recipe.mesoCost > 0}<span class="cost">{" · "}{formatNumber(recipe.mesoCost)} mesos</span>{/if}
  </div>
</div>

<style>
  .recipe {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 6px 0;
    border-top: 1px solid var(--border);
    font-size: var(--text-sm);
  }

  .recipe:first-child {
    border-top: 0;
  }

  .top {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 8px;
  }

  .makes strong {
    font-weight: 600;
  }

  .how {
    flex: none;
    font-size: var(--text-xs);
    color: var(--muted);
    white-space: nowrap;
  }

  .from {
    font-size: var(--text-xs);
    color: var(--muted);
  }

  .from strong {
    font-weight: 600;
    color: var(--text);
  }

  .cost {
    color: var(--faint);
  }
</style>
