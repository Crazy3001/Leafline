<script lang="ts">
  // Training spots for the character's level: maps ranked by the map data's base numbers, or
  // by the EXP per hour the player measured there. No game formulas are modelled.
  import { ChevronRight, Gauge, Minus, Plus } from "@lucide/svelte";
  import { untrack } from "svelte";

  import Card from "./Card.svelte";
  import MapDataPanel from "./map/MapDataPanel.svelte";
  import RouteSteps from "./map/RouteSteps.svelte";
  import { backend } from "$lib/api";
  import { formatCompact, formatDuration } from "$lib/format";
  import { measuredRates, watchHistory } from "$lib/history.svelte";
  import { mapData, watchMapData } from "$lib/mapdata.svelte";
  import { travelKey } from "$lib/settings.svelte";
  import type { MapReading, MapRoute, SessionSummary, TrainingSpot, TrainingSpots } from "$lib/types";

  let {
    level: readLevel,
    map = null,
    session = null,
    plain = false,
  }: {
    /** Read from the HUD or the session; the player can set it when it isn't. */
    level: number | null;
    map?: MapReading | null;
    session?: SessionSummary | null;
    plain?: boolean;
  } = $props();

  type Sort = "exp" | "efficiency" | "measured" | "near";
  const SORTS: { id: Sort; label: string; title: string }[] = [
    { id: "exp", label: "EXP / clear", title: "Base EXP for defeating every monster on the map once" },
    { id: "efficiency", label: "EXP / HP", title: "Base EXP per 1,000 monster HP: how much EXP each hit is worth" },
    { id: "measured", label: "Your rate", title: "EXP per hour you measured on the map, over every saved session and this one" },
    { id: "near", label: "Nearest", title: "Fewest portals from where you are" },
  ];
  /** Maps whose average monster level is this far below or above the character count as near. */
  const BELOW = 8;
  const ABOVE = 3;
  const SHOWN = 8;
  /** Maps scoring within this share of each other are a close call, listed nearest first. */
  const CLOSE_CALL = 0.1;

  const SORT_KEY = "leafline.train.sort";
  const LEVEL_KEY = "leafline.train.level";
  const NEARBY_KEY = "leafline.train.nearbyFirst";
  const ALL_LEVELS_KEY = "leafline.train.allLevels";

  watchMapData();
  const ready = $derived(mapData.status?.ready ?? false);

  let sort = $state<Sort>(stored(SORT_KEY, "exp") as Sort);
  let manualLevel = $state<number | null>(Number(stored(LEVEL_KEY, "")) || null);
  let allLevels = $state(stored(ALL_LEVELS_KEY, "false") === "true");
  let nearbyFirst = $state(stored(NEARBY_KEY, "true") === "true");
  let more = $state(false);
  let data = $state<TrainingSpots | null>(null);
  let open = $state<number | null>(null);
  let route = $state<MapRoute | null>(null);

  const level = $derived(readLevel ?? manualLevel);

  watchHistory();
  // Saved sessions and the current one, so maps can be compared across play sessions.
  const measured = $derived(measuredRates(session));
  const queryKey = $derived(JSON.stringify([map?.name, map?.street, map?.twin, measured, travelKey()]));
  // Each list is a new object: the route follows where it starts from, not the list.
  const fromId = $derived(data?.from?.id);

  // Follows the key alone: every live reading brings new objects with the same content, and
  // asking again for each would rebuild the list.
  $effect(() => {
    const key = queryKey;
    void mapData.status?.dataUpdatedAt;
    if (!ready) {
      data = null;
      return;
    }
    untrack(() => backend.trainingSpots(map?.name ?? null, map?.street ?? null, map?.twin ?? null, measured))
      .then((result) => {
        if (key === queryKey) data = result;
      })
      .catch(() => (data = null));
  });

  $effect(() => {
    const id = open;
    const from = fromId;
    void travelKey();
    route = null;
    if (id == null || from == null) return;
    backend.mapRoute(from, id).then((r) => {
      if (open === id) route = r;
    });
  });

  const efficiency = (s: TrainingSpot) => (s.hpPerClear ? (s.expPerClear * 1000) / s.hpPerClear : 0);
  /** Steps away: fewer with travel, when the player turned it on and it helps. */
  const steps = (s: TrainingSpot) => s.travel?.hops ?? s.hops;
  const near = (s: TrainingSpot) => level == null || (s.averageLevel >= level - BELOW && s.averageLevel <= level + ABOVE);

  const ranked = $derived.by(() => {
    const spots = (data?.spots ?? []).filter((s) => s.reachable && (allLevels || near(s)));
    const by: Record<Sort, (s: TrainingSpot) => number> = {
      exp: (s) => s.expPerClear,
      efficiency,
      measured: (s) => s.measured?.expPerHour ?? -1,
      near: (s) => -(steps(s) ?? 9999),
    };
    const value = by[sort];
    const sorted = [...spots].sort((a, b) => value(b) - value(a) || b.expPerClear - a.expPerClear);
    if (!nearbyFirst || sort === "near") return sorted;
    // Walk down the list in bands of close calls (each within CLOSE_CALL of the band's best)
    // and order each band by portals, so a slightly smaller map next door isn't buried.
    const out: TrainingSpot[] = [];
    let band: TrainingSpot[] = [];
    const flush = () => out.push(...band.sort((a, b) => (steps(a) ?? Infinity) - (steps(b) ?? Infinity)));
    for (const spot of sorted) {
      if (band.length && value(spot) < value(band[0]) - Math.abs(value(band[0])) * CLOSE_CALL) {
        flush();
        band = [];
      }
      band.push(spot);
    }
    flush();
    return out;
  });
  const shown = $derived(ranked.slice(0, more ? 30 : SHOWN));
  const anyMeasured = $derived(data?.spots.some((s) => s.measured) ?? false);

  function stored(key: string, fallback: string) {
    try {
      return localStorage.getItem(key) ?? fallback;
    } catch {
      return fallback;
    }
  }

  function remember(key: string, value: string) {
    try {
      localStorage.setItem(key, value);
    } catch {
      // A convenience only.
    }
  }

  function pickSort(next: Sort) {
    sort = next;
    remember(SORT_KEY, next);
  }

  function setLevel(next: number) {
    manualLevel = Math.max(1, Math.min(200, next));
    remember(LEVEL_KEY, String(manualLevel));
  }

  function levelTone(spot: TrainingSpot) {
    if (level == null) return "";
    const delta = spot.averageLevel - level;
    return delta > ABOVE ? "above" : delta < -BELOW ? "below" : "";
  }

  function metric(spot: TrainingSpot) {
    switch (sort) {
      case "efficiency":
        return { value: efficiency(spot).toFixed(0), unit: "EXP / 1k HP" };
      case "measured":
        return spot.measured
          ? { value: formatCompact(spot.measured.expPerHour), unit: "your EXP / hr" }
          : { value: "—", unit: "not measured" };
      case "near":
        if (spot.travel) return { value: String(spot.travel.hops), unit: "with travel" };
        return spot.hops == null
          ? { value: "—", unit: "no route" }
          : { value: String(spot.hops), unit: spot.hops === 1 ? "portal" : "portals" };
      default:
        return { value: formatCompact(spot.expPerClear), unit: "EXP / clear" };
    }
  }
</script>

<Card title="Training spots" {plain}>
  {#snippet aside()}
    {#if readLevel != null}
      <span class="tabular">For Lv {readLevel}</span>
    {:else}
      <span class="stepper" title="The HUD isn't in view: set your level">
        Lv
        <button aria-label="Lower level" onclick={() => setLevel((manualLevel ?? 2) - 1)}><Minus size={10} /></button>
        <input
          type="number"
          min="1"
          max="200"
          aria-label="Character level"
          value={manualLevel ?? ""}
          placeholder="?"
          onchange={(e) => setLevel(Number(e.currentTarget.value) || 1)}
        />
        <button aria-label="Raise level" onclick={() => setLevel((manualLevel ?? 0) + 1)}><Plus size={10} /></button>
      </span>
    {/if}
  {/snippet}

  {#if !ready}
    {#if mapData.status}<MapDataPanel compact />{/if}
  {:else if !data}
    <p class="empty">Loading maps…</p>
  {:else}
    <div class="sorts" role="radiogroup" aria-label="Rank by">
      {#each SORTS as option (option.id)}
        <button
          role="radio"
          aria-checked={sort === option.id}
          title={option.title}
          disabled={option.id === "measured" && !anyMeasured}
          onclick={() => pickSort(option.id)}>{option.label}</button
        >
      {/each}
    </div>

    {#if level == null && !allLevels}
      <p class="empty">Showing every level. Set your level above to see maps that suit it.</p>
    {/if}

    <ol class="spots">
      {#each shown as spot, i (spot.map.id)}
        {@const m = metric(spot)}
        {@const expanded = open === spot.map.id}
        <li class:expanded>
          <button class="row" aria-expanded={expanded} onclick={() => (open = expanded ? null : spot.map.id)}>
            <span class="rank tabular">{i + 1}</span>
            <span class="names">
              <span class="name">{spot.map.name}</span>
              <span class="meta">
                <span class="lv {levelTone(spot)}">Lv {spot.levelRange[0]}{spot.levelRange[1] !== spot.levelRange[0] ? `–${spot.levelRange[1]}` : ""}</span>
                · {spot.mobTotal} mobs{#if spot.hops != null && (sort !== "near" || spot.travel)}{" · "}{spot.hops === 0
                    ? "here"
                    : `${spot.hops} portal${spot.hops === 1 ? "" : "s"}`}{/if}{#if spot.travel && sort !== "near"}<span
                    class="shortcut"
                    title="With the travel you turned on in Settings, via {spot.travel.via.name}"
                    >{" "}({spot.travel.hops} with travel)</span
                  >{/if}
              </span>
            </span>
            <span class="metric">
              <span class="value tabular">{m.value}</span>
              <span class="unit">{m.unit}</span>
            </span>
            <span class="chevron" class:open={expanded}><ChevronRight size={11} /></span>
          </button>

          {#if expanded}
            <div class="detail">
              <table class="mobs">
                <thead>
                  <tr><th>Monster</th><th>Lv</th><th>×</th><th>EXP</th><th>HP</th></tr>
                </thead>
                <tbody>
                  {#each spot.mobs as mob (mob.id)}
                    <tr>
                      <td><button class="mob-link" title="Show it in the Lookup tab" onclick={() => backend.showMonster(mob.id, null)}>{mob.name}</button>{mob.boss ? " (boss)" : ""}</td>
                      <td class="tabular">{mob.level}</td>
                      <td class="tabular">{mob.count}</td>
                      <td class="tabular">{formatCompact(mob.exp)}</td>
                      <td class="tabular">{formatCompact(mob.hp)}</td>
                    </tr>
                  {/each}
                </tbody>
              </table>
              <p class="facts tabular">
                {formatCompact(spot.expPerClear)} EXP / clear · {efficiency(spot).toFixed(0)} EXP per 1k HP
                {#if spot.measured}
                  <br /><span class="yours"
                    ><Gauge size={11} /> You averaged {formatCompact(spot.measured.expPerHour)} EXP / hr over {formatDuration(
                      spot.measured.activeSeconds,
                    )}</span
                  >
                {/if}
              </p>
              {#if data.from && route}
                <RouteSteps {route} />
              {:else if !data.from}
                <p class="note">Routes appear when the minimap is in view.</p>
              {:else if spot.hops == null}
                <p class="note">No route through portals is known from {data.from.name}.</p>
              {/if}
            </div>
          {/if}
        </li>
      {:else}
        {#if level != null}
          <li class="empty">No maps with monsters near Lv {level}.</li>
        {/if}
      {/each}
    </ol>

    <div class="footer">
      <span class="checks">
        <label class="check">
          <input
            type="checkbox"
            checked={allLevels}
            onchange={(e) => {
              allLevels = e.currentTarget.checked;
              remember(ALL_LEVELS_KEY, String(allLevels));
            }}
          />
          Every level
        </label>
        {#if sort !== "near"}
          <label class="check" title="When maps score within 10% of each other, list the one fewer portals away first">
            <input
              type="checkbox"
              checked={nearbyFirst}
              onchange={(e) => {
                nearbyFirst = e.currentTarget.checked;
                remember(NEARBY_KEY, String(nearbyFirst));
              }}
            />
            Nearby first
          </label>
        {/if}
      </span>
      {#if ranked.length > SHOWN}
        <button class="link" onclick={() => (more = !more)}>{more ? "Fewer" : `Show ${Math.min(30, ranked.length) - SHOWN} more`}</button>
      {/if}
    </div>
    <p class="hedge">
      Base numbers from OSMS map data, before EXP events, party bonus or level differences. Your rate is what Leafline
      measured in your sessions.{#if nearbyFirst && sort !== "near" && data.from}{" "}Maps within 10% of each other are
        listed nearest first.{/if}
      Maps entered only through an NPC, such as party quests, are left out.
    </p>
  {/if}
</Card>

<style>
  .stepper {
    display: inline-flex;
    align-items: center;
    gap: 3px;
  }

  .stepper button {
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--muted);
    cursor: pointer;
  }

  .stepper input {
    width: 34px;
    padding: 1px 2px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--text);
    font: inherit;
    text-align: center;
    -moz-appearance: textfield;
    appearance: textfield;
  }

  .stepper input::-webkit-inner-spin-button {
    display: none;
  }

  .sorts {
    display: flex;
    gap: 2px;
    margin-bottom: 8px;
    padding: 2px;
    border-radius: var(--radius-sm);
    background: rgba(255, 255, 255, 0.04);
  }

  .sorts button {
    flex: 1;
    padding: 3px 4px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--muted);
    font-size: 11px;
    white-space: nowrap;
    cursor: pointer;
  }

  .sorts button[aria-checked="true"] {
    background: var(--surface-3);
    color: var(--text);
  }

  .sorts button:disabled {
    color: var(--faint);
    cursor: default;
  }

  .sorts button:focus-visible,
  .row:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }

  .spots {
    display: grid;
    gap: 1px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 5px 4px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    text-align: left;
    cursor: pointer;
  }

  .row:hover,
  .expanded .row {
    background: rgba(255, 255, 255, 0.04);
  }

  .rank {
    flex: none;
    width: 14px;
    color: var(--faint);
    font-size: 10.5px;
    text-align: right;
  }

  .names {
    display: grid;
    flex: 1;
    min-width: 0;
  }

  .name,
  .meta {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .name {
    font-weight: 500;
  }

  .meta {
    color: var(--faint);
    font-size: 11px;
  }

  .lv {
    color: var(--muted);
  }

  .shortcut {
    color: var(--travel);
  }

  .lv.above {
    color: var(--warn);
  }

  .lv.below {
    color: var(--faint);
  }

  .metric {
    display: grid;
    flex: none;
    justify-items: end;
  }

  .value {
    font-weight: 600;
  }

  .unit {
    color: var(--faint);
    font-size: 10px;
  }

  .chevron {
    display: inline-grid;
    flex: none;
    color: var(--faint);
    transition: transform 0.12s ease;
  }

  .chevron.open {
    transform: rotate(90deg);
  }

  .detail {
    margin: 2px 0 8px 26px;
  }

  .mobs {
    width: 100%;
    border-collapse: collapse;
    font-size: 11.5px;
  }

  .mobs th {
    padding: 2px 0;
    color: var(--faint);
    font-size: 10.5px;
    font-weight: 500;
    text-align: right;
  }

  .mobs td {
    padding: 2px 0;
    color: var(--muted);
    text-align: right;
  }

  .mobs th:first-child,
  .mobs td:first-child {
    text-align: left;
    color: var(--text);
  }

  .mob-link {
    padding: 0;
    border: 0;
    background: transparent;
    font-size: inherit;
    text-align: left;
    cursor: pointer;
  }

  .mob-link:hover,
  .mob-link:focus-visible {
    color: var(--map);
    text-decoration: underline;
    outline: none;
  }

  .mobs th:first-child {
    color: var(--faint);
  }

  .facts {
    margin: 6px 0 0;
    color: var(--muted);
    font-size: 11.5px;
  }

  .yours {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: var(--good);
  }

  .footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: 8px;
    font-size: 11.5px;
    color: var(--muted);
  }

  .checks {
    display: inline-flex;
    gap: 10px;
  }

  .check {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    cursor: pointer;
  }

  .check input {
    margin: 0;
    accent-color: var(--accent);
  }

  .link {
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--muted);
    font-size: inherit;
    cursor: pointer;
  }

  .link:hover {
    color: var(--text);
    text-decoration: underline;
  }

  .hedge,
  .note {
    margin: 8px 0 0;
    color: var(--faint);
    font-size: 10.5px;
  }

  .empty {
    margin: 0 0 6px;
    padding: 6px 0;
    color: var(--faint);
    font-size: 12px;
  }
</style>
