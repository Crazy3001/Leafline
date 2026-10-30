<script lang="ts">
  import Card from "./Card.svelte";
  import { formatCompact, formatDuration, formatNumber, splitMapLabel } from "$lib/format";
  import type { SessionSummary } from "$lib/types";

  let { session, compact = false }: { session: SessionSummary; compact?: boolean } = $props();

  const signed = (value: number) => (value > 0 ? `+${formatNumber(value)}` : formatNumber(value));
  const maps = $derived(session.maps.filter((m) => m.activeSeconds > 0 || m.expGained !== 0 || m.mesosGained !== 0));
  const dropped = $derived(session.rejected.exp + session.rejected.mesos);
</script>

<Card title="Session">
  {#snippet aside()}
    {#if session.paused}
      <span class="state paused" title="No EXP for a while, or the map changed: the clock waits for the next gain">
        Paused
      </span>
    {:else if session.activeSeconds > 0}
      <span class="state running">Tracking</span>
    {/if}
  {/snippet}

  <dl class="totals" class:compact>
    <div>
      <dt>EXP gained</dt>
      <dd class="tabular">{signed(session.expGained)}</dd>
      <!-- In the overlay the character card above already shows the rate. -->
      {#if session.percentPerHour != null && !compact}<dd class="sub tabular">{session.percentPerHour.toFixed(1)}% / hr</dd>{/if}
    </div>
    <div>
      <dt>Mesos gained</dt>
      <dd class="tabular">{signed(session.mesosGained)}</dd>
      {#if session.mesos != null}<dd class="sub tabular">{formatNumber(session.mesos)} now</dd>{/if}
    </div>
    <div>
      <dt>Active</dt>
      <dd class="tabular">{formatDuration(session.activeSeconds)}</dd>
      <dd class="sub tabular">of {formatDuration(session.elapsedSeconds)}</dd>
    </div>
    {#if !compact}
      <div>
        <dt>Level-ups</dt>
        <dd class="tabular">{session.levelUps}</dd>
        {#if session.expNeeded != null}<dd
            class="sub tabular"
            title="EXP this level needs in all, measured from the HUD's points and percent"
          >
            Level needs {formatCompact(session.expNeeded)}
          </dd>{/if}
      </div>
    {/if}
  </dl>

  {#if maps.length}
    <table class="maps">
      <thead>
        <tr><th>Map</th><th>Time</th><th>EXP / hr</th><th>Mesos / hr</th></tr>
      </thead>
      <tbody>
        {#each maps as row (row.map)}
          {@const label = row.map ? splitMapLabel(row.map) : null}
          <tr class:current={row.map != null && row.map === session.map}>
            <th scope="row">
              {#if label}
                <span class="map-name">{label.name}</span>
                {#if label.street && !compact}<span class="map-street">{label.street}</span>{/if}
              {:else}
                <span class="map-name unplaced" title="Gains between two readings on different maps, or before any map was read">
                  Between maps
                </span>
              {/if}
            </th>
            <td class="tabular">{formatDuration(row.activeSeconds)}</td>
            <td class="tabular">{formatCompact(row.expPerHour)}</td>
            <td class="tabular">{formatCompact(row.mesosPerHour)}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}

  {#if !compact && (session.mesosExcluded !== 0 || dropped > 0)}
    <ul class="notes">
      {#if session.mesosExcluded !== 0}
        <li>{signed(session.mesosExcluded)} mesos changed hands in storage or trade windows and are left out.</li>
      {/if}
      {#if dropped > 0}
        <li>
          Dropped as misreads: {session.rejected.exp} EXP and {session.rejected.mesos} meso reading{session.rejected
            .mesos === 1
            ? ""
            : "s"}.
        </li>
      {/if}
    </ul>
  {/if}
</Card>

<style>
  .state {
    padding: 1px 7px;
    border-radius: 999px;
    font-size: 10.5px;
    font-weight: 600;
  }

  .running {
    color: var(--good);
    background: var(--good-soft);
  }

  .paused {
    color: var(--warn);
    background: var(--warn-soft);
  }

  .totals {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 6px;
    margin: 0;
  }

  .totals.compact {
    grid-template-columns: repeat(3, 1fr);
  }

  .totals div {
    padding: 6px 8px;
    border-radius: var(--radius-sm);
    background: rgba(255, 255, 255, 0.03);
  }

  dt {
    font-size: 10.5px;
    color: var(--faint);
  }

  dd {
    margin: 0;
    font-weight: 600;
  }

  dd.sub {
    font-size: 11px;
    font-weight: 400;
    color: var(--muted);
  }

  .maps {
    width: 100%;
    margin-top: 10px;
    border-collapse: collapse;
    font-size: 12px;
  }

  .maps thead th {
    padding: 0 0 4px;
    font-size: 10.5px;
    font-weight: 500;
    color: var(--faint);
    text-align: right;
  }

  .maps thead th:first-child {
    text-align: left;
  }

  .maps tbody tr {
    border-top: 1px solid var(--border);
  }

  .maps tbody th {
    padding: 5px 8px 5px 0;
    font-weight: 400;
    text-align: left;
  }

  .maps td {
    padding: 5px 0 5px 8px;
    color: var(--muted);
    text-align: right;
    white-space: nowrap;
  }

  .map-name {
    display: block;
    color: var(--text);
  }

  .map-name.unplaced {
    color: var(--muted);
    font-style: italic;
  }

  .map-street {
    display: block;
    color: var(--faint);
    font-size: 11px;
  }

  .current .map-name::before {
    content: "";
    display: inline-block;
    width: 6px;
    height: 6px;
    margin: 0 6px 1px 0;
    border-radius: 50%;
    background: var(--map);
  }

  .notes {
    display: grid;
    gap: 3px;
    margin: 10px 0 0;
    padding: 0;
    list-style: none;
    color: var(--faint);
    font-size: 11.5px;
  }
</style>
