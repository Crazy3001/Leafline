<script lang="ts">
  // One panel pinned into a small window of its own. It stays on screen while the overlay is
  // hidden and sizes itself to what it shows, until the player resizes it by hand.
  import { AppWindow, Minimize2, PinOff } from "@lucide/svelte";
  import { onMount, tick, untrack } from "svelte";

  import { page } from "$app/state";
  import { backend } from "$lib/api";
  import NextStep from "$lib/components/NextStep.svelte";
  import PanelView from "$lib/components/PanelView.svelte";
  import { PIN_MAX_HEIGHT, PIN_WIDTH, pinId, pinInfo } from "$lib/panels";
  import { live, watchLive } from "$lib/readings.svelte";
  import { watchTyping } from "$lib/typing";
  import { display, watchSettings } from "$lib/settings.svelte";
  import type { PinId } from "$lib/types";
  import { followTrip } from "$lib/wayfinder.svelte";

  const panel = $derived<PinId>(pinId(page.params.panel) ?? "character");
  /** Taller than this, the panel scrolls instead of growing. */
  const maxHeight = $derived(PIN_MAX_HEIGHT[panel]);
  const info = $derived(pinInfo(panel));
  const Icon = $derived(info.icon);

  let frame = $state<HTMLElement>();
  let body = $state<HTMLElement>();
  let content = $state<HTMLElement>();

  /** A size set by hand, in the page's own units so it follows the size setting. */
  let sized = $state<{ width: number; height: number } | null>(storedSize());
  /** Content taller than the panel's cap: the body scrolls. Otherwise it never shows a scrollbar,
   *  because the window can come out a fraction of a pixel short of the content (display scaling
   *  rounds it), and a scrollbar narrows the content, which refits the window, which can drop the
   *  scrollbar again: the content then shakes sideways while the window keeps its size. */
  let capped = $state(false);
  /** The last size asked for, so an unchanged fit doesn't set the window again. */
  let lastFit = "";

  function setWindow(width: number, height: number) {
    const key = `${width}x${height}`;
    if (key === lastFit) return;
    lastFit = key;
    backend.fitWindow(width, height);
  }

  function storedSize() {
    try {
      return JSON.parse(localStorage.getItem(sizeKey()) ?? "null");
    } catch {
      return null;
    }
  }

  function sizeKey() {
    return `leafline.pin.size.${panel}`;
  }

  function keepSize(size: typeof sized) {
    sized = size;
    // The window was sized by hand: whatever was last asked for no longer holds.
    lastFit = "";
    try {
      if (size) localStorage.setItem(sizeKey(), JSON.stringify(size));
      else localStorage.removeItem(sizeKey());
    } catch {
      // A convenience only.
    }
  }

  /** Back to following the content. It is measured once the page has let go of the set height. */
  async function fitToContent() {
    keepSize(null);
    await tick();
    fit();
  }

  /** Sizes the window to the content, or to the player's size if they set one. The window is sized in screen units, the page in zoomed ones. */
  function fit() {
    if (sized) {
      setWindow(sized.width * display.scale, sized.height * display.scale);
      return;
    }
    if (!frame || !body) return;
    const natural = Math.ceil(frame.offsetHeight - body.clientHeight + body.scrollHeight);
    capped = natural > maxHeight;
    setWindow(PIN_WIDTH * display.scale, Math.min(maxHeight, natural) * display.scale);
  }

  // Clicks leave the game focused; typing in a box takes focus while it lasts.
  onMount(() => watchTyping());

  onMount(() => {
    watchLive();
    watchSettings();
    if (panel === "route") followTrip();
    const resized = backend.onPinResized(([width, height]) =>
      keepSize({ width: width / display.scale, height: height / display.scale }),
    );
    // Follow the content: grow and shrink the window as readings change. Not while it has a set
    // size, which a drag in progress would otherwise be pulled back to.
    const observer = new ResizeObserver(() => {
      if (!sized) fit();
    });
    if (content) observer.observe(content);
    return () => {
      observer.disconnect();
      resized.then((stop) => stop());
    };
  });

  // A new size setting changes the zoom but not the page's own measurements.
  $effect(() => {
    void display.scale;
    untrack(fit);
  });
</script>

<svelte:head><title>Leafline · {info.title}</title></svelte:head>

<div
  class="frame"
  class:locked={live.status.clickThrough}
  class:sized
  style:opacity={display.opacity * (live.status.clickThrough ? 0.85 : 1)}
  bind:this={frame}
>
  <header data-tauri-drag-region>
    <span class="title" data-tauri-drag-region><Icon size={12} />{info.title}</span>
    <span class="actions">
      {#if sized}
        <button class="action" title="Fit to contents" onclick={fitToContent}><Minimize2 size={12} /></button>
      {/if}
      <button
        class="action"
        title={panel === "route" ? "Open the Map tab" : `Open the ${info.title} tab`}
        onclick={() => backend.showPanel(panel)}
        ><AppWindow size={12} /></button
      >
      <button class="action" title="Unpin" onclick={() => backend.unpinPanel(panel)}><PinOff size={12} /></button>
    </span>
  </header>
  <div class="body" class:scrolls={capped} bind:this={body}>
    <div bind:this={content}>
      {#if panel === "route"}
        <NextStep variant="pin" />
      {:else}
        <PanelView {panel} analysis={live.analysis} session={live.session} pinned />
      {/if}
    </div>
  </div>
</div>

<style>
  :global(html),
  :global(body) {
    background: transparent;
    overflow: hidden;
  }

  .frame {
    display: flex;
    flex-direction: column;
    max-height: 100vh;
    background: var(--glass);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    overflow: hidden;
    transition: opacity 0.2s ease;
  }

  .frame.sized {
    height: 100vh;
  }

  .sized .body {
    flex: 1;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 24px;
    padding: 0 4px 0 10px;
    user-select: none;
    cursor: grab;
  }

  .title {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: var(--text-xs);
    font-weight: 600;
    color: var(--faint);
  }

  .actions {
    display: inline-flex;
  }

  .action {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--faint);
    opacity: 0;
    cursor: pointer;
    transition: opacity 0.15s ease;
  }

  .frame:hover .action,
  .action:focus-visible {
    opacity: 1;
  }

  .action:hover {
    background: rgba(255, 255, 255, 0.08);
    color: var(--text);
  }

  .locked .action {
    display: none;
  }

  .body {
    min-height: 0;
    padding: 2px 10px 10px;
    overflow-y: hidden;
  }

  .body.scrolls,
  .sized .body {
    overflow-y: auto;
  }
</style>
