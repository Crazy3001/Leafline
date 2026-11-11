<script lang="ts">
  // The training log's Records view: every map the player trained on, and under each the ways
  // they trained there (level, party size and job), with EXP and mesos per hour. Built from the
  // saved sessions' lines; everything stays on this PC.
  import { formatCompact, formatDuration, partyLabel } from "$lib/format";
  import { bandRange, levelBands, mapRecords, mapStats, shortDay, SHORT_SECONDS, type LineFilter, type RecordRates } from "$lib/training";
  import type { SavedSession } from "$lib/types";
  import Segmented from "$lib/ui/Segmented.svelte";

  let { sessions }: { sessions: SavedSession[] } = $props();

  type Sort = "exp" | "mesos" | "time";

  let band = $state("any");
  let party = $state<"solo" | "party" | "any">("any");
  let job = $state("");
  let sort = $state<Sort>("exp");
  /** Maps with their rows folded away. */
  let folded = $state<Set<string>>(new Set());

  const bands = $derived(levelBands(sessions));
  const jobs = $derived([...new Set(sessions.flatMap((s) => s.lines.map((l) => l.job)).filter((j): j is string => !!j))].sort());
  const filter = $derived<LineFilter>({
    levels: bandRange(band),
    party,
    job: job || null,
  });
  const records = $derived(mapRecords(sessions, filter, sort));
  /** Each map's best single run under the same filters. */
  const bests = $derived(new Map(mapStats(sessions, filter).map((m) => [m.map, m])));
  /** Show the after-potions column once any session has potion counts. */
  const withPotions = $derived(sessions.some((s) => s.potionCost > 0));
  const columns = $derived(withPotions ? 7 : 6);

  const exp = (r: RecordRates) =>
    r.expPerHour != null ? formatCompact(r.expPerHour) : r.percentPerHour != null ? `${r.percentPerHour.toFixed(1)}%` : "—";
  const mesos = (r: RecordRates) => (r.mesosPerHour != null ? formatCompact(r.mesosPerHour) : "—");
  const net = (r: RecordRates) => (r.netPerHour != null ? formatCompact(r.netPerHour) : "—");
  const mesosTitle = (r: RecordRates) =>
    r.mesosPerHour != null && r.mesosShare < 0.9
      ? `Over ${formatDuration(r.activeSeconds * r.mesosShare)} of ${formatDuration(r.activeSeconds)}: the mesos weren't in view for the rest.`
      : undefined;

  function toggle(map: string) {
    const next = new Set(folded);
    if (next.has(map)) next.delete(map);
    else next.add(map);
    folded = next;
  }
</script>

<div class="filters">
  {#if bands.length > 1}
    <span class="field">
      Level
      <Segmented label="Your level then" value={band} options={[...bands, { id: "any", label: "Any" }]} onchange={(b) => (band = b)} />
    </span>
  {/if}
  <span class="field">
    Party
    <Segmented
      label="Party"
      value={party}
      options={[
        { id: "solo", label: "Solo" },
        { id: "party", label: "In a party" },
        { id: "any", label: "Either" },
      ]}
      onchange={(p) => (party = p)}
    />
  </span>
  {#if jobs.length > 1}
    <label class="field">
      Job
      <select bind:value={job}>
        <option value="">Any job</option>
        {#each jobs as name (name)}<option value={name}>{name}</option>{/each}
      </select>
    </label>
  {/if}
  <span class="grow"></span>
  <span class="field">
    Order by
    <Segmented
      size="sm"
      label="Order by"
      value={sort}
      options={[
        { id: "exp", label: "EXP / hr" },
        { id: "mesos", label: "Mesos / hr" },
        { id: "time", label: "Time" },
      ]}
      onchange={(s) => (sort = s)}
    />
  </span>
</div>

{#if !records.length}
  <p class="quiet">No play matches these filters.</p>
{:else}
  <section class="panel">
    <div class="scroll">
      <table>
        <thead>
          <tr>
            <th>Map, then level · party · job</th>
            <th>Sessions</th>
            <th>Played</th>
            <th>EXP / hr</th>
            <th>Mesos / hr</th>
            {#if withPotions}<th>After potions</th>{/if}
            <th>Best run</th>
          </tr>
        </thead>
        {#each records as m (m.map)}
          {@const open = !folded.has(m.map)}
          <tbody>
            <tr class="map" class:short={m.activeSeconds < SHORT_SECONDS}>
              <td>
                <button class="fold" aria-expanded={open} onclick={() => toggle(m.map)}>
                  <span class="name">{m.name}</span>
                  {#if m.street}<span class="street">{m.street}</span>{/if}
                </button>
              </td>
              <td>{m.sessions}</td>
              <td>{formatDuration(m.activeSeconds)}</td>
              <td class="strong">{exp(m)}</td>
              <td class="strong" title={mesosTitle(m)}>{mesos(m)}{m.mesosPerHour != null && m.mesosShare < 0.9 ? "*" : ""}</td>
              {#if withPotions}<td class="strong">{net(m)}</td>{/if}
              <td>
                {#if bests.get(m.map)?.best}
                  {@const best = bests.get(m.map)!.best!}
                  <span title="{shortDay(best.at)}{best.level != null ? `, Lv ${best.level}` : ''}">{formatCompact(best.expPerHour)}</span>
                {:else}—{/if}
              </td>
            </tr>
            {#if open && m.rows.length > 1}
              {#each m.rows as row (row.key)}
                <tr class="sub" class:short={row.activeSeconds < SHORT_SECONDS}>
                  <td>
                    {[row.level != null ? `Lv ${row.level}` : "Level not read", partyLabel(row.party) ?? "Party not read", row.job ?? "Job not read"].join(" · ")}
                  </td>
                  <td>{row.sessions}</td>
                  <td>{formatDuration(row.activeSeconds)}</td>
                  <td>{exp(row)}</td>
                  <td title={mesosTitle(row)}>{mesos(row)}{row.mesosPerHour != null && row.mesosShare < 0.9 ? "*" : ""}</td>
                  {#if withPotions}<td>{net(row)}</td>{/if}
                  <td></td>
                </tr>
              {/each}
            {:else if open}
              {@const row = m.rows[0]}
              <tr class="sub">
                <td colspan={columns}>
                  {[row.level != null ? `Lv ${row.level}` : "Level not read", partyLabel(row.party) ?? "Party not read", row.job ?? "Job not read"].join(" · ")}
                </td>
              </tr>
            {/if}
          </tbody>
        {/each}
      </table>
    </div>
    <p class="caption">
      Best run is the best single session of 10 minutes or more on the map, in EXP / hr.
      Faded rows rest on under 30 minutes of play. * Mesos / hr covers only part of the time, while the mesos were in view.
      {#if withPotions}After potions shares each session's potion cost across its maps by time played.{/if}
    </p>
  </section>
{/if}

<style>
  .filters {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-4);
  }

  .field {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-sm);
    color: var(--muted);
  }

  .grow {
    flex: 1;
  }

  select {
    height: var(--control-h);
    padding: 0 6px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
  }

  .panel {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: var(--space-3);
    border: 1px solid var(--border);
    background: var(--surface);
  }

  .scroll {
    overflow-x: auto;
  }

  table {
    width: 100%;
    min-width: 560px;
    border-collapse: collapse;
    font-size: var(--text-sm);
    font-variant-numeric: tabular-nums;
  }

  th {
    padding: 0 0 6px;
    font-size: var(--text-xs);
    font-weight: 400;
    color: var(--faint);
    text-align: right;
  }

  td {
    padding: 6px 0;
    text-align: right;
  }

  th:first-child,
  td:first-child {
    text-align: left;
  }

  tr.map td {
    padding-top: 9px;
    border-top: 1px solid var(--border-strong);
  }

  tbody:first-of-type tr.map td {
    border-top: 0;
  }

  tr.sub td {
    padding: 3px 0;
    color: var(--muted);
  }

  tr.sub td:first-child {
    padding-left: 14px;
    font-size: var(--text-xs);
  }

  tr.short td {
    opacity: 0.6;
  }

  .strong {
    font-weight: 600;
  }

  .fold {
    display: inline-flex;
    align-items: baseline;
    gap: 8px;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .fold:hover .name {
    color: var(--accent-strong);
  }

  .name {
    font-weight: 600;
  }

  .street {
    font-size: var(--text-xs);
    color: var(--faint);
  }

  .caption {
    margin: 0;
    font-size: var(--text-xs);
    color: var(--faint);
  }

  .quiet {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--faint);
  }
</style>
