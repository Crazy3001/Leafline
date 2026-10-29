<script lang="ts">
  import { Lock, LockOpen, PanelsTopLeft, X } from "@lucide/svelte";
  import { onMount } from "svelte";

  import { backend } from "$lib/api";
  import CharacterCard from "$lib/components/CharacterCard.svelte";
  import ItemCard from "$lib/components/ItemCard.svelte";
  import MapCard from "$lib/components/MapCard.svelte";
  import Logo from "$lib/components/Logo.svelte";
  import QuestCard from "$lib/components/QuestCard.svelte";
  import SessionCard from "$lib/components/SessionCard.svelte";
  import type { Analysis, OverlayStatus, SessionSummary } from "$lib/types";

  let analysis = $state<Analysis | null>(null);
  let session = $state<SessionSummary | null>(null);
  let status = $state<OverlayStatus>({ visible: true, clickThrough: false });

  onMount(() => {
    backend.overlayStatus().then((s) => (status = s));
    const subscriptions = [
      backend.onAnalysis((a) => (analysis = a)),
      backend.onSession((s) => (session = s)),
      backend.onOverlayStatus((s) => (status = s)),
    ];
    return () => subscriptions.forEach((s) => s.then((unlisten) => unlisten()));
  });

  async function toggleLock() {
    await backend.setClickThrough(!status.clickThrough);
    status.clickThrough = !status.clickThrough;
  }
</script>

<svelte:head><title>Leafline</title></svelte:head>

<div class="shell" class:locked={status.clickThrough}>
  <header data-tauri-drag-region>
    <span class="brand" data-tauri-drag-region>
      <Logo size={16} />
      <span data-tauri-drag-region>Leafline</span>
      <span class="mode" data-tauri-drag-region>Offline</span>
    </span>
    <span class="actions">
      <button title="Open workbench (Ctrl+Alt+W)" onclick={() => backend.showWorkbench()}>
        <PanelsTopLeft size={14} />
      </button>
      <button
        title={status.clickThrough ? "Unlock with Ctrl+Alt+L" : "Lock: clicks pass through to the game (Ctrl+Alt+L)"}
        class:active={status.clickThrough}
        onclick={toggleLock}
      >
        {#if status.clickThrough}<Lock size={14} />{:else}<LockOpen size={14} />{/if}
      </button>
      <button title="Hide (Ctrl+Alt+O)" onclick={() => backend.hideOverlay()}>
        <X size={14} />
      </button>
    </span>
  </header>

  <main>
    {#if analysis}
      <CharacterCard hud={analysis.hud} mesos={analysis.mesos} {session} />
      {#if analysis.map}
        <MapCard map={analysis.map} {session} />
      {/if}
      {#if session && session.maps.length > 1}
        <SessionCard {session} compact />
      {/if}
      {#each analysis.tooltips as item, i}
        <ItemCard {item} title={i === 0 ? "Item" : "Compared item"} />
      {/each}
      {#if analysis.quest}
        <QuestCard quest={analysis.quest} />
      {/if}
    {:else}
      <div class="empty">
        <Logo size={28} />
        <p>Nothing to show yet.</p>
        <p class="hint">
          Open the workbench with <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>W</kbd> and load a screenshot. Its readings
          appear here, just as they will during play.
        </p>
      </div>
    {/if}
  </main>
</div>

<style>
  :global(html),
  :global(body) {
    background: transparent;
    overflow: hidden;
  }

  .shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--glass);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg);
    overflow: hidden;
    transition: opacity 0.2s ease;
  }

  .shell.locked {
    opacity: 0.82;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 36px;
    padding: 0 6px 0 12px;
    border-bottom: 1px solid var(--border);
    user-select: none;
    cursor: grab;
  }

  .brand {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    font-family: var(--font-display);
    font-weight: 600;
    letter-spacing: 0.01em;
  }

  .mode {
    padding: 1px 6px;
    border-radius: 999px;
    font-family: var(--font-sans);
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.04em;
    color: var(--accent);
    background: var(--accent-soft);
  }

  .actions {
    display: inline-flex;
    gap: 2px;
  }

  .actions button {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--muted);
    cursor: pointer;
  }

  .actions button:hover {
    background: rgba(255, 255, 255, 0.07);
    color: var(--text);
  }

  .actions button.active {
    color: var(--accent);
  }

  main {
    flex: 1;
    display: grid;
    align-content: start;
    gap: 8px;
    padding: 8px;
    overflow-y: auto;
  }

  .empty {
    display: grid;
    justify-items: center;
    gap: 4px;
    padding: 40px 20px;
    text-align: center;
    color: var(--muted);
  }

  .empty p {
    margin: 0;
  }

  .hint {
    max-width: 240px;
    font-size: 12px;
    color: var(--faint);
  }

  kbd {
    padding: 0 4px;
    border: 1px solid var(--border-strong);
    border-bottom-width: 2px;
    border-radius: 4px;
    font-family: var(--font-sans);
    font-size: 10.5px;
    color: var(--muted);
  }
</style>
