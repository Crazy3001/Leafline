<script lang="ts">
  // Saved sessions: each map's EXP per hour over all of them, and the sessions themselves. Live
  // sessions save themselves as they go.
  import { Check, ChevronDown, ChevronRight, Copy, PenLine, X } from "@lucide/svelte";

  import Card from "./Card.svelte";
  import Section from "./map/Section.svelte";
  import { backend } from "$lib/api";
  import { formatCompact, formatDay, formatDuration, formatNumber, partyLabel, splitMapLabel } from "$lib/format";
  import { history, watchHistory } from "$lib/history.svelte";
  import type { SavedSession, SessionLine } from "$lib/types";

  watchHistory();

  const MAPS = 5;
  /** Less time than this on a map makes its rate a guess; such rows are dimmed. */
  const SHORT = 10 * 60;

  const value = $derived(history.value);
  const maps = $derived(value ? [...value.maps].sort((a, b) => b.expPerHour - a.expPerHour).slice(0, MAPS) : []);

  let open = $state<number | null>(null);
  let renaming = $state<string | null>(null);
  let newName = $state("");
  let copied = $state<string | null>(null);
  let problem = $state<string | null>(null);
  let noteTimer: ReturnType<typeof setTimeout> | undefined;

  function mainMap(maps: { map: string }[]) {
    return maps.length ? splitMapLabel(maps[0].map).name : "No map read";
  }

  function saveNote(id: number, note: string) {
    clearTimeout(noteTimer);
    noteTimer = setTimeout(() => backend.setSessionNote(id, note).catch((e) => (problem = String(e))), 400);
  }

  const lineRate = (line: SessionLine) => {
    if (line.activeSeconds <= 0) return "—";
    if (line.expGained > 0) return `${formatCompact((line.expGained * 3600) / line.activeSeconds)} EXP/hr`;
    return `${((line.percentGained * 3600) / line.activeSeconds).toFixed(1)}%/hr`;
  };

  const lineMesos = (line: SessionLine) =>
    line.mesosSeconds > 0 ? `${formatCompact((line.mesosGained * 3600) / line.mesosSeconds)} mesos/hr` : "—";

  function lineText(session: SavedSession, line: SessionLine) {
    const parts = [
      formatDay(session.startedAt ?? session.savedAt),
      line.map ? splitMapLabel(line.map).name : "Between maps",
      line.level != null ? `Lv ${line.level}` : null,
      line.job,
      partyLabel(line.party),
      formatDuration(line.activeSeconds),
      lineRate(line),
      lineMesos(line),
    ];
    return parts.filter(Boolean).join(" · ");
  }

  async function copyLine(session: SavedSession, line: SessionLine, key: string) {
    try {
      await navigator.clipboard.writeText(lineText(session, line));
      copied = key;
      setTimeout(() => (copied = copied === key ? null : copied), 1500);
    } catch (e) {
      problem = `Couldn't copy: ${e}`;
    }
  }

  function startRename(map: string) {
    renaming = map;
    newName = splitMapLabel(map).name;
    problem = null;
  }

  async function finishRename(map: string) {
    const { street } = splitMapLabel(map);
    const name = newName.trim();
    renaming = null;
    if (!name || name === splitMapLabel(map).name) return;
    try {
      await backend.renameMap(map, street ? `${street} : ${name}` : name);
    } catch (e) {
      problem = String(e);
    }
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
      {#if problem}<p class="problem">{problem}</p>{/if}
      <ul class="sessions">
        {#each value.sessions as session (session.id)}
          {@const isOpen = open === session.id}
          <li class:open={isOpen}>
            <div class="row">
              <button
                class="toggle"
                aria-expanded={isOpen}
                aria-label="{isOpen ? 'Hide' : 'Show'} this session's details"
                onclick={() => (open = isOpen ? null : session.id)}
              >
                {#if isOpen}<ChevronDown size={11} />{:else}<ChevronRight size={11} />{/if}
              </button>
              <span class="day tabular">{formatDay(session.startedAt ?? session.savedAt)}</span>
              <span class="what" title={session.label}>{session.label || mainMap(session.maps)}</span>
              {#if session.note}<span class="marked" title={session.note}><PenLine size={10} /></span>{/if}
              <span class="tabular muted">{formatDuration(session.activeSeconds)}</span>
              <span class="tabular rate">{formatCompact(session.expPerHour)}</span>
              <button
                class="remove"
                title="Delete this session"
                aria-label="Delete this session"
                onclick={() => backend.deleteSession(session.id)}><X size={11} /></button
              >
            </div>

            {#if isOpen}
              <div class="details">
                <p class="facts tabular">
                  {#if session.level != null}Lv {session.level}{/if}
                  {#if session.job}· {session.job}{/if}
                  · {formatNumber(session.expGained)} EXP
                  {#if session.levelUps}· {session.levelUps} level-up{session.levelUps === 1 ? "" : "s"}{/if}
                  {#if session.deaths}· {session.deaths} death{session.deaths === 1 ? "" : "s"}{/if}
                  {#if session.mesosPerHour != null}· {formatCompact(session.mesosPerHour)} mesos/hr{/if}
                </p>

                {#if session.lines.length}
                  <ul class="lines">
                    {#each session.lines as line, i (i)}
                      {@const key = `${session.id}:${i}`}
                      <li>
                        {#if renaming === line.map && line.map}
                          {@const map = line.map}
                          <input
                            class="rename"
                            aria-label="Map name"
                            bind:value={newName}
                            onblur={() => finishRename(map)}
                            onkeydown={(e) => {
                              if (e.key === "Enter") e.currentTarget.blur();
                              if (e.key === "Escape") renaming = null;
                            }}
                          />
                        {:else}
                          <span class="line-map" title={line.map ?? undefined}>
                            {line.map ? splitMapLabel(line.map).name : "Between maps"}
                          </span>
                        {/if}
                        <span class="line-actions">
                          {#if line.map}
                            {@const map = line.map}
                            <button title="Rename this map; the same misread is fixed from now on" aria-label="Rename map" onclick={() => startRename(map)}>
                              <PenLine size={10} />
                            </button>
                          {/if}
                          <button title="Copy this line" aria-label="Copy line" onclick={() => copyLine(session, line, key)}>
                            {#if copied === key}<Check size={10} />{:else}<Copy size={10} />{/if}
                          </button>
                        </span>
                        <span class="line-facts tabular">
                          {[line.level != null ? `Lv ${line.level}` : null, partyLabel(line.party), formatDuration(line.activeSeconds), lineRate(line), lineMesos(line)]
                            .filter(Boolean)
                            .join(" · ")}
                        </span>
                      </li>
                    {/each}
                  </ul>
                {/if}

                <textarea
                  aria-label="Note"
                  placeholder="A note on this session: the party, the pots, why you left. It stays on this PC."
                  rows="2"
                  maxlength="4000"
                  value={session.note}
                  oninput={(e) => saveNote(session.id, e.currentTarget.value)}
                ></textarea>
              </div>
            {/if}
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
    white-space: nowrap;
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

  .problem {
    margin: 0 0 4px;
    color: var(--bad);
    font-size: 11.5px;
  }

  .sessions {
    display: grid;
    gap: 1px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .row {
    display: flex;
    align-items: baseline;
    gap: 6px;
    padding: 2px 0;
    font-size: 11.5px;
  }

  .toggle {
    align-self: center;
    display: grid;
    flex: none;
    place-items: center;
    width: 14px;
    height: 16px;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--faint);
    cursor: pointer;
  }

  .toggle:hover {
    color: var(--text);
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

  .marked {
    align-self: center;
    display: inline-grid;
    color: var(--faint);
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
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--faint);
    opacity: 0;
    cursor: pointer;
  }

  .row:hover .remove,
  .remove:focus-visible {
    opacity: 1;
  }

  .remove:hover {
    color: var(--bad);
  }

  .details {
    display: grid;
    gap: 6px;
    margin: 2px 0 8px 20px;
    padding-left: 8px;
    border-left: 1px solid var(--border);
    font-size: 11.5px;
  }

  .facts {
    margin: 0;
    color: var(--muted);
  }

  .lines {
    display: grid;
    gap: 5px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .lines li {
    display: grid;
    grid-template-columns: 1fr auto;
    align-items: center;
    column-gap: 6px;
  }

  .line-map {
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .line-actions {
    display: inline-flex;
    gap: 2px;
  }

  .line-actions button {
    display: grid;
    place-items: center;
    width: 18px;
    height: 16px;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--faint);
    cursor: pointer;
  }

  .line-actions button:hover {
    color: var(--text);
  }

  .line-facts {
    grid-column: 1 / -1;
    color: var(--faint);
    font-size: 11px;
  }

  input.rename,
  textarea {
    width: 100%;
    padding: 3px 6px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--text);
    font: inherit;
    font-size: 11.5px;
  }

  textarea {
    resize: vertical;
  }

  textarea::placeholder {
    color: var(--faint);
  }
</style>
