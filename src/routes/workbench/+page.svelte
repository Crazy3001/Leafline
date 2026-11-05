<script lang="ts">
  import {
    Check,
    FolderOpen,
    History,
    Image as ImageIcon,
    LoaderCircle,
    Play,
    RotateCw,
    ScanText,
    Settings,
    ShieldCheck,
    Square,
  } from "@lucide/svelte";
  import { onMount } from "svelte";

  import { backend, fileName } from "$lib/api";
  import Logo from "$lib/components/Logo.svelte";
  import { formatDuration } from "$lib/format";
  import { isSaved, watchHistory } from "$lib/history.svelte";
  import type {
    Analysis,
    Rect,
    ReplayOrder,
    ReplayProgress,
    SessionSummary,
  } from "$lib/types";
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

  // Session replay: the open folder run as one play session.
  let folder = $state<string | null>(null);
  let order = $state<ReplayOrder["by"]>("modified");
  let interval = $state(1);
  let replaying = $state(false);
  let progress = $state<{ done: number; total: number } | null>(null);
  let session = $state<SessionSummary | null>(null);

  const regions = $derived(analysis ? regionsOf(analysis, showText) : []);
  let saving = $state(false);

  /** Keeps the replayed session, so the Train tab can compare maps across sessions. */
  async function saveSession() {
    saving = true;
    try {
      await backend.saveSession(folder ? fileName(folder) : "");
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }

  onMount(() => {
    watchHistory();
    backend.takeLaunchImage().then((path) => {
      if (path) openPath(path);
    });
    const subscriptions = [backend.onOpenImage(openPath), backend.onReplayProgress(showProgress)];
    return () => subscriptions.forEach((s) => s.then((stop) => stop()));
  });

  async function openImage() {
    const path = await backend.pickImage();
    if (path) await openPath(path);
  }

  async function openPath(path: string) {
    if (replaying) return;
    folder = null;
    session = null;
    images = [path];
    await select(path);
  }

  async function openFolder() {
    if (replaying) return;
    const picked = await backend.pickFolder();
    if (!picked) return;
    folder = picked;
    session = null;
    images = await backend.listImages(picked);
    if (images.length) await select(images[0]);
  }

  async function replay() {
    if (!folder || replaying) return;
    replaying = true;
    error = null;
    session = null;
    progress = { done: 0, total: images.length };
    const replayOrder: ReplayOrder =
      order === "name" ? { by: "name", intervalSeconds: Math.max(0.1, interval || 1) } : { by: "modified" };
    try {
      session = await backend.replaySession(folder, replayOrder);
    } catch (e) {
      error = String(e);
    } finally {
      replaying = false;
      progress = null;
    }
  }

  /** Shows each replayed frame as it is read, without analysing it again. */
  async function showProgress(update: ReplayProgress) {
    progress = { done: update.done, total: update.total };
    session = update.session;
    if (selected !== update.path) {
      selected = update.path;
      const url = await backend.imageUrl(update.path);
      if (imageUrl) URL.revokeObjectURL(imageUrl);
      imageUrl = url;
    }
    analysis = update.analysis;
  }

  async function select(path: string) {
    if (replaying) return;
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

  function step(offset: number) {
    if (!selected || images.length < 2) return;
    const next = images[(images.indexOf(selected) + offset + images.length) % images.length];
    select(next);
  }

  function onKeydown(event: KeyboardEvent) {
    if (replaying || event.target instanceof HTMLInputElement) return;
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
      <button class="ghost icon" title="Settings" aria-label="Settings" onclick={() => backend.showSettings()}>
        <Settings size={15} />
      </button>
      <button class="primary" onclick={analyze} disabled={!selected || busy || replaying}>
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
            <button
              class:active={path === selected}
              onclick={() => select(path)}
              title={path}
              disabled={replaying && path !== selected}
            >
              <ImageIcon size={14} />
              <span>{fileName(path)}</span>
            </button>
          </li>
        {/each}
      </ul>
      {#if images.length > 1}
        <p class="hint">Use <kbd>↑</kbd> <kbd>↓</kbd> to step through, <kbd>R</kbd> to re-run.</p>
      {/if}
      {#if folder && images.length > 1}
        <section class="replay">
          <div class="eyebrow">Session replay</div>
          <p class="hint">Read the whole folder as one play session to see EXP and meso rates.</p>
          <div class="segmented" role="radiogroup" aria-label="Frame order">
            <button
              role="radio"
              aria-checked={order === "modified"}
              disabled={replaying}
              onclick={() => (order = "modified")}
              title="Order by when each file was saved, and use that as its time">File time</button
            >
            <button
              role="radio"
              aria-checked={order === "name"}
              disabled={replaying}
              onclick={() => (order = "name")}
              title="Order by file name, a fixed time apart: for frames taken from a recording">File name</button
            >
          </div>
          {#if order === "name"}
            <label class="interval">
              Frames
              <input type="number" min="0.1" step="0.1" bind:value={interval} disabled={replaying} />
              s apart
            </label>
          {/if}
          {#if replaying && progress}
            <div class="meter" role="progressbar" aria-valuenow={progress.done} aria-valuemax={progress.total}>
              <span style:width="{(progress.done / Math.max(1, progress.total)) * 100}%"></span>
            </div>
            <div class="replay-row">
              <span class="tabular">Frame {progress.done} of {progress.total}</span>
              <button class="ghost small" onclick={() => backend.cancelReplay()}><Square size={11} />Stop</button>
            </div>
          {:else}
            <button class="ghost wide" onclick={replay}><Play size={14} />Replay as session</button>
            {#if session}
              <p class="hint">
                Last run: {session.frames} frames over {formatDuration(session.elapsedSeconds)}, {formatDuration(
                  session.activeSeconds,
                )} active.
              </p>
              {#if session.activeSeconds > 0}
                {#if isSaved(session)}
                  <p class="hint saved"><Check size={12} /> Saved to your history.</p>
                {:else}
                  <button class="ghost wide" onclick={saveSession} disabled={saving}
                    ><History size={14} />Save to history</button
                  >
                {/if}
              {/if}
            {/if}
          {/if}
        </section>
      {/if}
    {:else}
      <p class="hint">Open a screenshot or a folder of them to test the readers.</p>
    {/if}

    <div class="spacer"></div>

    <div class="policy">
      <ShieldCheck size={16} />
      <p>
        <strong>Offline mode.</strong> The workbench reads saved screenshots. Live reading is opt-in, at your own risk,
        and with hotkeys, map data and more is in
        <button class="inline" onclick={() => backend.showSettings()}>Settings</button>.
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
        <ResultsPanel {analysis} {session} bind:highlight />
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
      <span>Map {analysis.map ? "found" : "not found"}</span>
    {/if}
    {#if replaying && progress}<span class="replaying">Replaying {progress.done} / {progress.total}</span>{/if}
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
    font-size: var(--text-lg);
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
    overflow-y: auto;
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
    font-size: var(--text-sm);
  }

  kbd {
    padding: 0 4px;
    border: 1px solid var(--border-strong);
    border-bottom-width: 2px;
    border-radius: var(--radius-sm);
    font-family: var(--font-sans);
    font-size: var(--text-xs);
  }

  .replay {
    display: grid;
    gap: 8px;
    margin-top: 6px;
    padding: 12px 6px 0;
    border-top: 1px solid var(--border);
  }

  .segmented {
    display: grid;
    grid-template-columns: 1fr 1fr;
    padding: 2px;
    border-radius: var(--radius-sm);
    background: var(--bg);
    border: 1px solid var(--border);
  }

  .segmented button {
    height: 26px;
    justify-content: center;
    padding: 0 6px;
    border-radius: var(--radius-sm);
    background: none;
    color: var(--muted);
    font-size: var(--text-sm);
  }

  .segmented button[aria-checked="true"] {
    background: var(--surface-3);
    color: var(--text);
  }

  .interval {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--muted);
    font-size: var(--text-sm);
  }

  .interval input {
    width: 56px;
    height: 26px;
    padding: 0 6px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--bg);
    color: var(--text);
    font: inherit;
  }

  .wide {
    justify-content: center;
    width: 100%;
  }

  .small {
    height: 24px;
    padding: 0 8px;
    gap: 5px;
    font-size: var(--text-sm);
  }

  .meter {
    height: 4px;
    border-radius: var(--radius-sm);
    background: rgba(255, 255, 255, 0.06);
    overflow: hidden;
  }

  .meter span {
    display: block;
    height: 100%;
    background: var(--accent);
    transition: width 0.2s ease;
  }

  .replay-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    color: var(--muted);
    font-size: var(--text-sm);
  }

  .replaying {
    color: var(--accent);
  }

  .spacer {
    flex: 1;
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
    font-size: var(--text-sm);
  }

  .policy strong {
    color: var(--good);
  }

  .saved {
    display: flex;
    align-items: center;
    gap: 5px;
    color: var(--good);
  }

  .inline {
    height: auto;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--text);
    font-size: inherit;
    text-decoration: underline;
    cursor: pointer;
  }

  .ghost.icon {
    padding: 0 8px;
  }

  .canvas {
    grid-area: canvas;
    position: relative;
    min-width: 0;
    min-height: 0;
    background: var(--bg);
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
    font-size: var(--text-sm);
  }

  .grow {
    flex: 1;
  }
</style>
