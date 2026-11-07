<script lang="ts">
  // The explicit "Download map data" action, with where the data comes from.
  import { Database, LoaderCircle, RefreshCw } from "@lucide/svelte";

  import { backend } from "$lib/api";
  import { OSMS_CREDIT, OSMS_URL, ago, downloadMapData, mapData, watchMapData } from "$lib/mapdata.svelte";

  let { compact = false }: { compact?: boolean } = $props();

  watchMapData();

  const status = $derived(mapData.status);
  const maps = $derived(status?.files.find((f) => f.name === "maps.json")?.records);
  const updated = $derived(
    status?.dataUpdatedAt ? new Date(status.dataUpdatedAt).toLocaleDateString(undefined, { dateStyle: "medium" }) : null,
  );
</script>

<div class="panel" class:compact>
  {#if status?.ready}
    <p class="state">
      <Database size={13} />
      <span><strong class="tabular">{maps}</strong> maps · data from {updated} · checked {ago(status.checkedAt)}</span>
    </p>
    <button class="action" onclick={() => downloadMapData(true)} disabled={mapData.downloading}>
      {#if mapData.downloading}<span class="spin"><LoaderCircle size={13} /></span>Checking…{:else}<RefreshCw
          size={13}
        />Check for updates{/if}
    </button>
  {:else}
    <p class="explain">Maps, monsters, quests and items need the community map data: about 2 MB, downloaded once.</p>
    <button class="action primary" onclick={() => downloadMapData(false)} disabled={mapData.downloading}>
      {#if mapData.downloading}<span class="spin"><LoaderCircle size={13} /></span>Downloading…{:else}<Database
          size={13}
        />Download map data{/if}
    </button>
  {/if}
  {#if mapData.error}<p class="error">{mapData.error}</p>{/if}
  {#if mapData.message && !compact}<p class="message">{mapData.message}</p>{/if}
  {#if !compact}
    <button class="credit" onclick={() => backend.openLink(OSMS_URL)}>{OSMS_CREDIT}</button>
  {/if}
</div>

<style>
  .panel {
    display: grid;
    gap: 8px;
  }

  .state,
  .explain {
    display: flex;
    gap: 6px;
    margin: 0;
    font-size: var(--text-sm);
    color: var(--muted);
  }

  .state :global(svg) {
    flex: none;
    margin-top: 2px;
    color: var(--muted);
  }

  .state strong {
    color: var(--text);
    font-weight: 600;
  }

  .explain {
    color: var(--faint);
  }

  .action {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 7px;
    height: var(--control-h);
    padding: 0 10px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: transparent;
    font-size: var(--text-sm);
    font-weight: 500;
    cursor: pointer;
  }

  .action:hover:not(:disabled) {
    background: var(--surface-2);
  }

  /* The one main action where it shows, so the primary button style. */
  .action.primary {
    border-color: var(--accent);
    background: var(--accent);
    color: var(--bg);
    font-weight: 600;
  }

  .action.primary:hover:not(:disabled) {
    border-color: var(--accent-strong);
    background: var(--accent-strong);
  }

  .action:disabled {
    color: var(--muted);
    cursor: default;
  }

  .compact .action {
    justify-self: start;
  }

  .spin {
    display: inline-grid;
    animation: spin 0.9s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .error,
  .message {
    margin: 0;
    font-size: var(--text-sm);
  }

  .error {
    color: var(--bad);
  }

  .message {
    color: var(--good);
  }

  .credit {
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--faint);
    font-size: var(--text-xs);
    line-height: 1.4;
    text-align: left;
    text-decoration: underline;
    text-decoration-color: var(--border-strong);
    cursor: pointer;
  }

  .credit:hover {
    color: var(--muted);
  }
</style>
