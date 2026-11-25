<script lang="ts">
  // The website's pop-out: the session being typed in, in a small window over the game. The clock
  // runs from Start now; type where your EXP bar is now and the rates follow.
  import { formatCompact, formatDuration } from "$lib/format";
  import { followTyped, playedMinutes, typed } from "$lib/typed.svelte";

  followTyped();

  const draft = $derived(typed.draft);
  const outcome = $derived(typed.outcome);
  const minutes = $derived.by(() => {
    void typed.now;
    return playedMinutes();
  });
  const level = $derived(draft.end.level || draft.start.level || "—");
</script>

<div class="hud">
  <div class="cell lv">
    <span class="label">Lv</span>
    <span class="value tabular">{level}</span>
  </div>
  <div class="cell">
    <span class="label">EXP / hr</span>
    <span class="value tabular">
      {#if outcome?.expPerHour != null}{formatCompact(outcome.expPerHour)}{:else if outcome}{outcome.percentPerHour.toFixed(1)}%{:else}—{/if}
    </span>
  </div>
  <div class="cell">
    <span class="label">Next level</span>
    <span class="value tabular">{outcome?.minutesToLevel != null ? formatDuration(outcome.minutesToLevel * 60) : "—"}</span>
  </div>
  <div class="cell">
    <span class="label">{draft.startedAt ? "Playing" : "Played"}</span>
    <span class="value tabular">{minutes != null ? formatDuration(minutes * 60) : "—"}</span>
  </div>
  <label class="now">
    <span class="label">EXP now</span>
    <span class="inputs">
      <input aria-label="Level now" inputmode="numeric" bind:value={draft.end.level} placeholder="Lv" />
      <input aria-label="EXP percent now" inputmode="decimal" bind:value={draft.end.percent} placeholder="%" />
    </span>
  </label>
</div>
{#if !draft.startedAt && !draft.minutes}
  <p class="hint">Press Start now in Session, on Now, to time your play.</p>
{/if}

<style>
  .hud {
    display: flex;
    flex-wrap: wrap;
    align-items: stretch;
    gap: 6px;
    padding: 8px;
    color: var(--text);
    font-family: var(--font-body, inherit);
  }

  .cell,
  .now {
    display: grid;
    gap: 2px;
    padding: 6px 10px;
    border: var(--line-w) solid var(--line);
    border-radius: var(--radius);
    background: var(--surface);
  }

  .label {
    color: var(--muted);
    font-size: var(--text-xs);
  }

  .value {
    font-size: var(--text-lg);
    font-weight: var(--weight-strong);
  }

  .lv .value {
    color: var(--accent);
  }

  .inputs {
    display: flex;
    gap: 4px;
  }

  input {
    width: 52px;
    height: 24px;
    padding: 0 6px;
    border: var(--line-w) solid var(--control-line);
    border-radius: var(--radius-sm);
    background: var(--bg);
    color: var(--text);
    font: inherit;
    font-size: var(--text-md);
  }

  .hint {
    margin: 0 8px 8px;
    color: var(--faint);
    font-size: var(--text-xs);
  }
</style>
