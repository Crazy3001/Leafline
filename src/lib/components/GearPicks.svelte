<script lang="ts">
  // Gear to look for at a level, on this build. Each name opens the item in the Lookup tab. The
  // Character tab shows the top pick of each slot; the editor shows more, and can add them as goals.
  import Ref from "$lib/components/Ref.svelte";
  import { Plus } from "@lucide/svelte";

  import { backend } from "$lib/api";
  import { buildState, watchBuilds } from "$lib/builds.svelte";
  import { gives } from "$lib/items.svelte";
  import { display } from "$lib/settings.svelte";
  import type { GearSuggestions, ItemRecord } from "$lib/types";

  let {
    build,
    level,
    full = false,
    slot = null,
    onAdd,
  }: {
    build: number;
    level: number;
    full?: boolean;
    /** Only this slot, as the item data names it. */
    slot?: string | null;
    onAdd?: (item: ItemRecord) => void;
  } = $props();

  watchBuilds();

  let picks = $state<GearSuggestions | null>(null);

  const key = $derived(JSON.stringify([build, level, buildState.version, display.buildRules]));

  $effect(() => {
    const asked = key;
    backend.gearSuggestions(build, level).then((result) => {
      if (asked === key) picks = result;
    });
  });

  const ranked = $derived.by(() => {
    const names = picks?.rankedBy ?? [];
    return names.length > 1 ? `${names.slice(0, -1).join(", ")}, then ${names[names.length - 1]}` : names.join("");
  });
  const shownSlots = $derived(picks?.slots.filter((s) => slot == null || s.slot === slot) ?? []);
</script>

{#if shownSlots.length}
  <ul class="slots" class:full>
    {#each shownSlots as slot (slot.slot)}
      <li>
        <span class="slot">{slot.slot}</span>
        <div class="picks">
          {#each full ? slot.picks : slot.picks.slice(0, 1) as pick (pick.item.id)}
            <div class="pick">
              <Ref kind="item" id={pick.item.id} class={["name", pick.goal ? "goal" : ""].join(" ")}>
                {pick.item.name}{#if pick.item.gender}<small>{pick.item.gender[0].toUpperCase()}</small>{/if}
              </Ref>
              <span class="gives">
                {#if full && pick.item.reqLevel}Lv {pick.item.reqLevel} · {/if}{gives(pick.item, full ? 3 : 2)}
              </span>
              {#if onAdd}
                <button class="add" disabled={pick.goal} title={pick.goal ? "Already a goal" : "Add as a gear goal"} aria-label="Add as a gear goal" onclick={() => onAdd?.(pick.item)}><Plus size={12} /></button>
              {/if}
            </div>
          {/each}
        </div>
      </li>
    {/each}
  </ul>
  <p class="how">
    Ranked by {ranked}, from what this plan can wear at Lv {picks?.level}. Nobody has checked these in game, and it doesn't
    know prices or how easy an item is to find.
  </p>
{:else if picks}
  <p class="how">Nothing to suggest at Lv {picks.level} yet.</p>
{/if}

<style>
  .slots {
    margin: 0;
    padding: 0;
    list-style: none;
    font-size: var(--text-sm);
  }

  .slots > li {
    display: grid;
    grid-template-columns: 52px 1fr;
    gap: 6px;
    padding: 3px 0;
  }

  .slots > li + li {
    border-top: 1px solid var(--border);
  }

  .slot {
    color: var(--faint);
    font-size: var(--text-xs);
    line-height: 1.5;
  }

  .picks {
    display: grid;
    gap: 2px;
    min-width: 0;
  }

  .pick {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 0 8px;
    min-width: 0;
  }

  li :global(.ref.name) {
    font-size: var(--text-sm);
    font-weight: 600;
  }

  li :global(.ref.name.goal) {
    color: var(--accent-strong);
  }

  li :global(.ref.name small) {
    margin-left: 4px;
    color: var(--faint);
    font-size: var(--text-xs);
    font-weight: 500;
  }

  .gives {
    color: var(--muted);
    font-size: var(--text-xs);
  }

  .add {
    display: inline-grid;
    place-items: center;
    width: 20px;
    height: 20px;
    margin-left: auto;
    padding: 0;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--muted);
    cursor: pointer;
  }

  .add:hover:not(:disabled) {
    color: var(--text);
  }

  .add:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .how {
    margin: 6px 0 0;
    color: var(--faint);
    font-size: var(--text-xs);
  }
</style>
