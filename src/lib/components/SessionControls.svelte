<script lang="ts">
  // Pause and a new session, while live reading is on. Every session of 5 minutes or more saves
  // itself; ones the player doesn't want are deleted in the training log. That is said by the
  // info mark at the row's end, not in words beside the buttons.
  //
  // `compact` is for a small cell (the desk's Session tile and player bar): small buttons, "New"
  // for New session, and the row wraps under the clock rather than running past the cell's edge.
  import { Info, Pause, Play, RotateCcw } from "@lucide/svelte";

  import { backend } from "$lib/api";
  import { live } from "$lib/readings.svelte";
  import { keyHint } from "$lib/settings.svelte";
  import { tip } from "$lib/tip.svelte";
  import Button from "$lib/ui/Button.svelte";
  import type { Snippet } from "svelte";

  /** Shown at the end of the row in place of the autosave mark. */
  let { trailing, compact = false }: { trailing?: Snippet; compact?: boolean } = $props();

  const SAVED = "Autosaves: every session of 5 minutes or more is kept. Delete ones you don't want in the Training log.";

  const status = $derived(live.liveStatus.state === "reading" ? live.liveStatus : null);
</script>

{#if status}
  <div class="controls" class:compact>
    {@render buttons(status)}
    {#if trailing}
      <span class="end">{@render trailing()}</span>
    {:else}
      <span class="saved" class:end={!compact} use:tip={SAVED} role="img" aria-label={SAVED}><Info size={13} /></span>
    {/if}
  </div>
{:else if trailing}
  <div class="controls"><span class="end">{@render trailing()}</span></div>
{/if}

{#snippet buttons(status: { held: boolean })}
  <Button
    size={compact ? "sm" : "md"}
    pressed={status.held}
    title="{status.held ? 'Resume' : 'Pause'} tracking{keyHint('PauseTracking')}"
    onclick={() => backend.setLiveHeld(!status.held)}
  >
    {#if status.held}<Play size={12} />Resume{:else}<Pause size={12} />Pause{/if}
  </Button>
  <Button size={compact ? "sm" : "md"} title="Start a new session{keyHint('NewSession')}" onclick={() => backend.resetSession()}>
    <RotateCcw size={12} />{compact ? "New" : "New session"}
  </Button>
{/snippet}

<style>
  .controls {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-1-5);
    min-width: 0;
  }

  .controls.compact {
    gap: var(--space-1);
  }

  .end {
    margin-left: auto;
  }

  .saved {
    display: inline-flex;
    flex: none;
    color: var(--faint);
    cursor: help;
  }

  .saved:hover {
    color: var(--muted);
  }
</style>
