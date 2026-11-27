<script lang="ts">
  // A character's look (cut from the game's Character Info window) drawn crisply: the game's pixels
  // made bigger by whole numbers only, never smoothed. "bust" fills a small round badge with the
  // head and shoulders; "whole" stands the character on the box's floor.
  import type { CharacterLook } from "$lib/types";

  let {
    look,
    size,
    crop = "whole",
  }: {
    look: CharacterLook;
    /** The square box it fills, in CSS pixels. */
    size: number;
    crop?: "bust" | "whole";
  } = $props();

  const scale = $derived.by(() => {
    if (crop === "bust") return Math.max(1, Math.floor(size / (look.width * 0.9)));
    const fit = Math.min(size / look.width, size / look.height);
    return fit >= 1 ? Math.floor(fit) : fit;
  });
</script>

<span class="look {crop}" style:width="{size}px" style:height="{size}px">
  <img src={look.src} alt="" width={look.width * scale} height={look.height * scale} draggable="false" />
</span>

<style>
  .look {
    display: flex;
    flex: none;
    justify-content: center;
    overflow: hidden;
  }

  .whole {
    align-items: flex-end;
  }

  /* The head a little below the top, so a round frame doesn't cut it. */
  .bust {
    align-items: flex-start;
    padding-top: 8%;
  }

  img {
    flex: none;
    max-width: none;
    image-rendering: pixelated;
    -webkit-user-drag: none;
    user-select: none;
  }
</style>
