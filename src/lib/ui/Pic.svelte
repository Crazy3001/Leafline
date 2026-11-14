<script lang="ts">
  // A game picture, or nothing at all when pictures are off or this one is missing, so the name
  // beside it stands alone. Small icons sit in line with text; big ones fill a card's frame.
  import type { Snippet } from "svelte";

  import { markMissing, pictureMissing, pictureUrl, type PictureKind } from "$lib/pictures";

  let {
    kind,
    id = null,
    name = null,
    size = 18,
    fit = "contain",
    class: extra = "",
    onstate,
    fallback,
  }: {
    kind: PictureKind;
    id?: number | null;
    /** Used when the id isn't known. */
    name?: string | null;
    /** The box the picture fits in, in pixels; null fills the parent's width. */
    size?: number | null;
    fit?: "contain" | "cover";
    class?: string;
    /** Told whether the picture showed, for a frame that hides itself without one. */
    onstate?: (shown: boolean) => void;
    /** Shown instead when there is no picture: pictures off, none in the data, or it failed. */
    fallback?: Snippet;
  } = $props();

  const src = $derived(pictureUrl(kind, id ?? name));
  let failed = $state<string | null>(null);
  let loaded = $state<string | null>(null);
  const shown = $derived(!!src && failed !== src && !pictureMissing(src));

  $effect(() => onstate?.(shown && loaded === src));
</script>

{#if shown && src}
  <img
    {src}
    alt=""
    class="pic {kind} {extra}"
    class:big={size == null || size >= 40}
    class:ready={loaded === src}
    style:width={size == null ? "100%" : `${size}px`}
    style:height={size == null ? "auto" : `${size}px`}
    style:object-fit={fit}
    loading="lazy"
    decoding="async"
    draggable="false"
    onload={() => (loaded = src)}
    onerror={() => {
      markMissing(src);
      failed = src;
    }}
  />
{:else if fallback}
  {@render fallback()}
{/if}

<style>
  .pic {
    flex: none;
    display: inline-block;
    vertical-align: -0.28em;
    opacity: 0;
    transition: opacity 160ms ease;
    /* Icons are small pixel art: scaled down they stay smooth, scaled up they stay crisp. */
    image-rendering: auto;
    user-select: none;
  }

  .pic.ready {
    opacity: 1;
  }

  .pic.big {
    image-rendering: pixelated;
  }

  .pic.map,
  .pic.minimap {
    image-rendering: auto;
  }
</style>
