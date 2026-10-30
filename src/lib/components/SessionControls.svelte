<script lang="ts">
  // Pause, a new session, and whether this run goes to the log, while live reading is on.
  import { Pause, Play, RotateCcw } from "@lucide/svelte";

  import { backend } from "$lib/api";
  import { live } from "$lib/readings.svelte";

  const status = $derived(live.liveStatus.state === "reading" ? live.liveStatus : null);
</script>

{#if status}
  <div class="controls">
    <button
      class:active={status.held}
      title="{status.held ? 'Resume' : 'Pause'} tracking. Paused time doesn't count. (Ctrl+Alt+K)"
      onclick={() => backend.setLiveHeld(!status.held)}
    >
      {#if status.held}<Play size={12} />Resume{:else}<Pause size={12} />Pause{/if}
    </button>
    <button
      title="Start a new session from zero. The one before stays in your history. (Ctrl+Alt+N)"
      onclick={() => backend.resetSession()}
    >
      <RotateCcw size={12} />New session
    </button>
    <label title="Turn off to keep this run out of your history. It turns back on with the next session. (Ctrl+Alt+J)">
      <input type="checkbox" checked={status.logging} onchange={(e) => backend.setLiveLogging(e.currentTarget.checked)} />
      Log this run
    </label>
  </div>
{/if}

<style>
  .controls {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 8px;
  }

  button {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 3px 8px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: 11.5px;
    cursor: pointer;
  }

  button:hover {
    background: rgba(255, 255, 255, 0.06);
  }

  button.active {
    color: var(--warn);
    border-color: var(--warn);
  }

  label {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    margin-left: auto;
    color: var(--muted);
    font-size: 11.5px;
    white-space: nowrap;
    cursor: pointer;
  }

  input {
    accent-color: var(--accent);
    margin: 0;
  }
</style>
