<script lang="ts">
  // Levelling a crafting discipline: with the EXP the game's crafting window shows, the recipes
  // open at your skill level ranked by what each point of EXP costs or by the fewest crafts, with
  // how many crafts it takes and what to buy or farm. The EXP each level needs isn't in the map
  // data, so it is typed from the game. Ingredients count at the cheapest NPC shop price, and as
  // "to farm" when no shop sells them.
  import { untrack } from "svelte";

  import Ref from "../Ref.svelte";
  import { backend } from "$lib/api";
  import { formatMesos, formatNumber } from "$lib/format";
  import type { Recipe } from "$lib/types";
  import Pic from "$lib/ui/Pic.svelte";

  let { discipline, recipes }: { discipline: string; recipes: Recipe[] } = $props();

  const SHOWN = 6;
  const KEY = "leafline.crafting.levelling";
  type Sort = "cheap" | "few";
  type Saved = { level: number; now: number | null; need: number | null; sort: Sort };

  function load(): Record<string, Saved> {
    try {
      return JSON.parse(localStorage.getItem(KEY) ?? "{}") as Record<string, Saved>;
    } catch {
      return {};
    }
  }

  let saved = $state<Record<string, Saved>>(load());
  const mine = $derived<Saved>(saved[discipline] ?? { level: 1, now: null, need: null, sort: "cheap" });

  function set(change: Partial<Saved>) {
    saved = { ...saved, [discipline]: { ...mine, ...change } };
    try {
      localStorage.setItem(KEY, JSON.stringify(saved));
    } catch {
      // A convenience only.
    }
  }

  // The cheapest shop price of every ingredient, fetched once per discipline.
  let prices = $state<Map<number, number>>(new Map());
  const ingredientKey = $derived([...new Set(recipes.flatMap((r) => r.ingredients.map((i) => i.id)))].sort((a, b) => a - b).join(","));
  $effect(() => {
    const k = ingredientKey;
    const ids = k ? k.split(",").map(Number) : [];
    untrack(() => backend.shopPrices(ids))
      .then((found) => {
        if (k === ingredientKey) prices = new Map(found);
      })
      .catch(() => (prices = new Map()));
  });

  const levels = $derived([...new Set(recipes.map((r) => r.skillLevel))].sort((a, b) => a - b));
  const left = $derived(mine.need != null ? Math.max(0, mine.need - (mine.now ?? 0)) : null);

  interface Plan {
    recipe: Recipe;
    crafts: number | null;
    /** Mesos a craft costs: the fee and the ingredients a shop sells. */
    perCraft: number;
    /** Mesos for each point of EXP. */
    perExp: number;
    /** Ingredients no shop sells, which have to be farmed or made. */
    farm: Recipe["ingredients"];
  }

  const plans = $derived.by((): Plan[] => {
    const open = recipes.filter((r) => r.craftExp > 0 && r.skillLevel <= mine.level);
    const list = open.map((recipe) => {
      const bought = recipe.ingredients.reduce((sum, i) => sum + (prices.get(i.id) ?? 0) * i.count, 0);
      const perCraft = recipe.mesoCost + bought;
      return {
        recipe,
        crafts: left != null ? Math.ceil(left / recipe.craftExp) : null,
        perCraft,
        perExp: perCraft / recipe.craftExp,
        farm: recipe.ingredients.filter((i) => !prices.has(i.id)),
      };
    });
    // Cheap: mesos per EXP, with less farming first among equals. Few: most EXP a craft.
    return list.sort((a, b) =>
      mine.sort === "few"
        ? b.recipe.craftExp - a.recipe.craftExp || a.perExp - b.perExp
        : a.perExp - b.perExp || a.farm.length - b.farm.length || b.recipe.craftExp - a.recipe.craftExp,
    );
  });

  const num = (v: string) => (v.trim() === "" ? null : Math.max(0, Math.floor(Number(v)) || 0));
</script>

<div class="levelling">
  <div class="inputs">
    <label>
      <span>Skill level</span>
      <select value={mine.level} onchange={(e) => set({ level: Number(e.currentTarget.value) })}>
        {#each levels as level (level)}<option value={level}>Lv {level}</option>{/each}
      </select>
    </label>
    <label>
      <span>EXP now</span>
      <input class="tabular" type="number" min="0" value={mine.now ?? ""} oninput={(e) => set({ now: num(e.currentTarget.value) })} />
    </label>
    <label>
      <span>EXP to level</span>
      <input class="tabular" type="number" min="0" placeholder="from the game" value={mine.need ?? ""} oninput={(e) => set({ need: num(e.currentTarget.value) })} />
    </label>
    <div class="sort" role="group" aria-label="Rank recipes by">
      <button aria-pressed={mine.sort === "cheap"} onclick={() => set({ sort: "cheap" })}>Fewest mesos</button>
      <button aria-pressed={mine.sort === "few"} onclick={() => set({ sort: "few" })}>Fewest crafts</button>
    </div>
  </div>

  {#if !plans.length}
    <p class="note">No {discipline} recipe gives EXP at Lv {mine.level}.</p>
  {:else}
    <ol class="plans">
      {#each plans.slice(0, SHOWN) as plan, i (plan.recipe.id)}
        <li class:best={i === 0}>
          <div class="head">
            <Pic kind="item" id={plan.recipe.output.id} name={plan.recipe.output.name} size={20} />
            <Ref kind="item" id={plan.recipe.output.id} name={plan.recipe.output.name} icon={false} />
            <span class="facts tabular"
              >{plan.recipe.craftExp} EXP · {plan.perCraft ? `${formatMesos(plan.perCraft)} mesos` : "free"} a craft</span
            >
          </div>
          <p class="line tabular">
            {#if plan.crafts != null}
              <b>{formatNumber(plan.crafts)} crafts</b>{plan.perCraft ? `, about ${formatMesos(plan.perCraft * plan.crafts)} mesos` : ""}
            {:else}
              {plan.perExp ? `${plan.perExp.toFixed(1)} mesos an EXP` : "Costs nothing but ingredients to farm"}
            {/if}
            {#if plan.farm.length}
              <span class="farm"
                >· to farm or make: {plan.farm
                  .map((f) => `${f.name} ×${formatNumber(f.count * (plan.crafts ?? 1))}`)
                  .join(", ")}</span
              >
            {/if}
          </p>
        </li>
      {/each}
    </ol>
  {/if}
  <p class="note">
    Type the EXP your crafting window shows; Leafline doesn't have the EXP each level needs. Ingredients count at the
    cheapest NPC shop's launch price; the rest you farm or make.
  </p>
</div>

<style>
  .levelling {
    margin: 4px 0 8px;
    padding: 8px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
  }

  .inputs {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(96px, 1fr));
    gap: 6px 8px;
    align-items: end;
  }

  label {
    display: grid;
    gap: 2px;
    color: var(--faint);
    font-size: var(--text-xs);
  }

  input,
  select {
    width: 100%;
    height: 26px;
    padding: 0 6px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    color: var(--text);
  }

  .sort {
    display: inline-flex;
    grid-column: span 2;
    gap: 2px;
    padding: 2px;
    border-radius: var(--radius-sm);
    background: var(--hover);
  }

  .sort button {
    flex: 1;
    padding: 3px 6px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--muted);
    font-size: var(--text-xs);
    cursor: pointer;
  }

  .sort button[aria-pressed="true"] {
    background: var(--accent-soft);
    color: var(--text);
  }

  .plans {
    display: grid;
    gap: 6px;
    margin: 10px 0 0;
    padding: 0;
    list-style: none;
  }

  .plans li {
    padding: 5px 7px;
    border-radius: var(--radius-sm);
    background: var(--hover);
  }

  .plans li.best {
    box-shadow: inset 2px 0 0 var(--accent);
  }

  .head {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: var(--text-sm);
    font-weight: 600;
  }

  .head .facts {
    margin-left: auto;
    color: var(--faint);
    font-size: var(--text-xs);
    font-weight: 400;
  }

  .line {
    margin: 2px 0 0;
    color: var(--muted);
    font-size: var(--text-xs);
  }

  .line b {
    color: var(--text);
  }

  .farm {
    color: var(--faint);
  }

  .note {
    margin: 8px 0 0;
    color: var(--faint);
    font-size: var(--text-xs);
  }
</style>
