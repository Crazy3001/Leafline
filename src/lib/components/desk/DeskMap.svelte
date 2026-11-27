<script lang="ts">
  // The desk view's map card on Now: the map you're on, its picture, the level of what lives there,
  // its monsters and their drops as pictures with names, and the ways out. Everything opens its own
  // card. With the game not read, it shows the map the character was last on, marked as such.
  // The picture keeps the map's own shape: a wide map runs across the top, a tall one stands beside.
  import { MapPin } from "@lucide/svelte";
  import { untrack } from "svelte";

  import { backend } from "$lib/api";
  import Ref from "$lib/components/Ref.svelte";
  import { mapData, watchMapData } from "$lib/mapdata.svelte";
  import { ago, shownCharacter, watchProfile } from "$lib/profile.svelte";
  import { live } from "$lib/readings.svelte";
  import { place, watchRegion } from "$lib/region.svelte";
  import Pic from "$lib/ui/Pic.svelte";
  import TitledCard from "$lib/ui/TitledCard.svelte";
  import type { MapDetails } from "$lib/types";

  let { ongo }: { ongo: () => void } = $props();

  watchRegion();
  watchProfile();
  watchMapData();

  const who = $derived(shownCharacter());
  /** The minimap's map, read now from the character on screen; not a frame left from before reading stopped. */
  const liveOn = $derived(live.liveStatus.state === "reading" || live.liveStatus.state === "waiting");
  const reading = $derived(who.onScreen && liveOn ? (live.analysis?.map ?? null) : null);

  // Not read now: the map they were last on, by its name in the profile.
  let remembered = $state<MapDetails | null>(null);
  const lastName = $derived(reading ? null : who.lastMap);
  const ready = $derived(mapData.status?.ready ?? false);
  $effect(() => {
    const name = lastName;
    void ready;
    if (!name || !ready) {
      remembered = null;
      return;
    }
    untrack(() =>
      backend
        .findMap(name, null, null)
        .then((found) => {
          if (name === lastName) remembered = found && found.score >= 0.85 ? found.card : null;
        })
        .catch(() => (remembered = null)),
    );
  });

  // Not read now: the profile's last map, or else the last one matched while reading.
  const card = $derived(reading ? place.card : (remembered ?? place.card));
  /** The map shown isn't read now. */
  const stale = $derived(!reading && card != null);
  const seenAt = $derived(reading ? null : who.seenAt);
  const mobs = $derived(card ? [...card.mobs].sort((a, b) => b.count - a.count).slice(0, 5) : []);
  /** Ways out, once each: two portals to the same map the same way (a dungeon's two doors) are one. */
  const exits = $derived(
    card
      ? card.exits
          .filter((e) => !e.hidden && e.to.known)
          .filter((e, i, all) => all.findIndex((o) => o.to.id === e.to.id && o.direction === e.direction) === i)
          .slice(0, 6)
      : [],
  );
  /** The map's picture showed, and its width over its height. */
  let pictured = $state(false);
  let aspect = $state(1);
  function showPicture(shown: boolean, size: { width: number; height: number } | null) {
    pictured = shown && size != null && size.width > 0 && size.height > 0;
    if (pictured) aspect = size!.width / size!.height;
  }
  /** Twice as wide as tall or more, the picture runs across the card; else it stands beside the rest. */
  const beside = $derived(pictured && aspect < 2);

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

  const WAY: Record<string, string> = { west: "West", east: "East", in: "In", out: "Out", other: "" };
</script>

<TitledCard title="This map" icon={MapPin} tone="violet" more="Map" onmore={ongo}>
  {#if card}
    <div class="body" class:beside style:--aspect={aspect}>
      <div class="pic" class:shown={pictured}>
        {#key card.map.id}<Pic kind="mapview" id={card.map.id} size={null} onstate={showPicture} />{/key}
      </div>
      <div class="info">
        <div>
          <div class="name" class:stale>{card.map.name}</div>
          <div class="chips">
            {#if stale}<span class="seen">{seenAt != null ? `Last seen ${ago(seenAt)}` : "Last seen"}</span>{/if}
            {#if card.region}<span>{card.region}</span>{/if}
            {#if card.map.street && card.map.street !== card.region}<span>{card.map.street}</span>{/if}
            {#if card.levelRange}<span>Monsters Lv {card.levelRange[0]}–{card.levelRange[1]}</span>{/if}
            {#if card.isTown}<span class="town">Town</span>{/if}
          </div>
        </div>
        {#if mobs.length}
          <div class="group">
            <span class="lab">Monsters</span>
            <div class="list">
              {#each mobs as m (m.id)}
                <span class="thing"><Pic kind="mob" id={m.id} size={22} /><Ref kind="mob" id={m.id} name={m.name} icon={false} /><span class="faint tabular">Lv {m.level}</span></span>
              {/each}
            </div>
          </div>
        {/if}
        {#if drops.length}
          <div class="group">
            <span class="lab">Drops</span>
            <div class="list">
              {#each drops as d (d)}<span class="thing"><Pic kind="item" name={d} size={20} /><Ref kind="item" name={d} icon={false} /></span>{/each}
            </div>
          </div>
        {/if}
        {#if exits.length}
          <div class="group">
            <span class="lab">Ways out</span>
            <div class="list">
              <!-- Two ways out can lead to the same map, one across and one through a door. -->
              {#each exits as e (`${e.to.id}:${e.direction}`)}
                <span class="thing">{#if WAY[e.direction]}<span class="faint">{WAY[e.direction]}</span>{/if}<Ref kind="map" id={e.to.id} name={e.to.name} /></span>
              {/each}
            </div>
          </div>
        {/if}
      </div>
    </div>
  {:else if reading}
    <div class="name">{reading.name}</div>
    <p class="quiet">Its monsters and drops show once the map data has it.</p>
  {:else if lastName}
    <div class="name stale">{lastName}</div>
    <p class="quiet">{who.seenAt != null ? `Last seen ${ago(who.seenAt)}. ` : ""}Its monsters and drops show once the map data has it.</p>
  {:else}
    <p class="quiet">The minimap isn't in view. Its map shows here when it is.</p>
  {/if}
</TitledCard>

<style>
  .body {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  /* A tall or square map: the picture on the left, the rest beside it. */
  .body.beside {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    align-items: start;
    gap: var(--space-4);
  }

  .info {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    min-width: 0;
  }

  /*
   * The whole map at its own shape, never cropped: across the card up to 220px tall, or beside
   * the rest up to 300px tall and under half the card's width.
   */
  .pic {
    display: none;
    overflow: hidden;
    border-radius: var(--radius);
    background: var(--surface-2);
  }

  .pic.shown {
    display: block;
    width: min(100%, calc(220px * var(--aspect)));
    aspect-ratio: var(--aspect);
    margin: 0 auto;
  }

  .beside .pic.shown {
    width: min(calc(300px * var(--aspect)), 45cqw);
    margin: 0;
  }

  .pic :global(img) {
    display: block;
  }

  .name {
    font-family: var(--font-display);
    font-size: var(--text-lg);
    font-weight: var(--weight-strong);
  }

  /* Remembered from an earlier read, not read now. */
  .stale {
    opacity: 0.7;
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

  .chips .seen {
    background: var(--warn-soft);
    color: var(--warn);
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

  .thing {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1-5);
  }

  .faint {
    color: var(--faint);
    font-size: var(--text-xs);
  }

  .quiet {
    margin: 0;
    color: var(--muted);
  }

  /* A narrow card stacks a tall picture over the rest, centred. */
  @container tcard (max-width: 460px) {
    .body.beside {
      display: flex;
    }

    .beside .pic.shown {
      width: min(calc(240px * var(--aspect)), 100%);
      margin: 0 auto;
    }
  }
</style>
