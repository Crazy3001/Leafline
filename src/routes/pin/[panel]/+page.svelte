<script lang="ts">
  // One panel pinned into a small window of its own. It stays on screen while the overlay is
  // hidden and sizes itself to what it shows.
  import { PinOff } from "@lucide/svelte";
  import { onMount } from "svelte";

  import { page } from "$app/state";
  import { backend } from "$lib/api";
  import PanelView from "$lib/components/PanelView.svelte";
  import { PANELS, panelInfo } from "$lib/panels";
  import { live, watchLive } from "$lib/readings.svelte";
  import { display, watchSettings } from "$lib/settings.svelte";
  import type { Panel } from "$lib/types";

  /** Taller than this, the panel scrolls instead of growing. */
  const MAX_HEIGHT = 560;

  const panel = $derived(
    (PANELS.some((p) => p.id === page.params.panel) ? page.params.panel : "stats") as Panel,
  );
  const info = $derived(panelInfo(panel));
  const Icon = $derived(info.icon);

  let frame = $state<HTMLElement>();
  let body = $state<HTMLElement>();
  let content = $state<HTMLElement>();

  /** Sizes the window to the content. The window is sized in screen units, the page in zoomed ones. */
  function fit() {
    if (!frame || !body) return;
    const natural = frame.offsetHeight - body.clientHeight + body.scrollHeight;
    backend.fitWindow(info.pinWidth * display.scale, Math.min(MAX_HEIGHT, Math.ceil(natural)) * display.scale);
  }

  onMount(() => {
    watchLive();
    watchSettings();
    if (!content) return;
    // Follow the content: grow and shrink the window as readings change.
    const observer = new ResizeObserver(fit);
    observer.observe(content);
    return () => observer.disconnect();
  });

  // A new size setting changes the zoom but not the page's own measurements.
  $effect(() => {
    void display.scale;
    fit();
  });
</script>

<svelte:head><title>Leafline · {info.title}</title></svelte:head>

<div
  class="frame"
  class:locked={live.status.clickThrough}
  style:opacity={display.opacity * (live.status.clickThrough ? 0.85 : 1)}
  bind:this={frame}
>
  <header data-tauri-drag-region>
    <span class="title" data-tauri-drag-region><Icon size={12} />{info.title}</span>
    <button class="unpin" title="Unpin" onclick={() => backend.unpinPanel(panel)}><PinOff size={12} /></button>
  </header>
  <div class="body" bind:this={body}>
    <div bind:this={content}>
      <PanelView {panel} analysis={live.analysis} session={live.session} pinned />
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
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--faint);
  }

  .unpin {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    border: 0;
    border-radius: 4px;
    background: transparent;
    color: var(--faint);
    opacity: 0;
    cursor: pointer;
    transition: opacity 0.15s ease;
  }

  .frame:hover .unpin,
  .unpin:focus-visible {
    opacity: 1;
  }

  .unpin:hover {
    background: rgba(255, 255, 255, 0.08);
    color: var(--text);
  }

  .locked .unpin {
    display: none;
  }

  .body {
    min-height: 0;
    padding: 2px 10px 10px;
    overflow-y: auto;
  }
</style>
