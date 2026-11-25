<script lang="ts">
  // A coloured name that opens its card: an item, monster, NPC, map or quest. The colour says
  // which, and a small picture sits before it when there is one. A quest also carries where it
  // stands in the character's quest log, when the log has seen it.
  import type { Snippet } from "svelte";

  import { tip, type Tip } from "$lib/tip.svelte";

  import { openPeek, type PeekKind } from "$lib/peek.svelte";
  import { splitMapLabel } from "$lib/format";
  import { isWorn } from "$lib/builds.svelte";
  import Pic from "$lib/ui/Pic.svelte";
  import QuestMark from "./quest/QuestMark.svelte";
  import type { PictureKind } from "$lib/pictures";

  let {
    kind,
    id = null,
    name = null,
    icon = true,
    mark = true,
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
    /** For a quest, show where it stands in the quest log. */
    mark?: boolean;
    class?: string;
    /** More about it on hover, never that it opens a card. */
    title?: Tip;
    /** The text shown, when it isn't the name. */
    children?: Snippet;
  } = $props();

  const PICTURE: Record<PeekKind, PictureKind | null> = { item: "item", mob: "mob", npc: "npc", map: "mark", quest: null, skill: "skill" };
  const picture = $derived(icon ? PICTURE[kind] : null);
  /** A map's label names the map after its street. */
  const pictureName = $derived(kind === "map" && name ? splitMapLabel(name).name : name);
  /** Gear the character wears has a colour of its own, wherever its name shows. */
  const wearing = $derived(kind === "item" && isWorn(id, name));
</script>

<button
  class="ref ref-{kind} {extra}"
  class:with-pic={!!picture}
  class:worn={wearing}
  use:tip={title ?? (wearing ? "You're wearing this" : undefined)}
  onclick={(e) => {
    e.stopPropagation();
    openPeek({ kind, id, name }, e.currentTarget);
  }}
>{#if picture}<Pic kind={picture} {id} name={pictureName} size={kind === "map" ? 14 : 18} class="ref-pic" />{/if}{#if children}{@render children()}{:else}{name}{/if}{#if kind === "quest" && id != null && mark}<QuestMark {id} />{/if}</button>
