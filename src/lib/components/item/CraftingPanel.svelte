<script lang="ts">
  // Lookup → Crafting: every recipe in the map data, by discipline and skill level, searchable by
  // the item made or any ingredient. Each item opens its own page.
  import { untrack } from "svelte";

  import MapDataPanel from "../map/MapDataPanel.svelte";
  import RecipeRow from "./RecipeRow.svelte";
  import { backend } from "$lib/api";
  import { mapData, watchMapData } from "$lib/mapdata.svelte";
  import type { CraftingPage } from "$lib/types";

  let { pinned = false }: { pinned?: boolean } = $props();

  const DISCIPLINE_KEY = "leafline.crafting.discipline";

  watchMapData();
  const ready = $derived(mapData.status?.ready ?? false);

  let discipline = $state(stored(DISCIPLINE_KEY));
  let search = $state("");
  let maxLevel = $state<number | null>(null);
  let page = $state<CraftingPage | null>(null);

  function stored(key: string) {
    try {
      return localStorage.getItem(key) ?? "";
    } catch {
      return "";
    }
  }

  function pick(next: string) {
    discipline = next;
    try {
      localStorage.setItem(DISCIPLINE_KEY, next);
    } catch {
      // A convenience only.
    }
  }

  const key = $derived(JSON.stringify([discipline, search.trim(), maxLevel]));
  $effect(() => {
    const wanted = key;
    void mapData.status?.dataUpdatedAt;
    if (!ready) return;
    untrack(() =>
      backend.craftingRecipes({ discipline: discipline || null, search: search.trim() || null, maxLevel }),
    )
      .then((result) => {
        if (key === wanted) page = result;
      })
      .catch(() => (page = null));
  });

  /** Recipes in runs of one discipline, for headings. */
  const groups = $derived.by(() => {
    const out: { discipline: string; recipes: CraftingPage["recipes"] }[] = [];
    for (const r of page?.recipes ?? []) {
      const last = out[out.length - 1];
      if (last?.discipline === r.discipline) last.recipes.push(r);
      else out.push({ discipline: r.discipline, recipes: [r] });
    }
    return out;
  });
</script>

{#if !ready}
  {#if mapData.status}<MapDataPanel compact />{/if}
{:else}
  <div class="filters" class:pinned>
    <input class="search" type="search" placeholder="Item or ingredient" aria-label="Search recipes" bind:value={search} />
    <select aria-label="Discipline" value={discipline} onchange={(e) => pick(e.currentTarget.value)}>
      <option value="">Every discipline</option>
      {#each page?.disciplines ?? [] as [name, count] (name)}<option value={name}>{name} ({count})</option>{/each}
    </select>
    <label class="level">
      Up to skill
      <select
        aria-label="Highest skill level"
        value={maxLevel ?? ""}
        onchange={(e) => (maxLevel = e.currentTarget.value ? Number(e.currentTarget.value) : null)}
      >
        <option value="">any</option>
        {#each Array.from({ length: 10 }, (_, i) => i + 1) as n (n)}<option value={n}>{n}</option>{/each}
      </select>
    </label>
  </div>

  {#if !page}
    <p class="quiet">Loading recipes…</p>
  {:else if !page.disciplines.length}
    <p class="quiet">The map data has no recipes yet. It comes with the next map data download.</p>
  {:else if !page.recipes.length}
    <p class="quiet">No recipes match.</p>
  {:else}
    <p class="count">{page.recipes.length} recipe{page.recipes.length === 1 ? "" : "s"}</p>
    {#each groups as group (group.discipline)}
      <section>
        {#if !discipline}<h3>{group.discipline}</h3>{/if}
        {#each group.recipes as recipe (recipe.id)}<RecipeRow {recipe} />{/each}
      </section>
    {/each}
  {/if}
{/if}

<style>
  .filters {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-bottom: var(--space-2);
  }

  .search {
    flex: 1 1 140px;
    min-width: 0;
  }

  input,
  select {
    height: var(--control-h);
    padding: 0 6px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
  }

  .level {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: var(--text-sm);
    color: var(--muted);
  }

  h3 {
    margin: var(--space-3) 0 2px;
    font-family: var(--font-display);
    font-size: var(--text-md);
    font-weight: 400;
    color: var(--muted);
  }

  .count,
  .quiet {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--faint);
  }
</style>
