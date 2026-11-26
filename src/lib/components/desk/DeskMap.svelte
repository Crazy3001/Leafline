<script lang="ts">
  // The desk view's map card on Now: the map you're on, its picture, the level of what lives there,
  // and its monsters and drops as pictures with names. Everything opens its own card.
  import { MapPin } from "@lucide/svelte";
  import { untrack } from "svelte";

  import { backend } from "$lib/api";
  import Ref from "$lib/components/Ref.svelte";
  import { live } from "$lib/readings.svelte";
  import { place, watchRegion } from "$lib/region.svelte";
  import Pic from "$lib/ui/Pic.svelte";
  import TitledCard from "$lib/ui/TitledCard.svelte";

  let { ongo }: { ongo: () => void } = $props();

  watchRegion();

  const card = $derived(place.card);
  const reading = $derived(live.analysis?.map ?? null);
  const mobs = $derived(card ? [...card.mobs].sort((a, b) => b.count - a.count).slice(0, 5) : []);
  let pictured = $state(false);

  /** The most likely drops of the monsters here, a few at most, from the drop list on this PC. */
  let drops = $state<string[]>([]);
  const mobKey = $derived(mobs.map((m) => m.name).join("|"));
  $effect(() => {
    const key = mobKey;
    if (!key) {
      drops = [];
      return;
    }
    untrack(() =>
      backend
        .monsterDrops(mobs.map((m) => m.name))
        .then((found) => {
          if (key !== mobKey) return;
          const seen = new Set<string>();
          const items: string[] = [];
          for (const m of found) for (const d of m.drops) if (!seen.has(d.item) && seen.add(d.item)) items.push(d.item);
          drops = items.slice(0, 6);
        })
        .catch(() => (drops = [])),
    );
  });
</script>

<TitledCard title="This map" icon={MapPin} tone="violet" more="Map" onmore={ongo}>
  {#if card}
    <div class="pic" class:shown={pictured}>
      {#key card.map.id}<Pic kind="mapview" id={card.map.id} size={null} fit="cover" onstate={(s) => (pictured = s)} />{/key}
    </div>
    <div>
      <div class="name">{card.map.name}</div>
      <div class="chips">
        {#if card.map.street}<span>{card.map.street}</span>{/if}
        {#if card.levelRange}<span>Monsters Lv {card.levelRange[0]}–{card.levelRange[1]}</span>{/if}
        {#if card.isTown}<span class="town">Town</span>{/if}
      </div>
    </div>
    {#if mobs.length}
      <div class="group">
        <span class="lab">Monsters</span>
        <div class="list">{#each mobs as m (m.id)}<Ref kind="mob" id={m.id} name={m.name} />{/each}</div>
      </div>
    {/if}
    {#if drops.length}
      <div class="group">
        <span class="lab">Drops</span>
        <div class="list">{#each drops as d (d)}<Ref kind="item" name={d} />{/each}</div>
      </div>
    {/if}
  {:else if reading}
    <div class="name">{reading.name}</div>
    <p class="quiet">Its monsters and drops show once the map data has it.</p>
  {:else}
    <p class="quiet">The minimap isn't in view. Its map shows here when it is.</p>
  {/if}
</TitledCard>

<style>
  .pic {
    display: none;
    height: 120px;
    overflow: hidden;
    border-radius: var(--radius);
    background: var(--surface-2);
  }

  .pic.shown {
    display: block;
  }

  .pic :global(img) {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .name {
    font-family: var(--font-display);
    font-size: var(--text-lg);
    font-weight: var(--weight-strong);
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1-5);
    margin-top: var(--space-1);
  }

  .chips span {
    padding: var(--space-0-5) var(--space-2);
    border-radius: var(--radius-pill);
    background: var(--surface-2);
    color: var(--muted);
    font-size: var(--text-xs);
  }

  .chips .town {
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .group {
    display: flex;
    flex-direction: column;
    gap: var(--space-1-5);
  }

  .lab {
    color: var(--faint);
    font-size: var(--text-xs);
    font-weight: var(--weight-strong);
  }

  .list {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1-5) var(--space-4);
  }

  .quiet {
    margin: 0;
    color: var(--muted);
  }
</style>
