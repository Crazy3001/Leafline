<script lang="ts">
  import { tip } from "$lib/tip.svelte";
  import { Map as MapIcon, MapPin } from "@lucide/svelte";

  import Card from "./Card.svelte";
  import MapNote from "./MapNote.svelte";
  import MapDataPanel from "./map/MapDataPanel.svelte";
  import MapDetailsView from "./map/MapDetailsView.svelte";
  import MapPicker from "./map/MapPicker.svelte";
  import GoTo from "./map/GoTo.svelte";
  import GoButton from "./GoButton.svelte";
  import SaveSpot from "./SaveSpot.svelte";
  import { mapPlace } from "$lib/destination.svelte";
  import { backend } from "$lib/api";
  import { formatCompact, formatDuration, mapLabel } from "$lib/format";
  import { mapData, watchMapData } from "$lib/mapdata.svelte";
  import type { MapDetails, MapMatch, MapReading, SessionSummary } from "$lib/types";

  let {
    map,
    session = null,
    variant = "full",
    plain = false,
  }: {
    map: MapReading | null;
    session?: SessionSummary | null;
    /** `compact` in the overlay, `glance` in a pinned window, `full` everything: the
     *  map, your rates on it and its monsters. */
    variant?: "full" | "compact" | "glance";
    plain?: boolean;
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

  // Each live reading is a new object, so the effect follows the text: the same header read
  // again must not look the map up, which would blank the card and shrink a pinned window.
  const name = $derived(map?.name);
  const street = $derived(map?.street ?? null);
  const twin = $derived(map?.twin ?? null);

  $effect(() => {
    const asked = name;
    const askedTwin = twin;
    void mapData.status?.dataUpdatedAt;
    browsing = null;
    found = null;
    if (!asked || !ready) return;
    looking = true;
    backend
      .findMap(asked, street, askedTwin)
      .then((match) => {
        if (name === asked && twin === askedTwin) found = match;
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
  /** The map the player is on. */
  const current = $derived(found?.card.map ?? null);
  const currentName = $derived(current?.name ?? map?.name ?? null);
  // A poor match, or other maps nearly as close: let the user correct it. OCR slips a letter
  // often, so a lone good match isn't worth a warning.
  const doubtful = $derived(!!found && !browsing && (found.score < 0.85 || found.alternatives.length > 0));
</script>

<Card title="Map" {plain}>
  {#snippet aside()}
    {#if map?.collapsed && !browsing}<span>Minimap title</span>{/if}
  {/snippet}

  {#if browsing}
    <button class="back" onclick={() => (browsing = null)}>
      <MapPin size={13} />
      <span class="back-to">{#if currentName}Back to <strong>{currentName}</strong>{:else}Back{/if}</span>
    </button>
  {/if}

  {#if map || shown}
    <div class="where" class:elsewhere={!!browsing}>
      <span class="pin" aria-hidden="true">
        {#if browsing}<MapIcon size={15} />{:else}<MapPin size={15} />{/if}
      </span>
      <div class="names">
        {#if browsing}
          {#if browsing.map.street}<div class="street">{browsing.map.street}</div>{/if}
          <div class="name">{browsing.map.name} <GoButton place={mapPlace(browsing.map)} /></div>
        {:else if found}
          {#if found.card.map.street}<div class="street">{found.card.map.street}</div>{/if}
          <div class="name">{found.card.map.name}</div>
        {:else if map}
          {#if map.street}<div class="street">{map.street}</div>{/if}
          <div class="name">{map.name}</div>
        {/if}
      </div>
    </div>
    {#if shown && variant !== "glance"}
      <div class="save-row"><SaveSpot mapId={shown.map.id} name={shown.map.name} text /></div>
    {/if}
    {#if here && !browsing}
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
          <button class="alt" onclick={() => navigate(alt.id)} use:tip={alt.street ?? undefined}>{alt.name}</button>
        {/each}
      {/if}
    </div>
  {/if}

  <!-- Setting a place is done in the tab; a pinned window follows it in the Route pin. -->
  {#if ready && variant !== "glance"}
    <div class="goto"><GoTo /></div>
  {/if}

  {#if shown}
    <MapNote mapId={shown.map.id} name={shown.map.name} />
    {#key shown.map.id}
      <MapDetailsView details={shown} {current} {variant} onnavigate={navigate} />
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
  .goto {
    margin: var(--space-2) 0;
  }

  .save-row {
    margin-top: 6px;
  }

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
    border-radius: var(--radius-sm);
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
    font-size: var(--text-sm);
  }

  .name {
    font-family: var(--font-display);
    font-size: var(--text-xl);
    font-weight: 600;
    line-height: 1.2;
  }

  .here {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 6px;
    margin: 10px 0 0;
  }

  /* The same tiles as src/lib/ui/Stat.svelte. */
  .here div {
    padding: var(--space-2);
    background: var(--surface-2);
  }

  dt {
    font-size: var(--text-xs);
    color: var(--muted);
  }

  dd {
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--text-lg);
    font-weight: 600;
    color: var(--text);
  }

  .empty {
    margin: 0;
    color: var(--faint);
    font-size: var(--text-sm);
  }

  .miss {
    margin-top: 8px;
  }

  /* Stays in view while a long map scrolls under it. */
  .back {
    position: sticky;
    top: 0;
    z-index: 1;
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    margin: -2px 0 10px;
    padding: 6px 9px;
    border: 1px solid color-mix(in srgb, var(--map) 45%, transparent);
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--map) 16%, var(--surface-3));
    color: var(--map);
    font-size: var(--text-sm);
    text-align: left;
    cursor: pointer;
    transition:
      border-color 0.12s ease,
      background 0.12s ease;
  }

  .back:hover {
    border-color: var(--map);
    background: color-mix(in srgb, var(--map) 26%, var(--surface-3));
  }

  .back:focus-visible {
    outline: 2px solid var(--map);
    outline-offset: 1px;
  }

  .back :global(svg) {
    flex: none;
  }

  .back-to {
    min-width: 0;
    overflow: hidden;
    color: var(--muted);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .back-to strong {
    color: var(--text);
    font-weight: 600;
  }

  .doubt {
    margin-top: 8px;
    padding: 6px 8px;
    border-radius: var(--radius-sm);
    background: var(--warn-soft);
    color: var(--muted);
    font-size: var(--text-xs);
  }

  .alt {
    margin: 2px 2px 0 0;
    padding: 0 6px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: transparent;
    font-size: var(--text-xs);
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
