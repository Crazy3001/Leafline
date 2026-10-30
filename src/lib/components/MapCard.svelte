<script lang="ts">
  import { ArrowLeft, MapPin } from "@lucide/svelte";

  import Card from "./Card.svelte";
  import MapDataPanel from "./map/MapDataPanel.svelte";
  import MapDetailsView from "./map/MapDetailsView.svelte";
  import MapPicker from "./map/MapPicker.svelte";
  import { backend } from "$lib/api";
  import { formatCompact, formatDuration, mapLabel } from "$lib/format";
  import { mapData, watchMapData } from "$lib/mapdata.svelte";
  import type { MapDetails, MapMatch, MapReading, SessionSummary } from "$lib/types";

  let {
    map,
    session = null,
    variant = "full",
  }: {
    map: MapReading | null;
    session?: SessionSummary | null;
    /** `full` in the workbench, `compact` in the overlay, `pin` in a pinned window. */
    variant?: "full" | "compact" | "pin";
  } = $props();

  const compact = $derived(variant !== "full");

  watchMapData();

  const here = $derived(map && session ? session.maps.find((m) => m.map === mapLabel(map)) : undefined);
  const ready = $derived(mapData.status?.ready ?? false);

  /** The cached map the minimap header matched. */
  let found = $state<MapMatch | null>(null);
  let looking = $state(false);
  /** Another map opened from the card (a neighbour, a route step, a search), if any. */
  let browsing = $state<MapDetails | null>(null);

  $effect(() => {
    const name = map?.name;
    const street = map?.street ?? null;
    void mapData.status?.dataUpdatedAt;
    browsing = null;
    found = null;
    if (!name || !ready) return;
    looking = true;
    backend
      .findMap(name, street)
      .then((match) => {
        if (map?.name === name) found = match;
      })
      .finally(() => (looking = false));
  });

  async function navigate(id: number) {
    if (found && id === found.card.map.id) {
      browsing = null;
      return;
    }
    browsing = await backend.mapCard(id);
  }

  const shown = $derived(browsing ?? found?.card ?? null);
  // A close but inexact match, or another map with the same name: let the user correct it.
  const doubtful = $derived(
    !!found && !browsing && (found.score < 0.97 || found.alternatives.some((a) => a.name === found!.card.map.name)),
  );
</script>

<Card title="Map" plain={variant === "pin"}>
  {#snippet aside()}
    {#if map?.collapsed && !browsing && variant !== "pin"}<span title="Read from the collapsed minimap's title bar">Minimap title</span>{/if}
  {/snippet}

  {#if browsing}
    <button class="back" onclick={() => (browsing = null)}>
      <ArrowLeft size={12} />
      {found ? `Back to ${found.card.map.name}` : map ? `Back to ${map.name}` : "Back"}
    </button>
  {/if}

  {#if map || shown}
    <div class="where" class:elsewhere={!!browsing}>
      <span class="pin" aria-hidden="true"><MapPin size={15} /></span>
      <div class="names">
        {#if browsing}
          {#if browsing.map.street}<div class="street">{browsing.map.street}</div>{/if}
          <div class="name">{browsing.map.name}</div>
        {:else if found}
          {#if found.card.map.street}<div class="street">{found.card.map.street}</div>{/if}
          <div class="name">{found.card.map.name}</div>
        {:else if map}
          {#if map.street}<div class="street">{map.street}</div>{/if}
          <div class="name">{map.name}</div>
        {/if}
      </div>
    </div>
    {#if here && !browsing && variant !== "pin"}
      <dl class="here">
        <div><dt>Time here</dt><dd class="tabular">{formatDuration(here.activeSeconds)}</dd></div>
        <div><dt>EXP / hr</dt><dd class="tabular">{formatCompact(here.expPerHour)}</dd></div>
        <div><dt>Mesos / hr</dt><dd class="tabular">{formatCompact(here.mesosPerHour)}</dd></div>
      </dl>
    {/if}
  {:else}
    <p class="empty">Minimap not visible.</p>
  {/if}

  {#if doubtful && found && map}
    <div class="doubt">
      Read as “{mapLabel(map)}” ({Math.round(found.score * 100)}% match).
      {#if found.alternatives.length}
        Not it?
        {#each found.alternatives as alt (alt.id)}
          <button class="alt" onclick={() => navigate(alt.id)} title={alt.street ?? ""}>{alt.name}</button>
        {/each}
      {/if}
    </div>
  {/if}

  {#if shown}
    {#key shown.map.id}
      <MapDetailsView details={shown} {variant} onnavigate={navigate} />
    {/key}
  {:else if !ready}
    {#if mapData.status}
      <div class="setup"><MapDataPanel compact /></div>
    {/if}
  {:else if map && !looking}
    <p class="empty miss">No map in the data matches this name closely enough.</p>
  {/if}

  {#if ready && !compact}
    <div class="lookup">
      <MapPicker label="Look up a map" placeholder="Look up any map…" onpick={(m) => navigate(m.id)} />
    </div>
  {/if}
</Card>

<style>
  .where {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .pin {
    display: grid;
    flex: none;
    place-items: center;
    width: 32px;
    height: 32px;
    border-radius: 9px;
    background: color-mix(in srgb, var(--map) 16%, transparent);
    border: 1px solid color-mix(in srgb, var(--map) 30%, transparent);
    color: var(--map);
  }

  .elsewhere .pin {
    background: transparent;
    border-style: dashed;
    color: var(--muted);
  }

  .names {
    min-width: 0;
  }

  .street {
    color: var(--muted);
    font-size: 11.5px;
  }

  .name {
    font-family: var(--font-display);
    font-size: 15px;
    font-weight: 600;
    line-height: 1.25;
  }

  .here {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 6px;
    margin: 10px 0 0;
  }

  .here div {
    padding: 5px 8px;
    border-radius: var(--radius-sm);
    background: rgba(255, 255, 255, 0.03);
  }

  dt {
    font-size: 10.5px;
    color: var(--faint);
  }

  dd {
    margin: 0;
    font-weight: 600;
    color: var(--muted);
  }

  .empty {
    margin: 0;
    color: var(--faint);
    font-size: 12px;
  }

  .miss {
    margin-top: 8px;
  }

  .back {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    margin: -2px 0 8px;
    padding: 2px 8px 2px 6px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: transparent;
    color: var(--muted);
    font-size: 11px;
    cursor: pointer;
  }

  .back:hover {
    border-color: var(--border-strong);
    color: var(--text);
  }

  .doubt {
    margin-top: 8px;
    padding: 6px 8px;
    border-radius: var(--radius-sm);
    background: var(--warn-soft);
    color: var(--muted);
    font-size: 11px;
  }

  .alt {
    margin: 2px 2px 0 0;
    padding: 0 6px;
    border: 1px solid var(--border-strong);
    border-radius: 999px;
    background: transparent;
    font-size: 11px;
    cursor: pointer;
  }

  .alt:hover {
    border-color: var(--warn);
  }

  .setup {
    margin-top: 10px;
    padding-top: 10px;
    border-top: 1px solid var(--border);
  }

  .lookup {
    margin-top: 12px;
  }
</style>
