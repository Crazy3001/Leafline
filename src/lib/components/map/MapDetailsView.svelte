<script lang="ts" module>
  import type { MapDetails as Details } from "$lib/types";

  /** A map in a few words: a town or not, its monsters' levels, its region. */
  export function mapFacts(details: Details): string[] {
    return [
      details.isTown ? "Town" : null,
      details.levelRange
        ? details.levelRange[0] === details.levelRange[1]
          ? `Lv ${details.levelRange[0]}`
          : `Lv ${details.levelRange[0]}–${details.levelRange[1]}`
        : null,
      details.region,
    ].filter((f): f is string => !!f);
  }
</script>

<script lang="ts">
  // Everything the cached data says about one map: its picture with the marks on it, where its
  // portals lead, its monsters, drops, quests, NPCs and shops.
  import { tip } from "$lib/tip.svelte";
  import Ref from "$lib/components/Ref.svelte";
  import { notDone, questCount, watchQuestLog } from "$lib/questlog.svelte";
  import { Skull } from "@lucide/svelte";

  import { backend } from "$lib/api";
  import { formatCompact, formatNumber, monsterLabel } from "$lib/format";
  import type { MapDetails, MapPicture as Picture, MapRef } from "$lib/types";
  import { trip } from "$lib/wayfinder.svelte";
  import ConnectionCompass from "./ConnectionCompass.svelte";
  import MapPicture from "./MapPicture.svelte";
  import { display } from "$lib/settings.svelte";
  import MapDrops from "./MapDrops.svelte";
  import ShopStock from "../item/ShopStock.svelte";
  import RouteFromHere from "./RouteFromHere.svelte";
  import RoutePlanner from "./RoutePlanner.svelte";
  import Section, { SECTION_LOOK } from "./Section.svelte";
  import { setContext } from "svelte";
  import Segmented from "$lib/ui/Segmented.svelte";

  let {
    details,
    current = null,
    variant = "full",
    split = false,
    onnavigate,
  }: {
    details: MapDetails;
    /** The map the player is on, when known. */
    current?: MapRef | null;
    /** `glance` keeps the facts and the monsters, for a pinned window. */
    variant?: "full" | "compact" | "glance";
    /** In the Map tab: each section a card of its own with an id to jump to, and the facts left
     *  to the map's own card above. */
    split?: boolean;
    onnavigate: (id: number) => void;
  } = $props();

  const compact = $derived(variant !== "full");
  const glance = $derived(variant === "glance");
  // svelte-ignore state_referenced_locally
  if (split) setContext(SECTION_LOOK, "cards");
  /** A section's id, where chips jump to it. */
  const anchor = (name: string) => (split ? `map-${name}` : undefined);
  watchQuestLog();
  /** The player's map, while they look at another. */
  const away = $derived(current && current.id !== details.map.id ? current : null);
  const placed = $derived(details.exits.some((e) => e.side));
  /** Shops opened to show what they sell, by NPC name. */
  let openShops = $state<string[]>([]);

  const facts = $derived(mapFacts(details));
  const monsterNames = $derived([...new Set(details.mobs.map((m) => m.name))]);
  /** The desk view has room for the map's picture; over the game it waits to be opened. */
  const desk = typeof location !== "undefined" && location.pathname.startsWith("/desk");
  /** The monster under the pointer in the list, whose spawn points stand out on the picture. */
  let hovered = $state<string | null>(null);

  // The picture's size and marks, for the picture and for placing the portals where they stand.
  let picture = $state<Picture | null>(null);
  let pictureDone = $state(false);
  $effect(() => {
    const id = details.map.id;
    picture = null;
    pictureDone = false;
    if (!display.showPictures) return;
    backend
      .mapPicture(id)
      .then((result) => {
        if (id === details.map.id) picture = result;
      })
      .catch(() => {})
      .finally(() => {
        if (id === details.map.id) pictureDone = true;
      });
  });

  /** A map whose portal is pointed at, in the Portals section or on the picture: lit in both. */
  let litExit = $state<number | null>(null);

  // The Portals section as a compass or a plain list, remembered as a convenience.
  type Layout = "compass" | "list";
  const LAYOUT_KEY = "leafline.portals.layout";
  let layout = $state<Layout>(
    (() => {
      try {
        return localStorage.getItem(LAYOUT_KEY) === "list" ? "list" : "compass";
      } catch {
        return "compass";
      }
    })(),
  );
  function setLayout(value: Layout) {
    layout = value;
    try {
      localStorage.setItem(LAYOUT_KEY, value);
    } catch {
      // A convenience only.
    }
  }
  /** Placed from the picture: the portals' real spots, rather than only the data's thirds. */
  const located = $derived(picture?.points.some((p) => p.kind === "portal" && !p.teleport) ?? false);

  /** The portal the trip being followed takes next, when it leaves from this map. */
  const nextKey = $derived.by(() => {
    const step = trip.route?.steps[trip.next];
    return step && !step.travel && step.from.id === details.map.id ? JSON.stringify([step.portal, step.to.id]) : null;
  });
  const nextStep = $derived.by(() => {
    if (!nextKey) return null;
    const [portal, to] = JSON.parse(nextKey) as [string | null, number];
    return { portal, to };
  });
</script>

{#if !split && (facts.length || details.returnMap)}
  <div class="facts">
    {#each facts as fact, i}<span class="fact" class:town={i === 0 && details.isTown}>{fact}</span>{/each}
    {#if details.returnMap}
      <span class="return">
        Returns to
        <Ref kind="map" id={details.returnMap.id} name={details.returnMap.name} />
      </span>
    {/if}
  </div>
{/if}

{#if display.showPictures}
  <Section title={split ? "Picture" : "Map"} memory="map-picture-{desk ? 'desk' : 'overlay'}" open={desk} anchor={anchor("picture")}>
    <MapPicture
      mapId={details.map.id}
      {picture}
      done={pictureDone}
      highlight={hovered}
      mobs={details.mobs}
      npcs={details.npcs}
      next={nextStep}
      lit={litExit}
      onhoverportal={(id) => (litExit = id)}
    />
  </Section>
{/if}

{#if away}
  <Section title="Route from your map">
    <RouteFromHere from={away} to={details.map} />
  </Section>
{/if}

{#if !glance}
  <Section title="Portals" count={details.exits.length} anchor={anchor("portals")}>
    {#snippet aside()}
      {#if placed && !located}<span>rough positions</span>{/if}
    {/snippet}
    {#if details.exits.length > 2}
      <div class="portal-layout">
        <Segmented
          size="sm"
          label="Show the portals as"
          value={layout}
          options={[
            { id: "compass", label: "Compass", title: "Around the map, on the side each portal stands" },
            { id: "list", label: "List", title: "Every portal in a row, left to right across the map" },
          ]}
          onchange={setLayout}
        />
      </div>
    {/if}
    <ConnectionCompass
      exits={details.exits}
      map={details.map}
      {current}
      {layout}
      points={picture?.points ?? null}
      next={nextStep}
      lit={litExit}
      onhover={(id) => (litExit = id)}
      {onnavigate}
    />
  </Section>
{/if}

{#if details.mobs.length}
  <Section title="Monsters" count={details.mobTotal} anchor={anchor("monsters")}>
    {#snippet aside()}<span use:tip={"Base EXP for killing every monster once, before any modifiers"}
        >{formatCompact(details.expPerClear)} EXP / clear</span
      >{/snippet}
    <table class="mobs">
      <thead>
        <tr><th>Monster</th><th>Lv</th><th>HP</th><th>EXP</th><th use:tip={"How many spawn"}>#</th></tr>
      </thead>
      <tbody>
        {#each details.mobs as mob (mob.id)}
          <tr onmouseenter={() => (hovered = mob.name)} onmouseleave={() => (hovered = null)}>
            <td class="mob">
              <Ref kind="mob" id={mob.id} name={monsterLabel(mob.name)} />
              {#if mob.boss}<span class="boss" use:tip={"Boss"}><Skull size={11} /></span>{/if}
              {#if mob.undead}<span class="tag">undead</span>{/if}
            </td>
            <td class="tabular">{mob.level}</td>
            <td class="tabular">{formatCompact(mob.hp)}</td>
            <td class="tabular">{formatNumber(mob.exp)}</td>
            <td class="tabular count">×{mob.count}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  </Section>
{/if}

<!-- A pinned window stops at the monsters; portals, drops, quests, NPCs and routes are in the tab. -->
{#if !glance}
{#if monsterNames.length}
  <Section title="Drops" anchor={anchor("drops")}>
    <MapDrops monsters={monsterNames} />
  </Section>
{/if}

{#if details.quests.length}
  {@const left = notDone(details.quests.map((need) => need.quest.id))}
  <Section title="Quests hunting here" count={details.quests.length} open={!compact && details.quests.length <= 4} anchor={anchor("quests")}>
    {#if left != null}
      <p class="undone">{left === 0 ? "Your quest log shows them all done." : `${left} not done, going by your quest log.`}</p>
    {/if}
    <ul class="quests">
      {#each details.quests as need (need.quest.id)}
        <li>
          <div class="quest-head">
            <Ref kind="quest" id={need.quest.id} name={need.quest.name} class="quest-name" />
            {#if need.quest.levelMin}<span class="lv">Lv {need.quest.levelMin}+</span>{/if}
          </div>
          <div class="quest-sub">
            {need.targets.map((t) => `${t.count} ${t.name}`).join(", ")}
            {#if need.giver}<span class="giver">· from <Ref kind="npc" name={need.giver} /></span>{/if}
          </div>
        </li>
      {/each}
    </ul>
  </Section>
{/if}

{#if details.npcs.length}
  <Section title="NPCs" count={details.npcs.length} open={!compact} anchor={anchor("npcs")}>
    <ul class="npcs">
      {#each details.npcs as npc (npc.id)}
        <li>
          <Ref kind="npc" id={npc.id} name={npc.name} />
          {#if npc.quests.length}
            <span class="gives" use:tip={npc.quests.map((q) => q.name).join("\n")}>
              {questCount(npc.quests.map((q) => q.id))}
            </span>
          {/if}
        </li>
      {/each}
    </ul>
  </Section>
{/if}

{#if details.shops?.length}
  <Section title="Shops" count={details.shops.length} open={!compact} anchor={anchor("shops")}>
    <ul class="shops">
      {#each details.shops as shop (shop.npc)}
        {@const open = openShops.includes(shop.npc)}
        <li>
          <div class="shop-head">
            <button
              class="fold"
              aria-expanded={open}
              aria-label="{open ? 'Hide' : 'Show'} what {shop.npc} sells"
              onclick={() => (openShops = open ? openShops.filter((n) => n !== shop.npc) : [...openShops, shop.npc])}
            >
              <span class="chev" class:open>›</span>
            </button>
            {#if shop.npcId != null}
              <Ref kind="npc" id={shop.npcId} name={shop.npc} />
            {:else}
              <span>{shop.npc}</span>
            {/if}
            <span class="gives">{shop.role ? `${shop.role} · ` : ""}{shop.items.length} item{shop.items.length === 1 ? "" : "s"}</span>
          </div>
          {#if open}<div class="stock"><ShopStock {shop} /></div>{/if}
        </li>
      {/each}
    </ul>
  </Section>
{/if}

<!-- The route planner lives in the Map tab; a looked-up map or a card shows the route from yours. -->
{#if !away && !compact}
  <Section title="Route">
    <RoutePlanner from={details.map} region={details.region} />
  </Section>
{/if}

{/if}

<style>
  .portal-layout {
    display: flex;
    justify-content: flex-end;
    margin: -2px 0 6px;
  }

  .facts {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px 6px;
    margin-top: 8px;
    font-size: var(--text-xs);
  }

  .fact {
    color: var(--muted);
  }

  .fact + .fact::before,
  .fact + .return::before {
    content: "·";
    margin-right: 6px;
    color: var(--faint);
  }

  .fact.town {
    color: var(--good);
  }

  .return {
    color: var(--faint);
  }

  .return :global(.ref) {
    font-size: var(--text-xs);
  }

  .mobs {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--text-sm);
  }

  .mobs th {
    padding: 0 0 3px;
    font-size: var(--text-xs);
    font-weight: 500;
    color: var(--faint);
    text-align: right;
  }

  .mobs th:first-child,
  .mobs td:first-child {
    text-align: left;
  }

  .mobs td {
    padding: 3px 0;
    border-top: 1px solid var(--border);
    text-align: right;
    color: var(--muted);
  }

  .mobs td + td,
  .mobs th + th {
    padding-left: 8px;
  }

  .mobs .mob {
    color: var(--text);
  }


  .count {
    color: var(--text) !important;
  }

  .boss {
    display: inline-grid;
    vertical-align: -1px;
    color: var(--bad);
  }

  .tag {
    margin-left: 3px;
    padding: 0 4px;
    border-radius: var(--radius-sm);
    background: var(--hover);
    font-size: var(--text-xs);
    color: var(--faint);
  }

  .quests,
  .npcs {
    display: grid;
    gap: 6px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .quest-head {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    font-size: var(--text-sm);
  }

  li :global(.ref.quest-name) {
    font-weight: 500;
  }

  .lv {
    flex: none;
    color: var(--faint);
    font-size: var(--text-xs);
  }

  .quest-sub {
    font-size: var(--text-xs);
    color: var(--muted);
  }

  .undone {
    margin: 0 0 4px;
    color: var(--muted);
    font-size: var(--text-xs);
  }

  .giver {
    color: var(--faint);
  }

  .npcs {
    gap: 3px;
  }

  .npcs li {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    font-size: var(--text-sm);
  }


  .gives {
    color: var(--faint);
    font-size: var(--text-xs);
  }

  .shops {
    margin: 0;
    padding: 0;
    list-style: none;
    font-size: var(--text-sm);
  }

  .shop-head {
    display: flex;
    align-items: baseline;
    gap: 6px;
    padding: 2px 0;
  }

  .shop-head .gives {
    margin-left: auto;
  }

  .fold {
    width: 14px;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--faint);
    cursor: pointer;
  }

  .chev {
    display: inline-block;
    transition: transform 0.12s ease;
  }

  .chev.open {
    transform: rotate(90deg);
  }

  .stock {
    padding: 0 0 6px 20px;
  }

  @media (prefers-reduced-motion: reduce) {
    .chev {
      transition: none;
    }
  }

</style>
