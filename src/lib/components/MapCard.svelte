<script lang="ts">
  import { MapPin } from "@lucide/svelte";

  import Card from "./Card.svelte";
  import { formatCompact, formatDuration, mapLabel } from "$lib/format";
  import type { MapReading, SessionSummary } from "$lib/types";

  let { map, session = null }: { map: MapReading | null; session?: SessionSummary | null } = $props();

  const here = $derived(map && session ? session.maps.find((m) => m.map === mapLabel(map)) : undefined);
</script>

<Card title="Map">
  {#snippet aside()}
    {#if map?.collapsed}<span title="Read from the collapsed minimap's title bar">Minimap title</span>{/if}
  {/snippet}

  {#if map}
    <div class="where">
      <span class="pin" aria-hidden="true"><MapPin size={15} /></span>
      <div class="names">
        {#if map.street}<div class="street">{map.street}</div>{/if}
        <div class="name">{map.name}</div>
      </div>
    </div>
    {#if here}
      <dl class="here">
        <div><dt>Time here</dt><dd class="tabular">{formatDuration(here.activeSeconds)}</dd></div>
        <div><dt>EXP / hr</dt><dd class="tabular">{formatCompact(here.expPerHour)}</dd></div>
        <div><dt>Mesos / hr</dt><dd class="tabular">{formatCompact(here.mesosPerHour)}</dd></div>
      </dl>
    {/if}
  {:else}
    <p class="empty">Minimap not visible.</p>
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
</style>
