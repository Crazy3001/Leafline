<script lang="ts">
  // "Go" beside a place's name: heads for it with one click, or cancels when it is the place set.
  // The way there shows in the bar under the overlay's header and in the compact HUD.
  import { Navigation } from "@lucide/svelte";

  import { isHeadingFor, toggleGo, watchDestination } from "$lib/destination.svelte";
  import type { Destination } from "$lib/types";

  let { place }: { place: Destination | null } = $props();

  watchDestination();

  const on = $derived(isHeadingFor(place));
</script>

{#if place}
  <button
    class="go"
    class:on
    aria-pressed={on}
    aria-label={on ? `Stop heading for ${place.name}` : `Go to ${place.name}`}
    onclick={(e) => {
      e.stopPropagation();
      void toggleGo(place);
    }}><Navigation size={9} />{on ? "Going" : "Go"}</button
  >
{/if}

<style>
  .go {
    display: inline-flex;
    flex: none;
    align-items: center;
    gap: 3px;
    height: 16px;
    padding: 0 4px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--muted);
    font-size: 10px;
    font-weight: 500;
    line-height: 1;
    vertical-align: 1px;
    cursor: pointer;
  }

  .go:hover,
  .go:focus-visible {
    border-color: var(--accent);
    color: var(--accent-strong);
    outline: none;
  }

  .go.on {
    border-color: var(--accent);
    background: var(--accent-soft);
    color: var(--accent-strong);
  }
</style>
