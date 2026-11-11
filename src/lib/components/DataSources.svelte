<script lang="ts">
  // Every download in one place: the map data, the drop table, and taxis and return scrolls when
  // the player turned travel on. Nothing downloads until asked, and nothing read from the game is
  // sent anywhere.
  import { Download, LoaderCircle, RefreshCw } from "@lucide/svelte";
  import { onMount } from "svelte";

  import { backend } from "$lib/api";
  import { DROPS_URL, OSMS_URL, ago, downloadMapData, mapData, watchMapData } from "$lib/mapdata.svelte";
  import { display } from "$lib/settings.svelte";
  import type { TravelInfo } from "$lib/types";
  import Button from "$lib/ui/Button.svelte";

  let { travel = $bindable(null) }: { travel?: TravelInfo | null } = $props();

  watchMapData();

  let dropsAt = $state<number | null>(null);
  let dropsBusy = $state(false);
  let travelBusy = $state(false);
  let problem = $state<string | null>(null);

  onMount(() => {
    loadDrops();
  });

  function loadDrops() {
    backend
      .dropsFetchedAt()
      .then((at) => (dropsAt = at))
      .catch(() => (dropsAt = null));
  }

  const status = $derived(mapData.status);
  const maps = $derived(status?.files.find((f) => f.name === "maps.json")?.records);
  const mapsReady = $derived(status?.ready ?? false);
  const travelOn = $derived(display.travel.enabled);
  const busy = $derived(mapData.downloading || dropsBusy || travelBusy);

  async function getDrops() {
    dropsBusy = true;
    problem = null;
    try {
      await backend.downloadDrops(true);
      loadDrops();
    } catch (e) {
      problem = `Couldn't download the drop table: ${e}`;
    } finally {
      dropsBusy = false;
    }
  }

  async function getTravel() {
    travelBusy = true;
    problem = null;
    try {
      travel = await backend.downloadTravel();
    } catch (e) {
      problem = `Couldn't download taxis and scrolls: ${e}`;
    } finally {
      travelBusy = false;
    }
  }

  /** Checks everything already downloaded, plus the map data if it never was. */
  async function updateAll() {
    await downloadMapData(true);
    if (dropsAt != null) await getDrops();
    if (travelOn && travel && travel.origin !== "none") await getTravel();
  }
</script>

<div class="sources">
  <div class="source">
    <div class="about">
      <span class="name">Map data</span>
      <span class="help">
        Maps, monsters, NPCs, quests, skills and items for every tab, from
        <button class="link" onclick={() => backend.openLink(OSMS_URL)}>OSMS Data Explorer</button>. About 2 MB.
      </span>
      <span class="state tabular">
        {#if mapsReady}{maps} maps · checked {ago(status?.checkedAt)}{:else}Not downloaded yet{/if}
      </span>
    </div>
    <Button variant={mapsReady ? "secondary" : "primary"} disabled={busy} onclick={() => downloadMapData(mapsReady)}>
      {#if mapData.downloading}<span class="spin"><LoaderCircle size={13} /></span>{:else if mapsReady}<RefreshCw size={13} />{:else}<Download
          size={13}
        />{/if}
      {mapsReady ? "Check" : "Download"}
    </Button>
  </div>

  <div class="source">
    <div class="about">
      <span class="name">Drop table</span>
      <span class="help">
        Which monsters drop which items, from the game's own item text,
        <button class="link" onclick={() => backend.openLink(DROPS_URL)}>StrategyWiki</button> and players' reports.
        Checked once a day.
      </span>
      <span class="state tabular">{dropsAt != null ? `Downloaded ${ago(dropsAt)}` : "Not downloaded yet"}</span>
    </div>
    <Button disabled={busy || !mapsReady} title={mapsReady ? undefined : "Needs the map data first"} onclick={getDrops}>
      {#if dropsBusy}<span class="spin"><LoaderCircle size={13} /></span>{:else if dropsAt != null}<RefreshCw size={13} />{:else}<Download
          size={13}
        />{/if}
      {dropsAt != null ? "Check" : "Download"}
    </Button>
  </div>

  {#if travelOn}
    <div class="source">
      <div class="about">
        <span class="name">Taxis and return scrolls</span>
        <span class="help">For routes, from what players recorded on mapleclassic.wiki.</span>
        <span class="state">
          {#if travel?.origin === "custom"}Your own copy is in use{:else if travel?.origin === "wiki"}Downloaded{:else}Not downloaded yet{/if}
        </span>
      </div>
      <Button disabled={busy} onclick={getTravel}>
        {#if travelBusy}<span class="spin"><LoaderCircle size={13} /></span>{:else if travel?.origin === "none" || !travel}<Download
            size={13}
          />{:else}<RefreshCw size={13} />{/if}
        {travel?.origin === "none" || !travel ? "Download" : "Check"}
      </Button>
    </div>
  {/if}

  <div class="footer">
    <span class="help">
      {#if mapData.error}<span class="warn">{mapData.error}</span>
      {:else if problem}<span class="warn">{problem}</span>
      {:else if mapData.message}{mapData.message}
      {:else}Only these lists are downloaded. Nothing read from your game leaves this PC.{/if}
    </span>
    {#if mapsReady}<Button variant="ghost" disabled={busy} onclick={updateAll}><RefreshCw size={13} />Check all</Button>{/if}
  </div>
</div>

<style>
  .sources {
    display: flex;
    flex-direction: column;
    border: 1px solid var(--border);
    background: var(--surface);
  }

  .source {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
    padding: var(--space-3);
    border-bottom: 1px solid var(--border);
  }

  .about {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .name {
    font-size: var(--text-md);
    font-weight: 600;
  }

  .help {
    font-size: var(--text-xs);
    color: var(--muted);
  }

  .state {
    font-size: var(--text-xs);
    color: var(--faint);
  }

  .footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
  }

  .warn {
    color: var(--warn);
  }

  .link {
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--accent-strong);
    font: inherit;
    cursor: pointer;
  }

  .link:hover {
    text-decoration: underline;
  }

  .spin {
    display: inline-grid;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .spin {
      animation: none;
    }
  }
</style>
