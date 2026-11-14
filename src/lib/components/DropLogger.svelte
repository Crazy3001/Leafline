<script lang="ts">
  // "Got it as a drop?" on the item check: one click logs which monster dropped the item, from
  // the monsters on the map the player is on. The log stays on this PC. Reporting the drop to
  // MeowDB opens its monster page in the player's browser with the item's name copied.
  import { tip } from "$lib/tip.svelte";
  import { Check, ExternalLink, Undo2 } from "@lucide/svelte";
  import { untrack } from "svelte";

  import Section from "./map/Section.svelte";
  import { backend } from "$lib/api";
  import { reportOnMeowdb, timesLogged, watchDrops } from "$lib/drops.svelte";
  import { mapData } from "$lib/mapdata.svelte";
  import { live } from "$lib/readings.svelte";
  import type { MapDetails, MapReading } from "$lib/types";

  let { item, itemId = null, map = null }: { item: string; itemId?: number | null; map?: MapReading | null } = $props();

  watchDrops();

  const ready = $derived(mapData.status?.ready ?? false);
  const character = $derived(live.analysis?.hud?.name ?? null);
  const place = $derived(map ? JSON.stringify([map.name, map.street, map.twin, ready]) : null);

  let card = $state<MapDetails | null>(null);
  /** The drop just logged, so it can be undone or reported. */
  let logged = $state<{ id: number; monster: string } | null>(null);
  let other = $state(false);
  let typed = $state("");
  let copied = $state<boolean | null>(null);
  let error = $state<string | null>(null);
  let box = $state<HTMLInputElement>();

  $effect(() => {
    if (other) box?.focus();
  });

  // Follows the map alone: every live reading brings new objects with the same content.
  $effect(() => {
    const key = place;
    card = null;
    if (key == null || !ready) return;
    const m = untrack(() => map!);
    backend.findMap(m.name, m.street, m.twin).then((match) => {
      if (key === place) card = match?.card ?? null;
    });
  });

  // A different item starts over.
  $effect(() => {
    void item;
    untrack(() => {
      logged = null;
      other = false;
      typed = "";
      copied = null;
      error = null;
    });
  });

  const monsters = $derived([...(card?.mobs ?? [])].sort((a, b) => b.count - a.count));
  const count = $derived(timesLogged(item));

  async function log(monster: string, monsterId: number | null) {
    error = null;
    copied = null;
    try {
      const id = await backend.logDrop({
        item,
        itemId,
        monster,
        monsterId: monsterId ?? (ready ? await backend.findMonster(monster) : null),
        map: card?.map.name ?? map?.name ?? null,
        mapId: card?.map.id ?? null,
        character,
      });
      logged = { id, monster };
      other = false;
      typed = "";
    } catch (e) {
      error = String(e);
    }
  }

  async function undo() {
    if (!logged) return;
    try {
      await backend.deleteDrop(logged.id);
      logged = null;
      copied = null;
    } catch (e) {
      error = String(e);
    }
  }

  async function report() {
    if (!logged) return;
    copied = await reportOnMeowdb(logged.monster, item);
  }

  function submitOther(event: SubmitEvent) {
    event.preventDefault();
    if (typed.trim()) log(typed.trim(), null);
  }
</script>

{#if item}
  <Section title="Got it as a drop?" count={count || undefined}>
    {#if logged}
      <p class="done"><Check size={13} /> Logged from <b>{logged.monster}</b>{card ? ` on ${card.map.name}` : ""}.</p>
      <div class="row">
        <button class="act" onclick={report} use:tip={"Copies the item's name too, to paste into MeowDB's search"}
          ><ExternalLink size={12} />Report on MeowDB</button
        >
        <button class="act ghost" onclick={undo}><Undo2 size={12} />Undo</button>
      </div>
      {#if copied != null}
        <p class="hint">
          {copied ? `“${item}” is copied. On MeowDB, open ${logged.monster} and paste it into “+ Add another item”.` : `On MeowDB, open ${logged.monster} and add “${item}” with “+ Add another item”.`}
        </p>
      {/if}
    {:else}
      {#if monsters.length}
        <p class="lead">Which monster dropped it{card ? ` on ${card.map.name}` : ""}?</p>
        <div class="row">
          {#each monsters as mob (mob.id)}
            <button class="mob" onclick={() => log(mob.name, mob.id)}>{mob.name}<small class="tabular">Lv {mob.level}</small></button>
          {/each}
          {#if !other}<button class="mob ghost" onclick={() => (other = true)}>Another…</button>{/if}
        </div>
      {:else if !other}
        <p class="lead">{map ? "No monsters known for this map." : "Your map isn't read yet."}
          <button class="link" onclick={() => (other = true)}>Type the monster</button>
        </p>
      {/if}
      {#if other}
        <form class="other" onsubmit={submitOther}>
          <input bind:this={box} bind:value={typed} onkeydown={(e) => e.key === "Escape" && (other = false)} placeholder="Monster's name" aria-label="Monster that dropped it" maxlength="60" />
          <button class="act" type="submit" disabled={!typed.trim()}>Log</button>
        </form>
      {/if}
    {/if}
    {#if error}<p class="error">{error}</p>{/if}
    <p class="hint">Your drop log stays on this PC. See it all in the Training log.</p>
  </Section>
{/if}

<style>
  .lead,
  .done {
    margin: 0 0 6px;
    color: var(--muted);
    font-size: var(--text-sm);
  }

  .done {
    display: flex;
    align-items: center;
    gap: 5px;
    color: var(--text);
  }

  .done :global(svg) {
    color: var(--good);
  }

  .row {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .mob,
  .act {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: var(--control-h-sm);
    padding: 0 8px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    color: var(--text);
    font-size: var(--text-sm);
    cursor: pointer;
  }

  .mob small {
    color: var(--faint);
    font-size: var(--text-xs);
  }

  .mob:hover,
  .act:hover:not(:disabled),
  .mob:focus-visible,
  .act:focus-visible {
    background: var(--surface-3);
    outline: none;
  }

  .ghost {
    border-color: transparent;
    background: transparent;
    color: var(--muted);
  }

  .act:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .link {
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--accent-strong);
    font-size: var(--text-sm);
    cursor: pointer;
  }

  .link:hover {
    text-decoration: underline;
  }

  .other {
    display: flex;
    gap: 4px;
    margin-top: 6px;
  }

  .other input {
    flex: 1;
    min-width: 0;
    height: var(--control-h-sm);
    padding: 0 8px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: rgba(0, 0, 0, 0.25);
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
  }

  .other input:focus {
    border-color: var(--accent);
    outline: none;
  }

  .hint {
    margin: 6px 0 0;
    color: var(--faint);
    font-size: var(--text-xs);
  }

  .error {
    margin: 6px 0 0;
    color: var(--bad);
    font-size: var(--text-sm);
  }
</style>
