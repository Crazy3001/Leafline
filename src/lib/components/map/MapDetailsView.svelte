<script lang="ts">
  // Everything the cached data says about one map. No map images or coordinates are shown.
  import { ExternalLink, Skull } from "@lucide/svelte";

  import { backend } from "$lib/api";
  import { formatCompact, formatNumber } from "$lib/format";
  import { OSMS_CREDIT, OSMS_URL } from "$lib/mapdata.svelte";
  import type { MapDetails, MapRef } from "$lib/types";
  import ConnectionCompass from "./ConnectionCompass.svelte";
  import MapDrops from "./MapDrops.svelte";
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
    variant?: "full" | "compact";
    onnavigate: (id: number) => void;
  } = $props();

  const compact = $derived(variant !== "full");
  /** The player's map, while they look at another. */
  const away = $derived(current && current.id !== details.map.id ? current : null);
  const placed = $derived(details.exits.some((e) => e.side));

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
</script>

{#if facts.length || details.returnMap}
  <div class="facts">
    {#each facts as fact, i}<span class="fact" class:town={i === 0 && details.isTown}>{fact}</span>{/each}
    {#if details.returnMap}
      <span class="return">
        Returns to
        <button class="inline" onclick={() => onnavigate(details.returnMap!.id)}>{details.returnMap.name}</button>
      </span>
    {/if}
  </div>
{/if}

{#if away}
  <Section title="Route from your map">
    <RouteFromHere from={away} to={details.map} {onnavigate} />
  </Section>
{/if}

<Section title="Exits" count={details.exits.length}>
  {#snippet aside()}
    {#if placed}<span title="Roughly which part of the map each portal is in, from the map data">rough positions</span>{/if}
  {/snippet}
  <ConnectionCompass exits={details.exits} map={details.map} {current} {onnavigate} />
</Section>

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
          <tr>
            <td class="mob">
              <button class="inline mob-link" title="Open in Lookup" onclick={() => backend.showMonster(mob.id, null)}>{mob.name}</button>
              {#if mob.boss}<span class="boss" title="Boss"><Skull size={11} /></span>{/if}
              {#if mob.undead}<span class="tag" title="Undead">undead</span>{/if}
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
            <span class="quest-name">{need.quest.name}</span>
            {#if need.quest.levelMin}<span class="lv">Lv {need.quest.levelMin}+</span>{/if}
          </div>
          <div class="quest-sub">
            {need.targets.map((t) => `${t.count} ${t.name}`).join(", ")}
            {#if need.giver}<span class="giver">· from {need.giver}</span>{/if}
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
          <button class="inline npc" title="Open in Lookup" onclick={() => backend.showPlace("npcs", npc.id)}>{npc.name}</button>
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

{#if !away}
  <Section title="Route" open={!compact}>
    <RoutePlanner from={details.map} region={details.region} {onnavigate} />
  </Section>
{/if}

<div class="links">
  {#each details.links as link}
    <button class="out" onclick={() => backend.openLink(link.url)} title={link.url}>
      {link.label}<ExternalLink size={11} />
    </button>
  {/each}
</div>
<button class="credit" onclick={() => backend.openLink(OSMS_URL)}>{OSMS_CREDIT}</button>

<style>
  .facts {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px 6px;
    margin-top: 8px;
    font-size: 11px;
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

  .inline {
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--muted);
    font-size: 11px;
    cursor: pointer;
  }

  .inline:hover {
    color: var(--map);
    text-decoration: underline;
  }

  .mobs {
    width: 100%;
    border-collapse: collapse;
    font-size: 11.5px;
  }

  .mobs th {
    padding: 0 0 3px;
    font-size: 10px;
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

  .mob-link {
    color: var(--text);
    font-size: 11.5px;
    text-align: left;
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
    font-size: 9.5px;
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
    font-size: 12px;
  }

  .quest-name {
    font-weight: 500;
  }

  .lv {
    flex: none;
    color: var(--faint);
    font-size: 11px;
  }

  .quest-sub {
    font-size: 11px;
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
    font-size: 12px;
  }

  .npc {
    color: var(--text);
    font-size: 12px;
    text-align: left;
  }

  .gives {
    color: var(--faint);
    font-size: 11px;
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
    font-size: 11px;
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
    font-size: 10px;
    line-height: 1.4;
    text-align: left;
    cursor: pointer;
  }

  .credit:hover {
    color: var(--muted);
    text-decoration: underline;
  }
</style>
