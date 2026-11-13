<script lang="ts">
  // An item page's crafting: the recipe that makes it and the recipes that use it, from the map
  // data's recipes file. Shows nothing for an item crafting never touches.
  import Section from "../map/Section.svelte";
  import RecipeRow from "./RecipeRow.svelte";
  import { backend } from "$lib/api";
  import type { ItemCrafting } from "$lib/types";

  let { id }: { id: number } = $props();

  let crafting = $state<ItemCrafting | null>(null);

  $effect(() => {
    const wanted = id;
    crafting = null;
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

{#if crafting?.madeBy.length}
  <Section title="Crafted from" count={crafting.madeBy.length}>
    {#each crafting.madeBy as recipe (recipe.id)}<RecipeRow {recipe} current={id} />{/each}
  </Section>
{/if}
{#if crafting?.usedIn.length}
  <Section title="Used in crafting" count={crafting.usedIn.length} open={crafting.usedIn.length <= SHOWN}>
    {#each uses as recipe (recipe.id)}<RecipeRow {recipe} current={id} />{/each}
    {#if crafting.usedIn.length > SHOWN}
      <button class="more" onclick={() => (all = !all)}>{all ? "Fewer" : `${crafting.usedIn.length - SHOWN} more`}</button>
    {/if}
  </Section>
{/if}

<style>
  .more {
    margin-top: 4px;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--accent-strong);
    font: inherit;
    font-size: var(--text-sm);
    cursor: pointer;
  }
</style>
