<script lang="ts">
  // A card's big picture on a soft tile. The tile goes when the picture is missing or pictures
  // are off, so the heading closes up around the name.
  import type { PictureKind } from "$lib/pictures";
  import Pic from "./Pic.svelte";

  let {
    kind,
    id = null,
    name = null,
    size = 48,
  }: { kind: PictureKind; id?: number | null; name?: string | null; size?: number } = $props();
</script>

<span class="frame {kind}" style:--frame="{size + 12}px"><Pic {kind} {id} {name} size={size} /></span>

<style>
  .frame {
    display: grid;
    flex: none;
    place-items: center;
    width: var(--frame);
    height: var(--frame);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background:
      radial-gradient(circle at 50% 35%, color-mix(in srgb, var(--tile, var(--accent)) 16%, transparent), transparent 70%),
      var(--surface-2);
  }

  .frame:empty {
    display: none;
  }

  .item {
    --tile: var(--ref-item);
  }

  .mob {
    --tile: var(--ref-mob);
  }

  .npc {
    --tile: var(--ref-npc);
  }

  .mark,
  .map {
    --tile: var(--ref-map);
  }
</style>
