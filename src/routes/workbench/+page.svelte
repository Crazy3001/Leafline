<script lang="ts">
  import { FolderOpen, Image as ImageIcon, LoaderCircle, RotateCw, ScanText, ShieldCheck } from "@lucide/svelte";
  import { onMount } from "svelte";

  import { backend, fileName } from "$lib/api";
  import Logo from "$lib/components/Logo.svelte";
  import type { Analysis, HudValues, Rect, ShortcutInfo } from "$lib/types";
  import ImageViewer from "$lib/workbench/ImageViewer.svelte";
  import ResultsPanel from "$lib/workbench/ResultsPanel.svelte";
  import { regionsOf } from "$lib/workbench/regions";

  let images = $state<string[]>([]);
  let selected = $state<string | null>(null);
  let imageUrl = $state<string | null>(null);
  let analysis = $state<Analysis | null>(null);
  let busy = $state(false);
  let error = $state<string | null>(null);
  let showText = $state(false);
  let highlight = $state<Rect | null>(null);
  let shortcuts = $state<ShortcutInfo[]>([]);

  const regions = $derived(analysis ? regionsOf(analysis, showText) : []);

  onMount(() => {
    backend.shortcuts().then((s) => (shortcuts = s));
    backend.takeLaunchImage().then((path) => {
      if (path) openPath(path);
    });
    const unlisten = backend.onOpenImage(openPath);
    return () => unlisten.then((stop) => stop());
  });

  async function openImage() {
    const path = await backend.pickImage();
    if (path) await openPath(path);
  }

  async function openPath(path: string) {
    images = [path];
    await select(path);
  }

  async function openFolder() {
    const folder = await backend.pickFolder();
    if (!folder) return;
    images = await backend.listImages(folder);
    if (images.length) await select(images[0]);
  }

  async function select(path: string) {
    selected = path;
    analysis = null;
    error = null;
    if (imageUrl) URL.revokeObjectURL(imageUrl);
    imageUrl = await backend.imageUrl(path);
    await analyze();
  }

  async function analyze() {
    if (!selected) return;
    const path = selected;
    busy = true;
    error = null;
    try {
      const result = await backend.analyzeImage(path);
      if (selected === path) analysis = result;
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  /** Learns the HUD font from the open screenshot, then reads it again with what was learned. */
  async function calibrate(values: HudValues) {
    if (!selected) throw new Error("No screenshot is open.");
    const result = await backend.calibrateHud(selected, values);
    await analyze();
    return result;
  }

  function step(offset: number) {
    if (!selected || images.length < 2) return;
    const next = images[(images.indexOf(selected) + offset + images.length) % images.length];
    select(next);
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key === "ArrowDown" || event.key === "j") step(1);
    else if (event.key === "ArrowUp" || event.key === "k") step(-1);
    else if (event.key === "r") analyze();
    else return;
    event.preventDefault();
  }
</script>

<svelte:head><title>Leafline — Workbench</title></svelte:head>
<svelte:window onkeydown={onKeydown} />

<div class="app">
  <header class="topbar">
    <div class="brand">
      <Logo size={20} />
      <span class="name">Leafline</span>
      <span class="divider"></span>
      <span class="section">Workbench</span>
    </div>
    <div class="tools">
      <button class="ghost" onclick={openImage}><ImageIcon size={15} />Open image</button>
      <button class="ghost" onclick={openFolder}><FolderOpen size={15} />Open folder</button>
      <label class="toggle" title="Outline every line of text the OCR found">
        <input type="checkbox" bind:checked={showText} disabled={!analysis} />
        <ScanText size={15} />Text boxes
      </label>
      <button class="primary" onclick={analyze} disabled={!selected || busy}>
        {#if busy}<span class="spin"><LoaderCircle size={15} /></span>{:else}<RotateCw size={15} />{/if}
        Analyze
      </button>
    </div>
  </header>

  <aside class="sidebar">
    <div class="eyebrow">Screenshots</div>
    {#if images.length}
      <ul class="files">
        {#each images as path (path)}
          <li>
            <button class:active={path === selected} onclick={() => select(path)} title={path}>
              <ImageIcon size={14} />
              <span>{fileName(path)}</span>
            </button>
          </li>
        {/each}
      </ul>
      {#if images.length > 1}
        <p class="hint">Use <kbd>↑</kbd> <kbd>↓</kbd> to step through, <kbd>R</kbd> to re-run.</p>
      {/if}
    {:else}
      <p class="hint">Open a screenshot or a folder of them to test the readers.</p>
    {/if}

    <div class="spacer"></div>

    {#if shortcuts.length}
      <div class="eyebrow">Hotkeys</div>
      <dl class="shortcuts">
        {#each shortcuts as shortcut (shortcut.action)}
          <div><dt>{shortcut.label}</dt><dd>{shortcut.keys}</dd></div>
        {/each}
      </dl>
    {/if}

    <div class="policy">
      <ShieldCheck size={16} />
      <p>
        <strong>Offline mode.</strong> Leafline reads saved screenshots only. Live capture stays off until Nexon confirms
        read-only overlays are allowed.
      </p>
    </div>
  </aside>

  <main class="canvas">
    {#if imageUrl && analysis}
      <ImageViewer src={imageUrl} width={analysis.width} height={analysis.height} {regions} {highlight} />
    {:else if busy}
      <div class="placeholder"><span class="spin"><LoaderCircle size={22} /></span>Reading screenshot…</div>
    {:else if error}
      <div class="placeholder error">{error}</div>
    {:else}
      <div class="placeholder">
        <Logo size={36} />
        <p>Load a screenshot to see what Leafline reads from it.</p>
        <div class="actions">
          <button class="ghost" onclick={openImage}><ImageIcon size={15} />Open image</button>
          <button class="ghost" onclick={openFolder}><FolderOpen size={15} />Open folder</button>
        </div>
      </div>
    {/if}
    {#if error && analysis}<div class="toast">{error}</div>{/if}
  </main>

  <aside class="results">
    {#if analysis}
      {#key selected}
        <ResultsPanel {analysis} {calibrate} bind:highlight />
      {/key}
    {:else}
      <div class="results-empty">Readings appear here.</div>
    {/if}
  </aside>

  <footer class="statusbar">
    {#if selected}<span>{fileName(selected)}</span>{/if}
    {#if analysis}
      <span class="tabular">{analysis.width} × {analysis.height}</span>
      <span>{analysis.tooltips.length} tooltip{analysis.tooltips.length === 1 ? "" : "s"}</span>
      <span>HUD {analysis.hudLayout ? "found" : "not found"}</span>
    {/if}
    <span class="grow"></span>
    <span>OCR: Windows.Media.Ocr</span>
  </footer>
</div>

<style>
  :global(body) {
    background: var(--bg);
  }

  .app {
    display: grid;
    grid-template: "top top top" 52px "side canvas results" 1fr "status status status" 26px / 240px 1fr 380px;
    height: 100vh;
  }

  .topbar {
    grid-area: top;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 14px 0 16px;
    border-bottom: 1px solid var(--border);
    background: var(--surface);
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 9px;
    font-family: var(--font-display);
  }

  .brand .name {
    font-size: 15px;
    font-weight: 700;
  }

  .divider {
    width: 1px;
    height: 16px;
    background: var(--border-strong);
  }

  .section {
    color: var(--muted);
  }

  .tools {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  button,
  .toggle {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    height: 32px;
    padding: 0 12px;
    border-radius: var(--radius-sm);
    border: 1px solid transparent;
    font-weight: 500;
    cursor: pointer;
  }

  .ghost,
  .toggle {
    background: transparent;
    border-color: var(--border-strong);
    color: var(--text);
  }

  .ghost:hover,
  .toggle:hover {
    background: var(--surface-2);
  }

  .toggle input {
    accent-color: var(--accent);
    margin: 0;
  }

  .toggle:has(input:disabled) {
    opacity: 0.45;
    cursor: default;
  }

  .primary {
    background: var(--accent);
    color: #1d1405;
    font-weight: 600;
  }

  .primary:hover:not(:disabled) {
    background: var(--accent-strong);
  }

  button:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .spin {
    display: inline-flex;
    animation: spin 0.9s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .sidebar {
    grid-area: side;
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-height: 0;
    padding: 14px 10px 12px;
    border-right: 1px solid var(--border);
    background: var(--surface);
  }

  .sidebar > .eyebrow {
    padding: 0 6px;
  }

  .files {
    margin: 0;
    padding: 0;
    list-style: none;
    overflow-y: auto;
  }

  .files button {
    width: 100%;
    height: 30px;
    padding: 0 8px;
    color: var(--muted);
    background: none;
    text-align: left;
  }

  .files button span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .files button:hover {
    background: var(--surface-2);
    color: var(--text);
  }

  .files button.active {
    background: var(--accent-soft);
    color: var(--accent-strong);
  }

  .hint {
    margin: 0;
    padding: 0 6px;
    color: var(--faint);
    font-size: 12px;
  }

  kbd {
    padding: 0 4px;
    border: 1px solid var(--border-strong);
    border-bottom-width: 2px;
    border-radius: 4px;
    font-family: var(--font-sans);
    font-size: 10.5px;
  }

  .spacer {
    flex: 1;
  }

  .shortcuts {
    display: grid;
    gap: 4px;
    margin: 0 0 6px;
    padding: 0 6px;
  }

  .shortcuts div {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    font-size: 11.5px;
  }

  .shortcuts dt {
    color: var(--muted);
  }

  .shortcuts dd {
    margin: 0;
    color: var(--faint);
    font-family: var(--font-mono);
    font-size: 10.5px;
    white-space: nowrap;
  }

  .policy {
    display: flex;
    gap: 9px;
    padding: 10px;
    border-radius: var(--radius);
    background: var(--good-soft);
    color: var(--good);
  }

  .policy :global(svg) {
    flex: none;
    margin-top: 1px;
  }

  .policy p {
    margin: 0;
    color: var(--muted);
    font-size: 11.5px;
  }

  .policy strong {
    color: var(--good);
  }

  .canvas {
    grid-area: canvas;
    position: relative;
    min-width: 0;
    min-height: 0;
    background:
      radial-gradient(circle at 50% 40%, rgba(243, 177, 63, 0.04), transparent 60%),
      var(--bg);
  }

  .placeholder {
    display: grid;
    place-content: center;
    justify-items: center;
    gap: 10px;
    height: 100%;
    color: var(--muted);
    text-align: center;
  }

  .placeholder p {
    margin: 0;
  }

  .placeholder .actions {
    display: flex;
    gap: 8px;
    margin-top: 4px;
  }

  .placeholder.error {
    padding: 40px;
    color: var(--bad);
  }

  .toast {
    position: absolute;
    left: 50%;
    bottom: 16px;
    transform: translateX(-50%);
    padding: 8px 12px;
    border-radius: var(--radius-sm);
    background: rgba(239, 79, 90, 0.15);
    color: var(--bad);
  }

  .results {
    grid-area: results;
    min-height: 0;
    border-left: 1px solid var(--border);
    background: var(--surface);
  }

  .results-empty {
    display: grid;
    place-items: center;
    height: 100%;
    color: var(--faint);
  }

  .statusbar {
    grid-area: status;
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 0 14px;
    border-top: 1px solid var(--border);
    background: var(--surface);
    color: var(--faint);
    font-size: 11.5px;
  }

  .grow {
    flex: 1;
  }
</style>
