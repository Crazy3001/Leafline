<script lang="ts">
  // Pause, a new session, and whether this run goes to the log, while live reading is on.
  import { Pause, Play, RotateCcw } from "@lucide/svelte";

  import { backend } from "$lib/api";
  import { live } from "$lib/readings.svelte";
  import { keyHint } from "$lib/settings.svelte";
  import Button from "$lib/ui/Button.svelte";

  const status = $derived(live.liveStatus.state === "reading" ? live.liveStatus : null);
</script>

{#if status}
  <div class="controls">
    <Button
      pressed={status.held}
      title="{status.held ? 'Resume' : 'Pause'} tracking. Paused time doesn't count.{keyHint('PauseTracking')}"
      onclick={() => backend.setLiveHeld(!status.held)}
    >
      {#if status.held}<Play size={12} />Resume{:else}<Pause size={12} />Pause{/if}
    </Button>
    <Button
      title="Start a new session from zero. The one before stays in your log.{keyHint('NewSession')}"
      onclick={() => backend.resetSession()}
    >
      <RotateCcw size={12} />New session
    </Button>
    <label title="Turn off to keep this run out of your log. It turns back on with the next session.{keyHint('ToggleLogging')}">
      <input type="checkbox" checked={status.logging} onchange={(e) => backend.setLiveLogging(e.currentTarget.checked)} />
      Save to log
    </label>
  </div>
{/if}

<style>
  .controls {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  label {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    margin-left: auto;
    font-size: var(--text-sm);
    color: var(--muted);
    cursor: pointer;
  }

  input {
    margin: 0;
    accent-color: var(--accent);
  }
</style>
