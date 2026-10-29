<script lang="ts">
  // Everything the cached data says about one map. No map images or coordinates are shown.
  import { ExternalLink, Skull } from "@lucide/svelte";

  import { backend } from "$lib/api";
  import { formatCompact, formatNumber } from "$lib/format";
  import { OSMS_CREDIT, OSMS_URL } from "$lib/mapdata.svelte";
  import type { MapDetails } from "$lib/types";
  import ConnectionCompass from "./ConnectionCompass.svelte";
  import MapDrops from "./MapDrops.svelte";
  import RoutePlanner from "./RoutePlanner.svelte";
  import Section from "./Section.svelte";

  let {
    details,
    variant = "full",
    onnavigate,
  }: { details: MapDetails; variant?: "full" | "compact" | "pin"; onnavigate: (id: number) => void } = $props();

  const compact = $derived(variant !== "full");
  /** A pinned window keeps to the essentials: where you can go and what lives here. */
  const pin = $derived(variant === "pin");

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

<Section title="Exits" count={details.exits.length}>
  <ConnectionCompass exits={details.exits} {onnavigate} />
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
              {mob.name}
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

{#if details.quests.length && !pin}
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

{#if details.npcs.length && !pin}
  <Section title="NPCs" count={details.npcs.length} open={!compact}>
    <ul class="npcs">
      {#each details.npcs as npc (npc.id)}
        <li>
          <span class="npc">{npc.name}</span>
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

{#if !pin}
  <Section title="Route" open={!compact}>
    <RoutePlanner from={details.map} {onnavigate} />
  </Section>
{/if}

{#if monsterNames.length && !pin}
  <Section title="Drops" open={false}>
    <MapDrops monsters={monsterNames} />
  </Section>
{/if}

{#if !pin}<div class="links">
  {#each details.links as link}
    <button class="out" onclick={() => backend.openLink(link.url)} title={link.url}>
      {link.label}<ExternalLink size={11} />
    </button>
  {/each}
</div>{/if}
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
    padding: 1px 7px;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.05);
    color: var(--muted);
  }

  .fact.town {
    background: var(--good-soft);
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
    border-radius: 3px;
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
    border-radius: 999px;
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
