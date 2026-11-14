<script lang="ts">
  // Lookup → Crafting: every recipe in the map data. Pick a discipline from its tile, narrow by
  // the skill level you have, or search by the item made or any ingredient. Recipes are grouped
  // by the skill level they ask for, and every item opens its own card.
  import { untrack } from "svelte";

  import MapDataPanel from "../map/MapDataPanel.svelte";
  import SearchBar from "../lookup/SearchBar.svelte";
  import Chips from "../lookup/Chips.svelte";
  import RecipeRow from "./RecipeRow.svelte";
  import { backend } from "$lib/api";
  import { mapData, watchMapData } from "$lib/mapdata.svelte";
  import type { CraftingPage, Recipe } from "$lib/types";
  import Pic from "$lib/ui/Pic.svelte";

  let { pinned = false }: { pinned?: boolean } = $props();

  const DISCIPLINE_KEY = "leafline.crafting.discipline";
  const SKILL_LEVELS = Array.from({ length: 10 }, (_, i) => String(i + 1));

  watchMapData();

  const ready = $derived(mapData.status?.ready ?? false);
  let discipline = $state(stored(DISCIPLINE_KEY));
  let search = $state("");
  let maxLevel = $state<number | null>(null);
  let page = $state<CraftingPage | null>(null);
  /** Every recipe, once, for each discipline's picture: the first thing it makes. */
  let all = $state<Recipe[]>([]);

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
    untrack(() => backend.craftingRecipes({ discipline: discipline || null, search: search.trim() || null, maxLevel }))
      .then((result) => {
        if (key === wanted) page = result;
      })
      .catch(() => (page = null));
  });

  $effect(() => {
    void mapData.status?.dataUpdatedAt;
    if (!ready) return;
    backend
      .craftingRecipes({ discipline: null, search: null, maxLevel: null })
      .then((result) => (all = result.recipes))
      .catch(() => (all = []));
  });

  /** The item each discipline's tile shows: the first thing it makes, at its lowest level. */
  const emblem = $derived.by(() => {
    const out = new Map<string, { id: number; name: string }>();
    for (const r of [...all].sort((a, b) => a.skillLevel - b.skillLevel || a.id - b.id)) {
      if (!out.has(r.discipline)) out.set(r.discipline, { id: r.output.id, name: r.output.name });
    }
    return out;
  });

  /** Recipes under a heading per discipline and skill level, lowest first. */
  const groups = $derived.by(() => {
    const out: { title: string; recipes: Recipe[] }[] = [];
    const sorted = [...(page?.recipes ?? [])].sort(
      (a, b) => a.discipline.localeCompare(b.discipline) || a.skillLevel - b.skillLevel || a.id - b.id,
    );
    for (const r of sorted) {
      const title = discipline ? `Skill Lv ${r.skillLevel}` : `${r.discipline} · Lv ${r.skillLevel}`;
      const last = out[out.length - 1];
      if (last?.title === title) last.recipes.push(r);
      else out.push({ title, recipes: [r] });
    }
    return out;
  });
</script>

{#if !ready}
  {#if mapData.status}<MapDataPanel compact />{/if}
{:else}
  <div class="controls" class:pinned>
    <SearchBar
      value={search}
      placeholder="Item or ingredient"
      label="Search recipes"
      filterable={false}
      clearable={search !== "" || discipline !== "" || maxLevel != null}
      oninput={(text) => (search = text)}
      onclear={() => {
        search = "";
        maxLevel = null;
        pick("");
      }}
    />
    {#if page?.disciplines.length}
      <div class="disciplines" role="radiogroup" aria-label="Discipline">
        <button role="radio" aria-checked={discipline === ""} onclick={() => pick("")}>
          <span class="all-mark" aria-hidden="true">All</span>
          <span class="name">Every discipline</span>
        </button>
        {#each page.disciplines as [name, count] (name)}
          {@const mark = emblem.get(name)}
          <button role="radio" aria-checked={discipline === name} onclick={() => pick(discipline === name ? "" : name)}>
            {#if mark}<Pic kind="item" id={mark.id} name={mark.name} size={26} />{/if}
            <span class="name">{name}</span>
            <span class="n tabular">{count}</span>
          </button>
        {/each}
      </div>
      <Chips
        label="Your skill level"
        caption="Up to Lv"
        options={SKILL_LEVELS}
        value={maxLevel == null ? null : String(maxLevel)}
        onselect={(level) => (maxLevel = level == null ? null : Number(level))}
      />
    {/if}
  </div>

  <div class="list" class:pinned>
    {#if !page}
      <p class="quiet">Loading recipes…</p>
    {:else if !page.disciplines.length}
      <p class="quiet">The map data has no recipes yet. It comes with the next map data download.</p>
    {:else if !page.recipes.length}
      <p class="quiet">No recipes match.</p>
    {:else}
      <p class="count tabular">{page.recipes.length} recipe{page.recipes.length === 1 ? "" : "s"}</p>
      {#each groups as group (group.title)}
        <section>
          <h3>{group.title} <span class="tabular">{group.recipes.length}</span></h3>
          {#each group.recipes as recipe (recipe.id)}<RecipeRow {recipe} discipline={false} />{/each}
        </section>
      {/each}
    {/if}
  </div>
{/if}

<style>
  /* Padded like the other halves of the Lookup tab; a pinned window pads its body. */
  .controls {
    padding: 8px 12px 0;
  }

  .list {
    padding: 0 12px 10px;
  }

  .controls.pinned,
  .list.pinned {
    padding: 0;
  }

  /* A tile per discipline: what it makes, its name and how many recipes. */
  .disciplines {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(118px, 1fr));
    gap: 4px;
    margin-bottom: 6px;
  }

  .disciplines button {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    height: 36px;
    padding: 0 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
    text-align: left;
    cursor: pointer;
    transition:
      background 0.12s ease,
      border-color 0.12s ease;
  }

  .disciplines button:hover {
    background: var(--surface-2);
  }

  .disciplines button[aria-checked="true"] {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 12%, var(--surface));
  }

  .disciplines .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .disciplines .n {
    color: var(--faint);
    font-size: var(--text-xs);
  }

  .all-mark {
    display: inline-grid;
    flex: none;
    place-items: center;
    width: 26px;
    height: 26px;
    border-radius: var(--radius-sm);
    background: var(--surface-3);
    color: var(--muted);
    font-size: 10px;
    font-weight: 600;
  }

  h3 {
    display: flex;
    align-items: baseline;
    gap: 6px;
    margin: var(--space-3) 0 2px;
    color: var(--faint);
    font-family: var(--font-display);
    font-size: var(--text-xs);
    font-weight: 600;
    letter-spacing: 0.05em;
    text-transform: uppercase;
  }

  .count,
  .quiet {
    margin: 0;
    color: var(--faint);
    font-size: var(--text-sm);
  }
</style>
