<script lang="ts">
  // A coloured name that opens its card: an item, monster, NPC, map or quest. The colour says
  // which, and a small picture sits before it when there is one.
  import type { Snippet } from "svelte";

  import { openPeek, type PeekKind } from "$lib/peek.svelte";
  import { splitMapLabel } from "$lib/format";
  import Pic from "$lib/ui/Pic.svelte";
  import type { PictureKind } from "$lib/pictures";

  let {
    kind,
    id = null,
    name = null,
    icon = true,
    class: extra = "",
    title,
    children,
  }: {
    kind: PeekKind;
    id?: number | null;
    /** The name, used to find the record when `id` isn't known. A map may be a `Street : Name` label. */
    name?: string | null;
    /** Show its picture, when it has one. */
    icon?: boolean;
    class?: string;
    title?: string;
    /** The text shown, when it isn't the name. */
    children?: Snippet;
  } = $props();

  const PICTURE: Record<PeekKind, PictureKind | null> = { item: "item", mob: "mob", npc: "npc", map: "mark", quest: null };
  const picture = $derived(icon ? PICTURE[kind] : null);
  /** A map's label names the map after its street. */
  const pictureName = $derived(kind === "map" && name ? splitMapLabel(name).name : name);
</script>

<button
  class="ref ref-{kind} {extra}"
  class:with-pic={!!picture}
  {title}
  onclick={(e) => {
    e.stopPropagation();
    openPeek({ kind, id, name }, e.currentTarget);
  }}
>{#if picture}<Pic kind={picture} {id} name={pictureName} size={kind === "map" ? 14 : 18} class="ref-pic" />{/if}{#if children}{@render children()}{:else}{name}{/if}</button>
