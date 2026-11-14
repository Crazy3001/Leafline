<script lang="ts">
  // A link card in a window of its own, to put anywhere on the screen. The header drags the
  // window. Links in the card open the next card here, with a back arrow. In the live card window
  // the pin keeps this card in a window that stays, so the next link opens a new one beside it.
  import { ArrowLeft, Maximize2, Pin, X } from "@lucide/svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount, setContext } from "svelte";

  import { page } from "$app/state";
  import { backend } from "$lib/api";
  import CardContent from "$lib/components/peek/CardContent.svelte";
  import { SECTION_SCOPE, cardWindow, openFull, peek, peekBack, type PeekKind, type PeekTarget } from "$lib/peek.svelte";
  import { display, watchSettings } from "$lib/settings.svelte";
  import { tip } from "$lib/tip.svelte";

  const LABEL: Record<PeekKind, string> = { item: "Item", mob: "Monster", npc: "NPC", map: "Map", quest: "Quest", skill: "Skill" };
  const TAB: Record<PeekKind, string> = { item: "Lookup", mob: "Lookup", npc: "Lookup", map: "Lookup", quest: "Quests", skill: "Lookup" };

  watchSettings();
  // Sections start folded, as in every card, and links page through this window.
  setContext(SECTION_SCOPE, "card");
  cardWindow.active = true;

  /** A card's page path back to what it shows. */
  function parse(path: string): PeekTarget | null {
    const [, kind, key] = path.replace(/^\//, "").split("/");
    if (!kind || !key) return null;
    if (key.startsWith("n-")) return { kind: kind as PeekKind, id: null, name: decodeURIComponent(key.slice(2)) };
    const id = Number(key);
    return Number.isFinite(id) ? { kind: kind as PeekKind, id, name: null } : null;
  }

  const first = parse(`card/${page.params.kind}/${page.params.key}`);
  peek.stack = first ? [first] : [];

  const top = $derived(peek.stack.at(-1) ?? null);
  let live = $state(false);
  let body = $state<HTMLElement | null>(null);

  onMount(() => {
    const win = getCurrentWindow();
    backend.cardIsLive().then((yes) => (live = yes));
    // The live window shows whichever card a link asks for next.
    const stop = win.listen<string>("card-show", (e) => {
      const next = parse(e.payload);
      if (next) peek.stack = [next];
    });
    const key = (e: KeyboardEvent) => {
      if (e.key === "Escape") close();
    };
    window.addEventListener("keydown", key);
    return () => {
      stop.then((f) => f());
      window.removeEventListener("keydown", key);
    };
  });

  // A new card starts at its top.
  $effect(() => {
    void peek.stack.length;
    void top;
    if (body) body.scrollTop = 0;
  });

  function close() {
    getCurrentWindow()
      .close()
      .catch(() => window.close());
  }

  /** Keeps this window where it is, like a pinned panel; the next link opens a new one. */
  async function pin() {
    await backend.pinCard();
    live = false;
  }

  function full() {
    if (!top) return;
    openFull(top);
    close();
  }
</script>

<svelte:head><title>Leafline · {top ? LABEL[top.kind] : "Card"}</title></svelte:head>

<div class="frame" class:pinned={!live} style:opacity={display.opacity}>
  <header data-tauri-drag-region>
    {#if peek.stack.length > 1}
      <button class="icon" aria-label="Back" use:tip={"Back"} onclick={peekBack}><ArrowLeft size={14} /></button>
    {/if}
    {#if top}<span class="kind ref-{top.kind}" data-tauri-drag-region>{LABEL[top.kind]}</span>{/if}
    <span class="spacer" data-tauri-drag-region></span>
    {#if top}
      <button class="full" use:tip={`In the ${TAB[top.kind]} tab`} onclick={full}><Maximize2 size={12} />Full view</button>
    {/if}
    {#if live}
      <button class="icon" aria-label="Pin this card" use:tip={"Pin: keep this card, and open the next one beside it"} onclick={pin}
        ><Pin size={13} /></button
      >
    {/if}
    <button class="icon" aria-label="Close" use:tip={"Close (Esc)"} onclick={close}><X size={14} /></button>
  </header>
  <div class="body" data-peek bind:this={body}>
    {#if top}
      {#key peek.stack.length + ":" + top.kind + ":" + (top.id ?? top.name)}
        <CardContent target={top} />
      {/key}
    {/if}
  </div>
</div>

<style>
  :global(html),
  :global(body) {
    background: transparent;
  }

  .frame {
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg);
    background: var(--surface-raised);
  }

  /* A pinned card keeps a gold edge, as in the overlay. */
  .frame.pinned {
    border-color: color-mix(in srgb, var(--accent) 45%, var(--border-strong));
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
    cursor: grab;
    user-select: none;
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
    align-self: stretch;
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
