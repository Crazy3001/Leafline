<script lang="ts">
  // The player's note on a map, such as where to stand or which channel is quiet. One per map,
  // shown in the Map tab, the spot list and the training log, and kept on this PC.
  import { onMount } from "svelte";

  import NoteBox from "./NoteBox.svelte";
  import { backend } from "$lib/api";

  let {
    mapId,
    name,
    flush = false,
  }: { mapId: number | null; name: string; flush?: boolean } = $props();

  /** Bumped when any window writes a note on this map. */
  let changed = $state(0);

  onMount(() => {
    const stop = backend.onMapNote((id) => {
      if (id === mapId) changed += 1;
    });
    return () => {
      stop.then((unlisten) => unlisten());
    };
  });
</script>

<NoteBox
  key={mapId}
  {changed}
  {flush}
  load={() => (mapId == null ? Promise.resolve(null) : backend.mapNote(mapId))}
  save={(text) => (mapId == null ? Promise.resolve(null) : backend.setMapNote(mapId, name, text))}
  placeholder="Where to stand, which channel is quiet, what to bring"
/>
