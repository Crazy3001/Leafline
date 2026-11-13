<script lang="ts">
  // Everything the cached data says about one map. No map images or coordinates are shown.
  import Ref from "$lib/components/Ref.svelte";
  import { ExternalLink, Skull } from "@lucide/svelte";

  import { backend } from "$lib/api";
  import { formatCompact, formatNumber, monsterLabel } from "$lib/format";
  import { OSMS_CREDIT, OSMS_URL } from "$lib/mapdata.svelte";
  import { showNpcNamed } from "$lib/places.svelte";
  import type { MapDetails, MapRef } from "$lib/types";
  import ConnectionCompass from "./ConnectionCompass.svelte";
  import MapPicture from "./MapPicture.svelte";
  import { display } from "$lib/settings.svelte";
  import MapDrops from "./MapDrops.svelte";
  import ShopStock from "../item/ShopStock.svelte";
  import RouteFromHere from "./RouteFromHere.svelte";
  import RoutePlanner from "./RoutePlanner.svelte";
  import Section from "./Section.svelte";

  let {
    details,
    current = null,
    variant = "full",
    onnavigate,
  }: {
    details: MapDetails;
    /** The map the player is on, when known. */
    current?: MapRef | null;
    /** `glance` keeps the facts and the monsters, for a pinned window. */
    variant?: "full" | "compact" | "glance";
    onnavigate: (id: number) => void;
  } = $props();

  const compact = $derived(variant !== "full");
  const glance = $derived(variant === "glance");
  /** The player's map, while they look at another. */
  const away = $derived(current && current.id !== details.map.id ? current : null);
  const placed = $derived(details.exits.some((e) => e.side));
  /** Shops opened to show what they sell, by NPC name. */
  let openShops = $state<string[]>([]);

  const facts = $derived(
    [
      details.isTown ? "Town" : null,
      details.levelRange
        ? details.levelRange[0] === details.levelRange[1]
          ? `Lv ${details.levelRange[0]}`
          : `Lv ${details.levelRange[0]}–${details.levelRange[1]}`
        : null,
      details.region,
    ].filter((f): f is string => !!f),
  );
  const monsterNames = $derived([...new Set(details.mobs.map((m) => m.name))]);
  /** The desk view has room for the map's picture; over the game it waits to be opened. */
  const desk = typeof location !== "undefined" && location.pathname.startsWith("/desk");
  /** The monster under the pointer in the list, whose spawn points stand out on the picture. */
  let hovered = $state<string | null>(null);
</script>

{#if facts.length || details.returnMap}
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
  <Section title="Map" memory="map-picture-{desk ? 'desk' : 'overlay'}" open={desk}>
    <MapPicture mapId={details.map.id} highlight={hovered} />
  </Section>
{/if}

{#if away}
  <Section title="Route from your map">
    <RouteFromHere from={away} to={details.map} />
  </Section>
{/if}

{#if !glance}
  <Section title="Exits" count={details.exits.length}>
    {#snippet aside()}
      {#if placed}<span>rough positions</span>{/if}
    {/snippet}
    <ConnectionCompass exits={details.exits} map={details.map} {current} {onnavigate} />
  </Section>
{/if}

{#if details.mobs.length}
  <Section title="Monsters" count={details.mobTotal}>
    {#snippet aside()}<span title="Base EXP for killing every monster once, before any modifiers"
        >{formatCompact(details.expPerClear)} EXP / clear</span
      >{/snippet}
    <table class="mobs">
      <thead>
        <tr><th>Monster</th><th>Lv</th><th>HP</th><th>EXP</th><th title="How many spawn">#</th></tr>
      </thead>
      <tbody>
        {#each details.mobs as mob (mob.id)}
          <tr onmouseenter={() => (hovered = mob.name)} onmouseleave={() => (hovered = null)}>
            <td class="mob">
              <Ref kind="mob" id={mob.id} name={monsterLabel(mob.name)} />
              {#if mob.boss}<span class="boss" title="Boss"><Skull size={11} /></span>{/if}
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

<!-- A pinned window stops at the monsters; exits, drops, quests, NPCs and routes are in the tab. -->
{#if !glance}
{#if monsterNames.length}
  <Section title="Drops">
    <MapDrops monsters={monsterNames} />
  </Section>
{/if}

{#if details.quests.length}
  <Section title="Quests hunting here" count={details.quests.length} open={!compact && details.quests.length <= 4}>
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
  <Section title="NPCs" count={details.npcs.length} open={!compact}>
    <ul class="npcs">
      {#each details.npcs as npc (npc.id)}
        <li>
          <Ref kind="npc" id={npc.id} name={npc.name} />
          {#if npc.quests.length}
            <span class="gives" title={npc.quests.map((q) => q.name).join("\n")}>
              {npc.quests.length} quest{npc.quests.length === 1 ? "" : "s"}
            </span>
          {/if}
        </li>
      {/each}
    </ul>
  </Section>
{/if}

{#if details.shops?.length}
  <Section title="Shops" count={details.shops.length} open={!compact}>
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

{#if !away}
  <Section title="Route" open={!compact}>
    <RoutePlanner from={details.map} region={details.region} />
  </Section>
{/if}

<div class="links">
  {#each details.links as link}
    <button class="out" onclick={() => backend.openLink(link.url)}>
      {link.label}<ExternalLink size={11} />
    </button>
  {/each}
</div>
{/if}
<button class="credit" onclick={() => backend.openLink(OSMS_URL)}>{OSMS_CREDIT}</button>

<style>
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
    background: rgba(255, 255, 255, 0.06);
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

  .links {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-top: 12px;
  }

  .out {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 24px;
    padding: 0 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--muted);
    font-size: var(--text-xs);
    cursor: pointer;
  }

  .out:hover {
    border-color: var(--border-strong);
    color: var(--text);
  }

  .credit {
    display: block;
    margin-top: 8px;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--faint);
    font-size: var(--text-xs);
    line-height: 1.4;
    text-align: left;
    cursor: pointer;
  }

  .credit:hover {
    color: var(--muted);
    text-decoration: underline;
  }
</style>
