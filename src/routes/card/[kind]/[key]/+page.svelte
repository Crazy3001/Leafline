<script lang="ts">
  // A link card popped out into a small window of its own, to put anywhere on the screen. The
  // header drags the window; the cross closes it. Links in it open cards inside this window.
  import { X } from "@lucide/svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { setContext } from "svelte";

  import { page } from "$app/state";
  import CardContent from "$lib/components/peek/CardContent.svelte";
  import { SECTION_SCOPE, type PeekKind, type PeekTarget } from "$lib/peek.svelte";
  import { display, watchSettings } from "$lib/settings.svelte";

  const LABEL: Record<PeekKind, string> = { item: "Item", mob: "Monster", npc: "NPC", map: "Map", quest: "Quest", skill: "Skill" };

  watchSettings();
  // Sections start folded, as in every card.
  setContext(SECTION_SCOPE, "card");

  const target = $derived.by((): PeekTarget => {
    const kind = (page.params.kind ?? "item") as PeekKind;
    const key = page.params.key ?? "";
    if (key.startsWith("n-")) return { kind, id: null, name: decodeURIComponent(key.slice(2)) };
    const id = Number(key);
    return { kind, id: Number.isFinite(id) ? id : null, name: null };
  });

  function close() {
    getCurrentWindow()
      .close()
      .catch(() => window.close());
  }
</script>

<svelte:head><title>Leafline · {LABEL[target.kind]}</title></svelte:head>

<div class="frame" style:opacity={display.opacity}>
  <header data-tauri-drag-region>
    <span class="kind ref-{target.kind}" data-tauri-drag-region>{LABEL[target.kind]}</span>
    <span class="spacer" data-tauri-drag-region></span>
    <button class="icon" aria-label="Close" onclick={close}><X size={14} /></button>
  </header>
  <div class="body">
    <CardContent {target} />
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
    border: 1px solid color-mix(in srgb, var(--accent) 45%, var(--border-strong));
    border-radius: var(--radius-lg);
    background: var(--surface-raised);
  }

  header {
    display: flex;
    flex: none;
    align-items: center;
    gap: 6px;
    min-height: 34px;
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

  .spacer {
    flex: 1;
    align-self: stretch;
  }

  .icon {
    display: inline-grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--muted);
    cursor: pointer;
  }

  .icon:hover {
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
