<script lang="ts">
  // The one link card a window shows, beside the link that opened it. Every window's layout
  // holds one; it stays empty until a coloured name is clicked.
  import { tip } from "$lib/tip.svelte";
  import { ArrowLeft, Maximize2, Pin, X } from "@lucide/svelte";

  import ItemPeek from "./ItemPeek.svelte";
  import MapPeek from "./MapPeek.svelte";
  import MobPeek from "./MobPeek.svelte";
  import NpcPeek from "./NpcPeek.svelte";
  import QuestPeek from "./QuestPeek.svelte";
  import SkillPeek from "./SkillPeek.svelte";
  import { setContext } from "svelte";

  import {
    SECTION_SCOPE,
    closePeek,
    openFull,
    peek,
    peekBack,
    pinPeek,
    unpinCard,
    type PeekKind,
    type PeekTarget,
    type PinnedCard,
  } from "$lib/peek.svelte";

  // Sections inside a card start folded.
  setContext(SECTION_SCOPE, "card");

  const KIND: Record<PeekKind, { label: string; tab: string }> = {
    item: { label: "Item", tab: "Lookup" },
    mob: { label: "Monster", tab: "Lookup" },
    npc: { label: "NPC", tab: "Lookup" },
    map: { label: "Map", tab: "Lookup" },
    quest: { label: "Quest", tab: "Quests" },
    skill: { label: "Skill", tab: "Lookup" },
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

  /**
   * Drags a card by its header: `move` gets how far the pointer has gone since it went down.
   * Buttons in the header still click.
   */
  function dragFrom(e: PointerEvent, move: (dx: number, dy: number) => void) {
    if (e.button !== 0 || (e.target as Element).closest("button")) return;
    const header = e.currentTarget as HTMLElement;
    const start = { x: e.clientX, y: e.clientY };
    header.setPointerCapture(e.pointerId);
    const onMove = (m: PointerEvent) => move(m.clientX - start.x, m.clientY - start.y);
    const onUp = () => {
      header.removeEventListener("pointermove", onMove);
      header.removeEventListener("pointerup", onUp);
      header.removeEventListener("pointercancel", onUp);
    };
    header.addEventListener("pointermove", onMove);
    header.addEventListener("pointerup", onUp);
    header.addEventListener("pointercancel", onUp);
  }

  /** Keeps a dragged card's header inside the window, so it can always be grabbed again. */
  function clampLeft(left: number, width: number) {
    return Math.min(Math.max(left, MARGIN - width + 80), viewport.width - 80);
  }
  function clampTop(top: number) {
    return Math.min(Math.max(top, MARGIN), viewport.height - 40);
  }

  function dragOpen(e: PointerEvent) {
    const from = { ...peek.drag };
    dragFrom(e, (dx, dy) => (peek.drag = { x: from.x + dx, y: from.y + dy }));
  }

  function dragPinned(e: PointerEvent, card: PinnedCard) {
    const from = { left: card.left, top: card.top };
    dragFrom(e, (dx, dy) => {
      card.left = clampLeft(from.left + dx, WIDTH);
      card.top = clampTop(from.top + dy);
    });
  }

  function pin() {
    const box = openCard?.getBoundingClientRect();
    if (box) pinPeek(box.left, box.top);
  }

  let openCard = $state<HTMLElement | null>(null);

  $effect(() => {
    measure();
    window.addEventListener("resize", measure);
    return () => window.removeEventListener("resize", measure);
  });

  $effect(() => {
    if (!top) return;
    measure();
    const outside = (e: PointerEvent) => {
      const target = e.target as Element | null;
      if (!target?.closest?.("[data-peek]") && !target?.closest?.("[data-pinned-card]") && !target?.closest?.(".ref")) closePeek();
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

{#snippet content(target: PeekTarget)}
  {#if target.kind === "item"}
    <ItemPeek {target} />
  {:else if target.kind === "mob"}
    <MobPeek {target} />
  {:else if target.kind === "npc"}
    <NpcPeek {target} />
  {:else if target.kind === "map"}
    <MapPeek {target} />
  {:else if target.kind === "skill"}
    <SkillPeek {target} />
  {:else}
    <QuestPeek {target} />
  {/if}
{/snippet}

<!-- Pinned cards: each stays put until its own close, and can be dragged by its header. -->
{#each peek.pinned as card (card.key)}
  <div
    class="peek pinned"
    data-pinned-card
    role="dialog"
    aria-label="Pinned {KIND[card.target.kind].label.toLowerCase()} card"
    style:left="{card.left}px"
    style:top="{card.top}px"
    style:width="{Math.min(WIDTH, viewport.width - MARGIN * 2)}px"
    style:max-height="{Math.max(viewport.height - card.top - MARGIN, 160)}px"
  >
    <header class="grab" role="toolbar" aria-label="Card header: drag to move" tabindex="-1" onpointerdown={(e) => dragPinned(e, card)}>
      <span class="kind ref-{card.target.kind}">{KIND[card.target.kind].label}</span>
      <span class="pinned-name">{card.target.name ?? ""}</span>
      <span class="spacer"></span>
      <button class="full" use:tip={`In the ${KIND[card.target.kind].tab} tab`} onclick={() => openFull(card.target)}>
        <Maximize2 size={12} />Full view
      </button>
      <button class="icon" aria-label="Close" use:tip={"Close"} onclick={() => unpinCard(card.key)}><X size={14} /></button>
    </header>
    <div class="body">{@render content(card.target)}</div>
  </div>
{/each}

{#if top && place}
  <div
    class="peek"
    data-peek
    role="dialog"
    aria-label={KIND[top.kind].label}
    bind:this={openCard}
    style:left="{place.left + peek.drag.x}px"
    style:width="{place.width}px"
    style:top={place.top == null ? null : `${place.top + peek.drag.y}px`}
    style:bottom={place.bottom == null ? null : `${place.bottom - peek.drag.y}px`}
    style:max-height="{place.maxHeight}px"
  >
    <header class="grab" role="toolbar" aria-label="Card header: drag to move" tabindex="-1" onpointerdown={dragOpen}>
      {#if peek.stack.length > 1}
        <button class="icon" aria-label="Back" use:tip={"Back"} onclick={peekBack}><ArrowLeft size={14} /></button>
      {/if}
      <span class="kind ref-{top.kind}">{KIND[top.kind].label}</span>
      <span class="spacer"></span>
      <button class="full" use:tip={`In the ${KIND[top.kind].tab} tab`} onclick={() => openFull(top)}>
        <Maximize2 size={12} />Full view
      </button>
      <button class="icon" aria-label="Pin this card" use:tip={"Pin: keep it here and open more beside it"} onclick={pin}
        ><Pin size={13} /></button
      >
      <button class="icon" aria-label="Close" use:tip={"Close (Esc)"} onclick={closePeek}><X size={14} /></button>
    </header>
    <div class="body" bind:this={card}>
      {#key peek.stack.length + ":" + top.kind + ":" + (top.id ?? top.name)}
        {@render content(top)}
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

  /* The header is the handle a card is dragged by. */
  .grab {
    cursor: grab;
    touch-action: none;
    user-select: none;
  }

  .grab:active {
    cursor: grabbing;
  }

  .peek.pinned {
    z-index: 990;
    border-color: color-mix(in srgb, var(--accent) 45%, var(--border-strong));
    animation: none;
  }

  .pinned-name {
    min-width: 0;
    overflow: hidden;
    color: var(--text);
    font-size: var(--text-sm);
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
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
