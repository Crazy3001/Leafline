<script lang="ts">
  // The training log's Drops view: every drop the player logged, by monster and as a list. Each
  // can be reported on MeowDB, which opens its page in the player's browser.
  import { tip } from "$lib/tip.svelte";
  import Ref from "$lib/components/Ref.svelte";
  import { Check, ExternalLink, Trash2 } from "@lucide/svelte";

  import { backend } from "$lib/api";
  import { drops, meowdbMonster, reportOnMeowdb, tallyByMonster, watchDrops } from "$lib/drops.svelte";
  import { showItemNamed } from "$lib/items.svelte";
  import { showMonsterNamed } from "$lib/monsters.svelte";
  import { showMapNamed } from "$lib/places.svelte";
  import { formatDay } from "$lib/format";
  import { clock } from "$lib/training";
  import type { LoggedDrop } from "$lib/types";

  let { character = "", since = null }: { character?: string; since?: number | null } = $props();

  watchDrops();

  let problem = $state<string | null>(null);
  let copiedId = $state<number | null>(null);

  const shown = $derived(
    (drops.value ?? []).filter((d) => (!character || d.character === character) && (since == null || d.seenAt >= since)),
  );
  const tallies = $derived(tallyByMonster(shown));

  async function report(d: LoggedDrop) {
    try {
      if (await reportOnMeowdb(d.monster, d.item)) copiedId = d.id;
    } catch (e) {
      problem = String(e);
    }
  }

  async function remove(id: number) {
    try {
      await backend.deleteDrop(id);
    } catch (e) {
      problem = String(e);
    }
  }
</script>

{#if problem}<p class="problem" role="alert">{problem}</p>{/if}

{#if drops.value == null}
  <p class="quiet">Loading…</p>
{:else if !shown.length}
  <div class="empty">
    <strong>No drops logged {drops.value.length ? "in this range" : "yet"}</strong>
    <span>
      When you pick up something worth noting, hover it in game and read it with the Read item hotkey. Then pick the monster
      under “Got it as a drop?” on the item check. Your drop log stays on this PC.
    </span>
  </div>
{:else}
  <div class="split">
    <section class="panel by">
      <div class="head">
        <h2>By monster</h2>
        <span class="quiet tabular">{shown.length} drop{shown.length === 1 ? "" : "s"}</span>
      </div>
      {#each tallies as t (t.monster)}
        <div class="monster">
          <div class="mrow">
            <span class="mname"><Ref kind="mob" name={t.monster} /></span>
            <span class="tabular quiet">{t.total}</span>
            <button class="icon" use:tip={`${t.monster} on MeowDB`} aria-label="Open {t.monster} on MeowDB" onclick={() => backend.openLink(meowdbMonster(t.monster))}
              ><ExternalLink size={12} /></button
            >
          </div>
          <ul>
            {#each t.items as i (i.item)}
              <li><span><Ref kind="item" name={i.item} /></span><span class="tabular muted">×{i.count}</span></li>
            {/each}
          </ul>
        </div>
      {/each}
      <p class="caption">
        Counts are what you logged, not drop rates: Leafline doesn't count kills. MeowDB's pages show what other players have
        confirmed.
      </p>
    </section>

    <section class="panel list">
      <div class="head">
        <h2>Every drop</h2>
        <span class="grow"></span>
      </div>
      <table>
        <thead><tr><th>When</th><th>Item</th><th>Monster</th><th>Map</th><th></th></tr></thead>
        <tbody>
          {#each shown as d (d.id)}
            <tr>
              <td class="tabular muted nowrap">{formatDay(d.seenAt)} {clock(d.seenAt)}</td>
              <td><Ref kind="item" name={d.item} /></td>
              <td><Ref kind="mob" name={d.monster} /></td>
              <td>{#if d.map}<Ref kind="map" name={d.map} />{:else}<span class="muted">—</span>{/if}</td>
              <td class="nowrap r">
                <button class="report" use:tip={"Copies the item's name too, to paste into MeowDB's search"} onclick={() => report(d)}>
                  {#if copiedId === d.id}<Check size={12} />Name copied{:else}<ExternalLink size={12} />Report on MeowDB{/if}
                </button>
                <button class="icon" use:tip={"Delete this drop"} aria-label="Delete this drop" onclick={() => remove(d.id)}><Trash2 size={12} /></button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </section>
  </div>
{/if}

<style>
  .split {
    display: grid;
    grid-template-columns: minmax(240px, 1fr) minmax(0, 2fr);
    gap: 16px;
    align-items: start;
  }

  .panel {
    padding: 12px 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
  }

  .head {
    display: flex;
    align-items: baseline;
    gap: 8px;
    margin-bottom: 8px;
  }

  h2 {
    margin: 0;
    font-size: var(--text-lg);
    font-weight: 600;
  }

  .grow {
    flex: 1;
  }

  /* Each monster's drops in a block of their own, framed as the Quests tab frames a quest. */
  .monster {
    padding: 6px 8px;
    border: 1px solid var(--border-strong);
    border-left: 3px solid var(--accent);
    border-radius: var(--radius-sm);
    background: var(--surface-2);
  }

  .monster + .monster {
    margin-top: 6px;
  }

  .mrow {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .mname {
    flex: 1;
    font-weight: 600;
  }

  ul {
    margin: 4px 0 0;
    padding: 0;
    list-style: none;
  }

  li {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    padding: 1px 0;
    font-size: var(--text-sm);
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--text-sm);
  }

  th {
    padding: 4px 6px;
    color: var(--faint);
    font-size: var(--text-xs);
    font-weight: 600;
    text-align: left;
    text-transform: uppercase;
  }

  td {
    padding: 5px 6px;
    border-top: 1px solid var(--border);
  }

  .nowrap {
    white-space: nowrap;
  }

  .r {
    text-align: right;
  }

  .muted {
    color: var(--muted);
  }

  .quiet {
    margin: 0;
    color: var(--faint);
    font-size: var(--text-sm);
  }

  .caption {
    margin: 10px 0 0;
    color: var(--faint);
    font-size: var(--text-xs);
  }

  .icon,
  .report {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: var(--control-h-sm);
    padding: 0 6px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--muted);
    font-size: var(--text-xs);
    vertical-align: middle;
    cursor: pointer;
  }

  .icon:hover,
  .report:hover {
    background: var(--surface-3);
    color: var(--text);
  }

  .empty {
    display: grid;
    gap: 6px;
    max-width: 520px;
    margin: 40px auto;
    color: var(--muted);
    text-align: center;
  }

  .empty strong {
    color: var(--text);
    font-size: var(--text-lg);
  }

  .problem {
    color: var(--bad);
  }
</style>
