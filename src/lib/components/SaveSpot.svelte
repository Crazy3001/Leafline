<script lang="ts">
  // A bookmark for a map: saves it to "your spots" (the Saved list in Where to train). It takes
  // the map's ID, or a session's `Street : Name` label to find it by.
  import { Bookmark } from "@lucide/svelte";
  import { onMount } from "svelte";

  import { backend } from "$lib/api";
  import { splitMapLabel } from "$lib/format";
  import { isSpotSaved, toggleSpot, watchSpots } from "$lib/spots.svelte";

  let {
    mapId = null,
    label = null,
    name,
    /** Draws the word beside the bookmark, for where the bare icon would be easy to miss. */
    text = false,
  }: { mapId?: number | null; label?: string | null; name: string; text?: boolean } = $props();

  let found = $state<number | null>(null);
  const id = $derived(mapId ?? found);
  const saved = $derived(id != null && isSpotSaved(id));

  onMount(() => {
    watchSpots();
  });

  // A label has to be matched to a map first; no match, no button.
  $effect(() => {
    if (mapId != null || !label) return;
    const { street, name: mapName } = splitMapLabel(label);
    let current = true;
    backend
      .findMap(mapName, street, null)
      .then((match) => {
        if (current) found = match?.card.map.id ?? null;
      })
      .catch(() => (found = null));
    return () => (current = false);
  });
</script>

{#if id != null}
  <button
    class="save"
    class:on={saved}
    class:text
    aria-pressed={saved}
    title={saved ? "Remove from your spots" : "Save to your spots"}
    aria-label={saved ? `Remove ${name} from your spots` : `Save ${name} to your spots`}
    onclick={(e) => {
      e.stopPropagation();
      void toggleSpot(id, name);
    }}
  >
    <Bookmark size={13} fill={saved ? "currentColor" : "none"} />{#if text}<span>{saved ? "Saved" : "Save"}</span>{/if}
  </button>
{/if}

<style>
  .save {
    display: inline-flex;
    flex: none;
    align-items: center;
    justify-content: center;
    gap: 5px;
    min-width: 26px;
    height: 26px;
    padding: 0;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--muted);
    font-size: var(--text-xs);
    vertical-align: middle;
    cursor: pointer;
  }

  .save.text {
    padding: 0 8px;
    border: 1px solid var(--border-strong);
    background: var(--surface-2);
  }

  .save:hover {
    background: rgba(255, 255, 255, 0.07);
    color: var(--text);
  }

  .save.on {
    color: var(--accent);
  }

  .save:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }
</style>
