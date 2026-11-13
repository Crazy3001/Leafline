<script lang="ts">
  // The one link card a window shows, beside the link that opened it. Every window's layout
  // holds one; it stays empty until a coloured name is clicked.
  import { ArrowLeft, Maximize2, X } from "@lucide/svelte";

  import ItemPeek from "./ItemPeek.svelte";
  import MapPeek from "./MapPeek.svelte";
  import MobPeek from "./MobPeek.svelte";
  import NpcPeek from "./NpcPeek.svelte";
  import QuestPeek from "./QuestPeek.svelte";
  import { closePeek, openFull, peek, peekBack, type PeekKind } from "$lib/peek.svelte";

  const KIND: Record<PeekKind, { label: string; tab: string }> = {
    item: { label: "Item", tab: "Lookup" },
    mob: { label: "Monster", tab: "Lookup" },
    npc: { label: "NPC", tab: "Lookup" },
    map: { label: "Map", tab: "Lookup" },
    quest: { label: "Quest", tab: "Quests" },
  };

  const MARGIN = 8;
  const GAP = 6;
  const WIDTH = 380;

  const top = $derived(peek.stack.at(-1) ?? null);
  let card = $state<HTMLElement | null>(null);
  let viewport = $state({ width: 0, height: 0 });

  function measure() {
    viewport = { width: window.innerWidth, height: window.innerHeight };
  }

  /** Below the link when there is room, else above it; always inside the window. */
  const place = $derived.by(() => {
    const { width: vw, height: vh } = viewport;
    if (!vw || !vh) return null;
    const width = Math.min(WIDTH, vw - MARGIN * 2);
    const a = peek.anchor;
    if (!a) return { left: (vw - width) / 2, width, top: MARGIN, bottom: null, maxHeight: vh - MARGIN * 2 };
    const left = Math.min(Math.max(a.left - 12, MARGIN), vw - width - MARGIN);
    const below = vh - a.bottom - GAP - MARGIN;
    const above = a.top - GAP - MARGIN;
    if (below >= 280 || below >= above) {
      return { left, width, top: a.bottom + GAP, bottom: null, maxHeight: Math.max(below, 140) };
    }
    return { left, width, top: null, bottom: vh - a.top + GAP, maxHeight: above };
  });

  $effect(() => {
    if (!top) return;
    measure();
    const outside = (e: PointerEvent) => {
      const target = e.target as Element | null;
      if (!target?.closest?.("[data-peek]") && !target?.closest?.(".ref")) closePeek();
    };
    const key = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        closePeek();
      }
    };
    window.addEventListener("pointerdown", outside, true);
    window.addEventListener("keydown", key);
    window.addEventListener("resize", measure);
    return () => {
      window.removeEventListener("pointerdown", outside, true);
      window.removeEventListener("keydown", key);
      window.removeEventListener("resize", measure);
    };
  });

  // A new card starts at its top.
  $effect(() => {
    void peek.stack.length;
    if (card) card.scrollTop = 0;
  });
</script>

{#if top && place}
  <div
    class="peek"
    data-peek
    role="dialog"
    aria-label={KIND[top.kind].label}
    style:left="{place.left}px"
    style:width="{place.width}px"
    style:top={place.top == null ? null : `${place.top}px`}
    style:bottom={place.bottom == null ? null : `${place.bottom}px`}
    style:max-height="{place.maxHeight}px"
  >
    <header>
      {#if peek.stack.length > 1}
        <button class="icon" aria-label="Back" title="Back" onclick={peekBack}><ArrowLeft size={14} /></button>
      {/if}
      <span class="kind ref-{top.kind}">{KIND[top.kind].label}</span>
      <span class="spacer"></span>
      <button class="full" title="Open the full view in the {KIND[top.kind].tab} tab" onclick={() => openFull(top)}>
        <Maximize2 size={12} />Full view
      </button>
      <button class="icon" aria-label="Close" title="Close (Esc)" onclick={closePeek}><X size={14} /></button>
    </header>
    <div class="body" bind:this={card}>
      {#key peek.stack.length + ":" + top.kind + ":" + (top.id ?? top.name)}
        {#if top.kind === "item"}
          <ItemPeek target={top} />
        {:else if top.kind === "mob"}
          <MobPeek target={top} />
        {:else if top.kind === "npc"}
          <NpcPeek target={top} />
        {:else if top.kind === "map"}
          <MapPeek target={top} />
        {:else}
          <QuestPeek target={top} />
        {/if}
      {/key}
    </div>
  </div>
{/if}

<style>
  .peek {
    position: fixed;
    z-index: 1000;
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg);
    background: var(--surface-raised);
    box-shadow: var(--shadow-lg);
    animation: rise 140ms cubic-bezier(0.2, 0.8, 0.2, 1);
  }

  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(4px) scale(0.985);
    }
  }

  header {
    display: flex;
    flex: none;
    align-items: center;
    gap: 6px;
    min-height: 36px;
    padding: 4px 6px 4px 12px;
    border-bottom: 1px solid var(--border);
    background: var(--surface-2);
  }

  .kind {
    color: var(--ref, var(--accent));
    font-family: var(--font-display);
    font-size: var(--text-xs);
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .ref-quest {
    --ref: var(--accent);
  }

  .spacer {
    flex: 1;
  }

  .icon,
  .full {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 26px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--muted);
    cursor: pointer;
  }

  .icon {
    justify-content: center;
    width: 26px;
  }

  .full {
    padding: 0 9px;
    font-size: var(--text-xs);
  }

  .icon:hover,
  .full:hover {
    background: var(--surface-3);
    color: var(--text);
  }

  .body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 12px 14px 14px;
    overscroll-behavior: contain;
  }
</style>
