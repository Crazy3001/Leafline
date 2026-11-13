<script lang="ts">
  // A map's card: its area and name, a picture of the whole map (folded in the overlay and pinned
  // windows, which sit over the game), its monsters, exits and NPCs, and Go here.
  import { ChevronDown, ChevronRight } from "@lucide/svelte";

  import GoButton from "../GoButton.svelte";
  import MapDetailsView from "../map/MapDetailsView.svelte";
  import Resolve from "./Resolve.svelte";
  import { backend } from "$lib/api";
  import { mapPlace } from "$lib/destination.svelte";
  import { splitMapLabel } from "$lib/format";
  import { pushPeek, type PeekTarget } from "$lib/peek.svelte";
  import { display } from "$lib/settings.svelte";
  import type { MapDetails } from "$lib/types";
  import Pic from "$lib/ui/Pic.svelte";

  let { target }: { target: PeekTarget } = $props();

  async function find(): Promise<MapDetails | null> {
    let id = target.id;
    if (id == null && target.name) {
      const { street, name } = splitMapLabel(target.name);
      id = (await backend.findMap(name, street, null))?.card.map.id ?? null;
    }
    return id == null ? null : backend.mapCard(id);
  }

  /** Over the game the picture waits to be asked for; in the desk view it shows at once. */
  let open = $state(typeof location !== "undefined" && location.pathname.startsWith("/desk"));
</script>

<Resolve {find} what="map" name={target.name}>
  {#snippet children(details: MapDetails)}
    <div class="head">
      <Pic kind="mark" id={details.map.id} size={28} />
      <div class="names">
        {#if details.map.street || details.region}<div class="area">{details.map.street ?? details.region}</div>{/if}
        <div class="name">{details.map.name}</div>
      </div>
      <GoButton place={mapPlace(details.map)} />
    </div>

    {#if display.showPictures}
      <button class="fold" aria-expanded={open} onclick={() => (open = !open)}>
        {#if open}<ChevronDown size={13} />{:else}<ChevronRight size={13} />{/if}Map picture
      </button>
      {#if open}
        <div class="preview">
          <Pic kind="map" id={details.map.id} size={null} fit="contain" />
        </div>
      {/if}
    {/if}

    <MapDetailsView {details} variant="compact" onnavigate={(id) => pushPeek({ kind: "map", id, name: null })} />
  {/snippet}
</Resolve>

<style>
  .head {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 10px;
  }

  .names {
    flex: 1;
    min-width: 0;
  }

  .area {
    color: var(--muted);
    font-size: var(--text-xs);
    letter-spacing: 0.02em;
  }

  .name {
    font-family: var(--font-display);
    font-size: var(--text-xl);
    font-weight: 600;
    line-height: 1.15;
  }

  .fold {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    margin: 0 0 8px;
    padding: 2px 8px 2px 4px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--muted);
    font-size: var(--text-sm);
    cursor: pointer;
  }

  .fold:hover {
    background: var(--surface-3);
    color: var(--text);
  }

  .preview {
    overflow: hidden;
    margin-bottom: 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg);
    max-height: 220px;
  }

  .preview :global(img) {
    display: block;
    max-height: 220px;
  }
</style>
