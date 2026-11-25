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
                <button
                  class="report"
                  aria-label={copiedId === d.id ? "Name copied" : "Report on MeowDB"}
                  use:tip={"Copies the item's name too, to paste into MeowDB's search"}
                  onclick={() => report(d)}
                >
                  {#if copiedId === d.id}<Check size={12} /><span class="words">Name copied</span>{:else}<ExternalLink size={12} /><span
                      class="words">Report on MeowDB</span
                    >{/if}
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
    gap: var(--space-4);
    align-items: start;
  }

  /* In a narrow window (the Tools sheet) the two panels stack, and each drop stacks too: the item,
     then its monster and map, then when, with its buttons as icons at the right. */
  @media (max-width: 560px) {
    .split {
      grid-template-columns: minmax(0, 1fr);
    }

    thead {
      display: none;
    }

    table,
    tbody {
      display: block;
    }

    tr {
      display: grid;
      grid-template-columns: auto minmax(0, 1fr) auto;
      grid-template-areas:
        "item item act"
        "mob map act"
        "when when act";
      gap: 1px var(--space-1-5);
      align-items: center;
      padding: var(--space-1-5) 0;
      border-top: 1px solid var(--border);
    }

    tr:first-child {
      border-top: 0;
    }

    tr > td {
      padding: 0;
      border-top: 0;
    }

    td:nth-child(1) {
      grid-area: when;
      font-size: var(--text-xs);
    }

    td:nth-child(2) {
      grid-area: item;
      font-weight: var(--weight-strong);
    }

    td:nth-child(3) {
      grid-area: mob;
    }

    td:nth-child(4) {
      grid-area: map;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    td:nth-child(4)::before {
      content: "· ";
      color: var(--faint);
    }

    td:nth-child(5) {
      grid-area: act;
    }

    .report .words {
      display: none;
    }
  }

  .panel {
    padding: var(--space-3) var(--space-4);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
  }

  .head {
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
    margin-bottom: var(--space-2);
  }

  h2 {
    margin: 0;
    font-size: var(--text-lg);
    font-weight: var(--weight-strong);
  }

  .grow {
    flex: 1;
  }

  /* Each monster's drops in a block of their own, framed as the Quests tab frames a quest. */
  .monster {
    padding: var(--space-1-5) var(--space-2);
    border: 1px solid var(--border-strong);
    border-left: 3px solid var(--accent);
    border-radius: var(--radius-sm);
    background: var(--surface-2);
  }

  .monster + .monster {
    margin-top: var(--space-1-5);
  }

  .mrow {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .mname {
    flex: 1;
    font-weight: var(--weight-strong);
  }

  ul {
    margin: var(--space-1) 0 0;
    padding: 0;
    list-style: none;
  }

  li {
    display: flex;
    justify-content: space-between;
    gap: var(--space-2);
    padding: 1px 0;
    font-size: var(--text-md);
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--text-md);
  }

  th {
    padding: var(--space-1) var(--space-1-5);
    text-align: left;
    color: var(--faint);
    font-size: var(--text-xs);
    font-weight: var(--weight-body);
  }

  td {
    padding: var(--space-1) var(--space-1-5);
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
    font-size: var(--text-md);
  }

  .caption {
    margin: var(--space-2) 0 0;
    color: var(--faint);
    font-size: var(--text-xs);
  }

  .icon,
  .report {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    height: var(--control-h-sm);
    padding: 0 var(--space-1-5);
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
    gap: var(--space-1-5);
    max-width: 520px;
    margin: var(--space-10) auto;
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
