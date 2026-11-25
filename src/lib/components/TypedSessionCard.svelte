<script lang="ts">
  // A session typed in: what the EXP bar showed when you started and when you stopped. The website
  // reads nothing from the game, so this is how it learns your pace; the program offers it while
  // live reading is off. Start now keeps the start and the time in this browser; or type the
  // minutes you already played. Saved, it joins the training log like any session. The website's
  // pop-out shows the same session (see typed.svelte.ts).
  import { Play, RotateCcw } from "@lucide/svelte";

  import { backend } from "$lib/api";
  import { formatCompact, formatDuration } from "$lib/format";
  import { followTyped, resetTyped, saveTyped, typed } from "$lib/typed.svelte";
  import type { MapRef } from "$lib/types";

  import Card from "./Card.svelte";

  followTyped();

  const draft = $derived(typed.draft);
  const outcome = $derived(typed.outcome);
  let saved = $state<string | null>(null);
  let failed = $state<string | null>(null);
  let maps = $state<MapRef[]>([]);

  const problem = $derived(failed ?? typed.problem);

  function startNow() {
    typed.draft.startedAt = Date.now();
    typed.draft.minutes = "";
  }

  function reset() {
    resetTyped();
    saved = null;
    failed = null;
  }

  async function save() {
    failed = null;
    try {
      await saveTyped();
      saved = "Saved to the training log.";
    } catch (e) {
      failed = e instanceof Error ? e.message : String(e);
    }
  }

  async function findMaps(text: string) {
    saved = null;
    maps = text.trim().length >= 2 ? await backend.searchMaps(text).catch(() => []) : [];
  }

  const mapLabel = (m: MapRef) => (m.street ? `${m.street} : ${m.name}` : m.name);
  const now = $derived(typed.now);
</script>

<Card title="Session">
  {#snippet aside()}
    <button class="icon" aria-label="Clear" title="Clear" onclick={reset}><RotateCcw size={12} /></button>
  {/snippet}
  <p class="lead">Type what your EXP bar shows when you start and when you stop.</p>

  <div class="sides">
    {#each [["Start", draft.start], ["End", draft.end]] as const as [name, side] (name)}
      <fieldset>
        <legend>{name}</legend>
        <label><span>Level</span><input inputmode="numeric" bind:value={side.level} placeholder="30" /></label>
        <label><span>EXP %</span><input inputmode="decimal" bind:value={side.percent} placeholder="12.34" /></label>
        <label class="optional"><span>EXP points</span><input inputmode="numeric" bind:value={side.points} placeholder="optional" /></label>
        <label class="optional"><span>Mesos</span><input inputmode="numeric" bind:value={side.mesos} placeholder="optional" /></label>
      </fieldset>
    {/each}
  </div>

  <div class="time">
    {#if draft.startedAt}
      <span class="clock tabular">Playing for {formatDuration((now - draft.startedAt) / 1000)}</span>
      <button class="link" onclick={() => (typed.draft.startedAt = null)}>Type the minutes instead</button>
    {:else}
      <label><span>Minutes played</span><input inputmode="numeric" bind:value={draft.minutes} placeholder="60" /></label>
      <span class="or">or</span>
      <button class="go" onclick={startNow}><Play size={12} />Start now</button>
    {/if}
  </div>

  <label class="map">
    <span>Map</span>
    <input list="typed-session-maps" bind:value={draft.map} oninput={(e) => findMaps(e.currentTarget.value)} placeholder="optional, for where to train" />
    <datalist id="typed-session-maps">
      {#each maps as m (m.id)}<option value={mapLabel(m)}></option>{/each}
    </datalist>
  </label>

  {#if outcome}
    <dl class="outcome">
      <div><dt>EXP / hr</dt><dd class="tabular">{outcome.expPerHour != null ? formatCompact(outcome.expPerHour) : "—"}</dd></div>
      <div><dt>% / hr</dt><dd class="tabular">{outcome.percentPerHour.toFixed(1)}%</dd></div>
      <div>
        <dt>Next level</dt>
        <dd class="tabular">{outcome.minutesToLevel != null ? formatDuration(outcome.minutesToLevel * 60) : "—"}</dd>
      </div>
      {#if outcome.mesosPerHour != null}
        <div><dt>Mesos / hr</dt><dd class="tabular">{formatCompact(outcome.mesosPerHour)}</dd></div>
      {/if}
    </dl>
    {#if outcome.expPerHour == null}
      <p class="hint">Type the EXP points too for EXP an hour.</p>
    {/if}
  {/if}
  {#if problem}<p class="hint warn">{problem}</p>{/if}
  {#if saved}<p class="hint good">{saved}</p>{/if}

  <div class="actions">
    <button class="go" onclick={save} disabled={!outcome}>Save to the training log</button>
  </div>
</Card>

<style>
  .lead,
  .hint {
    margin: 0 0 10px;
    color: var(--muted);
    font-size: var(--text-md);
  }

  .hint.warn {
    color: var(--warn);
  }

  .hint.good {
    color: var(--good);
  }

  .sides {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(210px, 1fr));
    gap: 10px;
  }

  fieldset {
    display: grid;
    gap: 6px;
    margin: 0;
    padding: 8px 10px 10px;
    border: var(--line-w) solid var(--line);
    border-radius: var(--radius);
  }

  legend {
    padding: 0 4px;
    color: var(--muted);
    font-size: var(--text-xs);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  label {
    display: grid;
    grid-template-columns: 76px 1fr;
    align-items: center;
    gap: 8px;
    font-size: var(--text-md);
  }

  label span {
    color: var(--muted);
  }

  .optional span {
    color: var(--faint);
  }

  input {
    min-width: 0;
    height: var(--control-h);
    padding: 0 8px;
    border: var(--line-w) solid var(--control-line);
    border-radius: var(--radius);
    background: var(--surface);
    color: var(--text);
    font: inherit;
  }

  input:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  .time,
  .map {
    margin-top: 10px;
  }

  .time {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
  }

  .time label {
    grid-template-columns: auto 80px;
  }

  .map {
    grid-template-columns: 76px 1fr;
  }

  .or {
    color: var(--faint);
    font-size: var(--text-md);
  }

  .clock {
    font-weight: var(--weight-strong);
  }

  .outcome {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(110px, 1fr));
    gap: 8px;
    margin: 12px 0 6px;
  }

  .outcome dt {
    color: var(--muted);
    font-size: var(--text-xs);
  }

  .outcome dd {
    margin: 2px 0 0;
    font-size: var(--text-lg);
    font-weight: var(--weight-strong);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    margin-top: 8px;
  }

  .go {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: var(--control-h);
    padding: 0 12px;
    border: var(--line-w) solid var(--accent);
    border-radius: var(--radius);
    background: var(--accent);
    color: var(--accent-ink);
    font: inherit;
    font-size: var(--text-md);
    cursor: pointer;
  }

  .go:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .link {
    padding: 0;
    border: 0;
    background: none;
    color: var(--accent);
    font: inherit;
    font-size: var(--text-md);
    cursor: pointer;
  }

  .icon {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    padding: 0;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--faint);
    cursor: pointer;
  }

  .icon:hover {
    background: var(--hover);
    color: var(--text);
  }
</style>
