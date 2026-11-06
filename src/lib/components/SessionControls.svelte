<script lang="ts">
  // Pause and a new session, while live reading is on. Every session of 5 minutes or more saves
  // itself; ones the player doesn't want are deleted in the training log.
  import { Pause, Play, RotateCcw } from "@lucide/svelte";

  import { backend } from "$lib/api";
  import { live } from "$lib/readings.svelte";
  import { keyHint } from "$lib/settings.svelte";
  import Button from "$lib/ui/Button.svelte";
  import type { Snippet } from "svelte";

  /** Shown at the end of the row in place of the autosave note. */
  let { trailing }: { trailing?: Snippet } = $props();

  const AUTOSAVE = "Sessions of 5 minutes or more save themselves; delete any you don't want in the training log. A break of 30 minutes ends a session.";

  const status = $derived(live.liveStatus.state === "reading" ? live.liveStatus : null);
</script>

{#if status}
  <div class="controls">
    {@render buttons(status)}
    {#if trailing}
      <span class="end">{@render trailing()}</span>
    {:else}
      <span class="end saved" title={AUTOSAVE}>Autosaves</span>
    {/if}
  </div>
{:else if trailing}
  <div class="controls"><span class="end">{@render trailing()}</span></div>
{/if}

{#snippet buttons(status: { held: boolean })}
    <Button
      pressed={status.held}
      title="{status.held ? 'Resume' : 'Pause'} tracking. Paused time doesn't count.{keyHint('PauseTracking')}"
      onclick={() => backend.setLiveHeld(!status.held)}
    >
      {#if status.held}<Play size={12} />Resume{:else}<Pause size={12} />Pause{/if}
    </Button>
    <Button
      title="Start a new session from zero. The one before stays in your log. {AUTOSAVE}{keyHint('NewSession')}"
      onclick={() => backend.resetSession()}
    >
      <RotateCcw size={12} />New session
    </Button>
{/snippet}

<style>
  .controls {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .end {
    margin-left: auto;
  }

  .saved {
    font-size: var(--text-xs);
    color: var(--faint);
    cursor: help;
  }
</style>
