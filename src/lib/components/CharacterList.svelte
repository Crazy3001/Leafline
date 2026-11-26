<script lang="ts">
  // Every character the records know, to pick whose numbers Leafline shows: the Your character
  // card, Me, Quests, the bag and the training numbers all follow the pick. The one the game shows
  // is marked; picking another stands until the HUD reads a different name. With two or more,
  // "All characters" blends the log, records and best maps. Used by the desk sidebar's switch and
  // by the sheet the overlay's header opens.
  import { Check } from "@lucide/svelte";
  import { SvelteMap } from "svelte/reactivity";

  import { backend } from "$lib/api";
  import { characters } from "$lib/characters.svelte";
  import { history } from "$lib/history.svelte";
  import { isWeb } from "$lib/platform";
  import { shownCharacter } from "$lib/profile.svelte";
  import { live } from "$lib/readings.svelte";

  let { onpick }: { onpick?: () => void } = $props();

  const same = (a: string | null | undefined, b: string | null | undefined) =>
    a != null && b != null && a.trim().toLowerCase() === b.trim().toLowerCase();

  const onScreen = $derived(live.session?.name ?? null);
  const shown = $derived(shownCharacter());

  /** What was last read of each character, asked for once each while the list is open. */
  const read = new SvelteMap<string, { level: number | null; job: string | null }>();
  $effect(() => {
    for (const name of characters.known) {
      const key = name.trim().toLowerCase();
      if (read.has(key)) continue;
      read.set(key, { level: null, job: null });
      backend
        .knownCharacter(name)
        .then((k) => k && read.set(key, { level: k.level, job: k.job }))
        .catch(() => {});
    }
  });

  /** "Lv 34 · Hunter" for a character: read now or last for the one shown, else what was read last. */
  function line(name: string): string | null {
    let level: number | null = null;
    let job: string | null = null;
    if (!characters.all && same(name, shown.name)) {
      level = shown.level.value;
      job = shown.job.value;
    } else {
      const kept = read.get(name.trim().toLowerCase());
      const last = (history.value?.sessions ?? []).find((s) => same(s.character, name));
      level = kept?.level ?? last?.level ?? null;
      job = kept?.job ?? last?.job ?? null;
    }
    const parts = [level != null ? `Lv ${level}` : null, job].filter(Boolean);
    return parts.length ? parts.join(" · ") : null;
  }

  function pick(name: string | null) {
    characters.choose(name);
    onpick?.();
  }

  function pickAll() {
    characters.chooseAll();
    onpick?.();
  }
</script>

{#if characters.known.length === 0}
  <p class="none">
    {isWeb
      ? "No character in this browser yet: share the game to read one, or open a records file in Settings."
      : "No character read yet: log in with reading on."}
  </p>
{:else}
  <ul class="list" aria-label="Characters">
    {#each characters.known as name (name)}
      {@const on = !characters.all && same(name, characters.active)}
      {@const sub = line(name)}
      <li>
        <button class="row" class:on aria-pressed={on} onclick={() => pick(name)}>
          <span class="mark" aria-hidden="true">{#if on}<Check size={14} />{/if}</span>
          <span class="who">
            <b>{name}</b>
            {#if sub || same(name, onScreen)}
              <span class="sub">{#if same(name, onScreen)}<span class="here">In game now</span>{sub ? " · " : ""}{/if}{sub ?? ""}</span>
            {/if}
          </span>
        </button>
      </li>
    {/each}
    {#if characters.known.length === 1}
      <li class="more">Others show here once Leafline reads them.</li>
    {:else}
      <li class="all">
        <button class="row" class:on={characters.all} aria-pressed={characters.all} onclick={pickAll}>
          <span class="mark" aria-hidden="true">{#if characters.all}<Check size={14} />{/if}</span>
          <span class="who">
            <b>All characters</b>
            <span class="sub">Log, records, best maps</span>
          </span>
        </button>
      </li>
    {/if}
  </ul>
{/if}

<style>
  .none {
    margin: 0;
    color: var(--muted);
    font-size: var(--text-xs);
    line-height: 1.4;
  }

  .list {
    display: grid;
    gap: 1px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .more {
    padding: var(--space-1) var(--space-2) var(--space-1) calc(var(--space-2) * 2 + 16px);
    color: var(--faint);
    font-size: var(--text-xs);
  }

  .all {
    margin-top: var(--space-1);
    padding-top: var(--space-1);
    border-top: 1px solid var(--border);
  }

  .row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
    min-height: 40px;
    padding: var(--space-1) var(--space-2);
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .row:hover {
    background: var(--hover);
  }

  .row.on {
    background: var(--accent-soft);
  }

  .row:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }

  .mark {
    display: grid;
    flex: none;
    place-items: center;
    width: 16px;
    color: var(--accent-text-strong);
  }

  .who {
    display: grid;
    min-width: 0;
  }

  .who b {
    overflow: hidden;
    font-size: var(--text-md);
    font-weight: var(--weight-strong);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .sub {
    overflow: hidden;
    color: var(--muted);
    font-size: var(--text-xs);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .here {
    color: var(--good);
  }
</style>
