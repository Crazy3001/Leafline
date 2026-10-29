<script lang="ts">
  // Saved sessions: each map's EXP per hour over all of them, and the sessions themselves.
  import { X } from "@lucide/svelte";

  import Card from "./Card.svelte";
  import Section from "./map/Section.svelte";
  import { backend } from "$lib/api";
  import { formatCompact, formatDay, formatDuration, splitMapLabel } from "$lib/format";
  import { history, watchHistory } from "$lib/history.svelte";

  watchHistory();

  const MAPS = 5;
  /** Less time than this on a map makes its rate a guess; such rows are dimmed. */
  const SHORT = 10 * 60;

  const value = $derived(history.value);
  const maps = $derived(value ? [...value.maps].sort((a, b) => b.expPerHour - a.expPerHour).slice(0, MAPS) : []);

  function mainMap(maps: { map: string }[]) {
    return maps.length ? splitMapLabel(maps[0].map).name : "No map read";
  }
</script>

{#if value?.sessions.length}
  <Card title="History">
    {#snippet aside()}<span class="tabular">{value.sessions.length} saved</span>{/snippet}

    {#if maps.length}
      <table class="maps">
        <thead>
          <tr><th>Your best maps</th><th>Time</th><th>EXP / hr</th></tr>
        </thead>
        <tbody>
          {#each maps as row (row.map)}
            <tr class:short={row.activeSeconds < SHORT}>
              <th scope="row" title={row.map}>{splitMapLabel(row.map).name}</th>
              <td class="tabular" title="{row.sessions} session{row.sessions === 1 ? '' : 's'}"
                >{formatDuration(row.activeSeconds)}</td
              >
              <td class="tabular">{formatCompact(row.expPerHour)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}

    <Section title="Sessions" count={value.sessions.length} open={false}>
      <ul class="sessions">
        {#each value.sessions as session (session.id)}
          <li>
            <span class="day tabular">{formatDay(session.startedAt ?? session.savedAt)}</span>
            <span class="what" title={session.label}>{session.label || mainMap(session.maps)}</span>
            <span class="tabular muted">{formatDuration(session.activeSeconds)}</span>
            <span class="tabular rate">{formatCompact(session.expPerHour)}</span>
            <button
              class="remove"
              title="Delete this session"
              aria-label="Delete this session"
              onclick={() => backend.deleteSession(session.id)}><X size={11} /></button
            >
          </li>
        {/each}
      </ul>
    </Section>
  </Card>
{/if}

<style>
  .maps {
    width: 100%;
    border-collapse: collapse;
    font-size: 12px;
  }

  .maps thead th {
    padding: 0 0 4px 10px;
    color: var(--faint);
    font-size: 10.5px;
    font-weight: 500;
    text-align: right;
  }

  .maps thead th:first-child {
    padding-left: 0;
    text-align: left;
  }

  .maps tbody th {
    max-width: 0;
    width: 100%;
    padding: 3px 8px 3px 0;
    overflow: hidden;
    font-weight: 500;
    text-align: left;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .maps td {
    padding: 3px 0 3px 10px;
    color: var(--muted);
    text-align: right;
    white-space: nowrap;
  }

  .maps td:last-child {
    color: var(--text);
    font-weight: 600;
  }

  .short th,
  .short td,
  .short td:last-child {
    color: var(--faint);
    font-weight: 500;
  }

  .sessions {
    display: grid;
    gap: 1px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .sessions li {
    display: flex;
    align-items: baseline;
    gap: 8px;
    padding: 2px 0;
    font-size: 11.5px;
  }

  .day {
    width: 42px;
    flex: none;
    color: var(--faint);
  }

  .what {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .muted {
    color: var(--faint);
  }

  .rate {
    width: 44px;
    text-align: right;
    font-weight: 600;
  }

  .remove {
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    padding: 0;
    border: 0;
    border-radius: 4px;
    background: transparent;
    color: var(--faint);
    opacity: 0;
    cursor: pointer;
  }

  .sessions li:hover .remove,
  .remove:focus-visible {
    opacity: 1;
  }

  .remove:hover {
    color: var(--bad);
  }
</style>
