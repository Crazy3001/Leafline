<script lang="ts">
  // A session typed in: what the EXP bar showed when you started and when you stopped. The website
  // reads nothing from the game, so this is how it learns your pace; the program offers it while
  // live reading is off. Start now keeps the start and the time in this browser; or type the
  // minutes you already played. Saved, it joins the training log like any session.
  import { Play, RotateCcw } from "@lucide/svelte";
  import { onMount } from "svelte";

  import { backend } from "$lib/api";
  import { characters } from "$lib/characters.svelte";
  import { formatCompact, formatDuration } from "$lib/format";
  import type { HudAt, MapRef, TypedOutcome } from "$lib/types";

  import Card from "./Card.svelte";

  const KEY = "leafline.typed-session";

  interface Side {
    level: string;
    percent: string;
    points: string;
    mesos: string;
  }

  interface Draft {
    start: Side;
    end: Side;
    /** When Start now was pressed, in ms. */
    startedAt: number | null;
    minutes: string;
    map: string;
  }

  const blank = (): Side => ({ level: "", percent: "", points: "", mesos: "" });

  function load(): Draft {
    try {
      const saved = JSON.parse(localStorage.getItem(KEY) ?? "null");
      if (saved?.start && saved?.end) return saved;
    } catch {
      // A missing or unreadable draft starts empty.
    }
    return { start: blank(), end: blank(), startedAt: null, minutes: "", map: "" };
  }

  let draft = $state<Draft>(load());
  let now = $state(Date.now());
  let outcome = $state<TypedOutcome | null>(null);
  let problem = $state<string | null>(null);
  let saved = $state<string | null>(null);
  let maps = $state<MapRef[]>([]);

  onMount(() => {
    const tick = setInterval(() => (now = Date.now()), 1000);
    return () => clearInterval(tick);
  });

  $effect(() => {
    try {
      localStorage.setItem(KEY, JSON.stringify($state.snapshot(draft)));
    } catch {
      // The draft is a convenience; without storage it lasts as long as the page.
    }
  });

  const number = (text: string): number | null => {
    const n = Number(text.replace(/[,\s]/g, ""));
    return text.trim() === "" || !Number.isFinite(n) ? null : n;
  };

  function hud(side: Side): HudAt | null {
    const level = number(side.level);
    const percent = number(side.percent.replace("%", ""));
    if (level == null || percent == null) return null;
    return { level, percent, points: number(side.points), mesos: number(side.mesos) };
  }

  const timedMinutes = $derived(draft.startedAt ? (now - draft.startedAt) / 60_000 : null);
  const minutes = $derived(timedMinutes ?? number(draft.minutes));

  function session() {
    const start = hud(draft.start);
    const end = hud(draft.end);
    if (!start || !end || minutes == null) return null;
    return {
      character: characters.viewing,
      map: draft.map.trim() || null,
      start,
      end,
      minutes,
      startedAt: draft.startedAt ? draft.startedAt / 1000 : null,
      note: null,
    };
  }

  // What it works out to, as you type. Asked again only when an input changes, not every tick.
  let asked = 0;
  $effect(() => {
    const s = session();
    const ask = ++asked;
    saved = null;
    if (!s) {
      outcome = null;
      problem = null;
      return;
    }
    // The running clock only changes the rates, not whether it adds up: round it to the minute.
    s.minutes = Math.max(1, Math.round(s.minutes));
    backend.typedSessionOutcome(s).then(
      (o) => ask === asked && ((outcome = o), (problem = null)),
      (e) => ask === asked && ((outcome = null), (problem = e instanceof Error ? e.message : String(e))),
    );
  });

  function startNow() {
    draft.startedAt = Date.now();
    draft.minutes = "";
  }

  function reset() {
    draft = { start: blank(), end: blank(), startedAt: null, minutes: "", map: "" };
    outcome = null;
    problem = null;
  }

  async function save() {
    const s = session();
    if (!s) return;
    s.minutes = Math.max(1, Math.round(s.minutes * 10) / 10);
    try {
      await backend.saveTypedSession(s);
      const kept = { ...draft.end };
      reset();
      // The next session starts where this one ended.
      draft.start = { ...kept, points: kept.points, mesos: kept.mesos };
      saved = "Saved to the training log.";
    } catch (e) {
      problem = e instanceof Error ? e.message : String(e);
    }
  }

  async function findMaps(text: string) {
    maps = text.trim().length >= 2 ? await backend.searchMaps(text).catch(() => []) : [];
  }

  const mapLabel = (m: MapRef) => (m.street ? `${m.street} : ${m.name}` : m.name);
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
      <button class="link" onclick={() => (draft.startedAt = null)}>Type the minutes instead</button>
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
    font-size: var(--text-sm);
  }

  .hint.warn {
    color: var(--warn);
  }

  .hint.good {
    color: var(--good);
  }

  .sides {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
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
    font-size: var(--text-sm);
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
    font-size: var(--text-sm);
  }

  .clock {
    font-weight: 600;
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
    font-weight: 600;
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
    font-size: var(--text-sm);
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
    font-size: var(--text-sm);
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
