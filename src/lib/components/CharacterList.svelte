<script lang="ts">
  // Every character the records know, to pick whose numbers Leafline shows: the Your character
  // card, Me, Quests, the bag and the training numbers all follow the pick. Under each name, one
  // line of when Leafline last saw them: level, job, how long ago and the map they were on, or
  // "Playing now" for the one the game shows. Picking another stands until the HUD reads a
  // different name. With two or more, "All characters" blends the log, records and best maps.
  // Used by the desk sidebar's switch and by the sheet the overlay's header opens. A listbox:
  // arrow keys, Home and End move, Enter or Space picks; Esc is the opener's to close.
  import { Check } from "@lucide/svelte";
  import { onMount } from "svelte";
  import { SvelteMap } from "svelte/reactivity";

  import { backend } from "$lib/api";
  import { characters } from "$lib/characters.svelte";
  import { splitMapLabel } from "$lib/format";
  import { history } from "$lib/history.svelte";
  import { isWeb } from "$lib/platform";
  import { ago, shownCharacter } from "$lib/profile.svelte";
  import { live } from "$lib/readings.svelte";
  import { clipped } from "$lib/tip.svelte";
  import { lookOf, watchLooks } from "$lib/looks.svelte";
  import LookPicture from "$lib/ui/LookPicture.svelte";

  let { onpick }: { onpick?: () => void } = $props();

  watchLooks();
  /** Once any character has a look kept, every row has a face (a letter where none is). */
  const faces = $derived(characters.known.some((name) => lookOf(name) != null));

  const same = (a: string | null | undefined, b: string | null | undefined) =>
    a != null && b != null && a.trim().toLowerCase() === b.trim().toLowerCase();
  const keyOf = (name: string) => name.trim().toLowerCase();

  const onScreen = $derived(live.session?.name ?? null);
  const shown = $derived(shownCharacter());

  type Seen = { level: number | null; job: string | null; map: string | null; at: number | null };
  /** What was last read of each character (records `character_profiles`), asked for once each. */
  const read = new SvelteMap<string, Seen>();
  $effect(() => {
    for (const name of characters.known) {
      const key = keyOf(name);
      if (read.has(key)) continue;
      read.set(key, { level: null, job: null, map: null, at: null });
      backend
        .knownCharacter(name)
        .then((k) => k && read.set(key, { level: k.level, job: k.job, map: k.map, at: k.seenAt }))
        .catch(() => {});
    }
  });

  /**
   * "Lv 23 Archer · 2h ago · Henesys Hunting Ground I": the map comes last, so a narrow list cuts
   * it before the age. The character the game shows is "Playing now", with what is read live.
   */
  function line(name: string): { text: string; playing: boolean } {
    const playing = same(name, onScreen);
    const kept = read.get(keyOf(name));
    const last = (history.value?.sessions ?? []).find((s) => same(s.character, name));
    const isShown = !characters.all && same(name, shown.name);
    const level = (isShown ? shown.level.value : null) ?? kept?.level ?? last?.level ?? null;
    const job = (isShown ? shown.job.value : null) ?? kept?.job ?? last?.job ?? null;
    const at = kept?.at ?? last?.savedAt ?? null;
    const who = [level != null ? `Lv ${level}` : null, job].filter(Boolean).join(" ");
    const when = playing ? null : at != null ? ago(at) : null;
    const where = playing ? null : kept?.map ? splitMapLabel(kept.map).name : null;
    return { text: [who || null, when, where].filter(Boolean).join(" · "), playing };
  }

  function pick(name: string | null) {
    characters.choose(name);
    onpick?.();
  }

  function pickAll() {
    characters.chooseAll();
    onpick?.();
  }

  let list = $state<HTMLElement>();
  const rows = () => [...(list?.querySelectorAll<HTMLElement>('[role="option"]') ?? [])];

  /** One Tab stop, the picked row; the arrows, Home and End move between rows. */
  function onkeydown(event: KeyboardEvent) {
    const all = rows();
    const at = all.indexOf(document.activeElement as HTMLElement);
    let to: number | null = null;
    if (event.key === "ArrowDown") to = at < 0 ? 0 : Math.min(all.length - 1, at + 1);
    else if (event.key === "ArrowUp") to = at < 0 ? all.length - 1 : Math.max(0, at - 1);
    else if (event.key === "Home") to = 0;
    else if (event.key === "End") to = all.length - 1;
    else if ((event.key === "Enter" || event.key === " ") && at >= 0) {
      event.preventDefault();
      all[at].click();
      return;
    }
    if (to == null) return;
    event.preventDefault();
    all[to]?.focus();
  }

  // Opened as a menu: the picked row takes focus, so the arrows work at once.
  onMount(() => {
    if (onpick) (rows().find((r) => r.getAttribute("aria-selected") === "true") ?? rows()[0])?.focus();
  });
</script>

{#if characters.known.length === 0}
  <p class="none">
    {isWeb
      ? "No character in this browser yet: share the game to read one, or open a records file in Settings."
      : "No character read yet: log in with reading on."}
  </p>
{:else}
  <ul class="list" role="listbox" aria-label="Characters" tabindex="-1" bind:this={list} {onkeydown}>
    {#each characters.known as name (name)}
      {@const on = !characters.all && same(name, characters.active)}
      {@const seen = line(name)}
      <!-- svelte-ignore a11y_click_events_have_key_events (the list's keydown picks with Enter or Space) -->
      <li class="row" class:on role="option" aria-selected={on} tabindex={on ? 0 : -1} onclick={() => pick(name)}>
        <span class="mark" aria-hidden="true">{#if on}<Check size={14} />{/if}</span>
        {#if faces}
          {@const look = lookOf(name)}
          <span class="face" aria-hidden="true">{#if look}<LookPicture {look} size={30} crop="bust" />{:else}{name.trim().charAt(0).toUpperCase()}{/if}</span>
        {/if}
        <span class="who">
          <b use:clipped>{name}</b>
          {#if seen.text || seen.playing}
            <span class="sub" use:clipped
              >{#if seen.playing}<span class="here">Playing now</span>{seen.text ? " · " : ""}{/if}{seen.text}</span
            >
          {/if}
        </span>
      </li>
    {/each}
    {#if characters.known.length === 1}
      <li class="more" role="presentation">Others show here once Leafline reads them.</li>
    {:else}
      <li class="rule" role="presentation"></li>
      <!-- svelte-ignore a11y_click_events_have_key_events (the list's keydown picks with Enter or Space) -->
      <li class="row" class:on={characters.all} role="option" aria-selected={characters.all} tabindex={characters.all ? 0 : -1} onclick={pickAll}>
        <span class="mark" aria-hidden="true">{#if characters.all}<Check size={14} />{/if}</span>
        <span class="who">
          <b>All characters</b>
          <span class="sub">Log, records, best maps</span>
        </span>
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

  .list:focus {
    outline: none;
  }

  .more {
    padding: var(--space-1) var(--space-2) var(--space-1) calc(var(--space-2) * 2 + 16px);
    color: var(--faint);
    font-size: var(--text-xs);
  }

  .rule {
    margin: var(--space-1) 0;
    border-top: 1px solid var(--border);
  }

  .row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-height: 40px;
    padding: var(--space-1) var(--space-2);
    border-radius: var(--radius-sm);
    color: var(--text);
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

  .face {
    display: grid;
    flex: none;
    place-items: center;
    width: 30px;
    height: 30px;
    overflow: hidden;
    border-radius: 50%;
    background: var(--surface-2);
    color: var(--muted);
    font-family: var(--font-display);
    font-size: var(--text-sm);
    font-weight: var(--weight-strong);
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
