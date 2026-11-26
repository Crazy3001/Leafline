<script lang="ts">
  // An item page's crafting: the recipe that makes it and the recipes that use it, from the map
  // data's recipes file. Shows nothing for an item crafting never touches.
  import { ChevronRight } from "@lucide/svelte";

  import Section from "../map/Section.svelte";
  import RecipeRow from "./RecipeRow.svelte";
  import { backend } from "$lib/api";
  import type { ItemCrafting } from "$lib/types";

  let {
    id,
    part,
  }: {
    id: number;
    /** "made": the recipes that make it, a group under Where to get it. "used": what it goes into. */
    part: "made" | "used";
  } = $props();
  /** Folded, like the other ways to get it, for each item opened. */
  let madeOpen = $state(false);

  let crafting = $state<ItemCrafting | null>(null);

  $effect(() => {
    const wanted = id;
    crafting = null;
    madeOpen = false;
    backend
      .itemCrafting(wanted)
      .then((found) => {
        if (id === wanted) crafting = found;
      })
      .catch(() => {});
  });

  /** More uses than this fold behind a button. */
  const SHOWN = 6;
  let all = $state(false);
  const uses = $derived(crafting ? (all ? crafting.usedIn : crafting.usedIn.slice(0, SHOWN)) : []);
</script>

{#if part === "made" && crafting?.madeBy.length}
  <div class="group">
    <button class="group-head" aria-expanded={madeOpen} onclick={() => (madeOpen = !madeOpen)}>
      <span class="chevron" class:open={madeOpen}><ChevronRight size={12} /></span>
      <span class="group-title">Crafted from</span>
      <span class="count tabular">{crafting.madeBy.length}</span>
    </button>
    {#if madeOpen}
      {#each crafting.madeBy as recipe (recipe.id)}<RecipeRow {recipe} current={id} />{/each}
    {/if}
  </div>
{/if}
{#if part === "used" && crafting?.usedIn.length}
  <Section title="Used in crafting" count={crafting.usedIn.length} open={crafting.usedIn.length <= SHOWN}>
    {#each uses as recipe (recipe.id)}<RecipeRow {recipe} current={id} />{/each}
    {#if crafting.usedIn.length > SHOWN}
      <button class="more" onclick={() => (all = !all)}>{all ? "Fewer" : `${crafting.usedIn.length - SHOWN} more`}</button>
    {/if}
  </Section>
{/if}

<style>
  .more {
    margin-top: var(--space-1);
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--accent-text-strong);
    font: inherit;
    font-size: var(--text-md);
    cursor: pointer;
  }

  /* The same group heads as the other ways to get an item. */
  .group {
    margin-top: var(--space-1-5);
  }

  .group-head {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    width: 100%;
    padding: var(--space-1) 0;
    border: 0;
    border-bottom: 1px solid var(--border);
    background: transparent;
    text-align: left;
    cursor: pointer;
  }

  .chevron {
    display: inline-grid;
    color: var(--faint);
    transition: transform 0.12s ease;
  }

  .chevron.open {
    transform: rotate(90deg);
  }

  .group-title {
    color: var(--muted);
    font-size: var(--text-xs);
    font-weight: var(--weight-strong);
  }

  .count {
    color: var(--faint);
    font-size: var(--text-xs);
  }
</style>
