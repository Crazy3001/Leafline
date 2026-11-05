<script lang="ts">
  // Pause and a new session, while live reading is on. Every session of 5 minutes or more saves
  // itself; ones the player doesn't want are deleted in the training log.
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
    <span class="saved" title="Sessions of 5 minutes or more save themselves; delete any you don't want in the training log. A break of 30 minutes ends a session.">
      Saves itself
    </span>
  </div>
{/if}

<style>
  .controls {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .saved {
    margin-left: auto;
    font-size: var(--text-xs);
    color: var(--faint);
    cursor: help;
  }
</style>
