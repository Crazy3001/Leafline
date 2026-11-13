<script lang="ts">
  // A map's card: its area and name, its picture with portals, NPCs and spawns marked (folded over
  // the game, open in the desk view), its monsters, exits and NPCs, and Go here.
  import GoButton from "../GoButton.svelte";
  import MapDetailsView from "../map/MapDetailsView.svelte";
  import Resolve from "./Resolve.svelte";
  import { backend } from "$lib/api";
  import { mapPlace } from "$lib/destination.svelte";
  import { splitMapLabel } from "$lib/format";
  import { pushPeek, type PeekTarget } from "$lib/peek.svelte";
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

</style>
