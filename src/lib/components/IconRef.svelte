<script lang="ts">
  // A picture that opens its card: a monster beside the item it drops, say. Without a picture it
  // falls back to the name, small and muted, so the row stays one line.
  import { openPeek, type PeekKind } from "$lib/peek.svelte";
  import { tip, type Tip } from "$lib/tip.svelte";
  import Pic from "$lib/ui/Pic.svelte";

  let {
    kind,
    id = null,
    name,
    size = 20,
    title,
  }: {
    kind: Extract<PeekKind, "item" | "mob" | "npc">;
    id?: number | null;
    name: string;
    size?: number;
    /** What the tooltip says; the name by default, since a picture alone doesn't. */
    title?: Tip;
  } = $props();
</script>

<button
  class="icon-ref"
  aria-label={name}
  use:tip={title ?? name}
  onclick={(e) => {
    e.stopPropagation();
    openPeek({ kind, id, name }, e.currentTarget);
  }}
>
  <Pic {kind} {id} {name} {size}>
    {#snippet fallback()}<span class="label">{name}</span>{/snippet}
  </Pic>
</button>

<style>
  .icon-ref {
    display: inline-flex;
    flex: none;
    align-items: center;
    justify-content: center;
    min-width: 24px;
    height: 24px;
    padding: 0 2px;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    background: transparent;
    cursor: pointer;
    transition:
      background 0.12s ease,
      border-color 0.12s ease;
  }

  .icon-ref:hover,
  .icon-ref:focus-visible {
    border-color: var(--border-strong);
    background: var(--surface-3);
    outline: none;
  }

  .label {
    max-width: 96px;
    overflow: hidden;
    padding: 0 3px;
    color: var(--muted);
    font-size: var(--text-xs);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
