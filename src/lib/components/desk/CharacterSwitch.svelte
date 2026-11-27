<script lang="ts">
  // The desk sidebar's character switch, under the logo on every page: whose numbers Leafline
  // shows, with when they were read when it isn't reading them now, and the list of every character
  // to pick another. The list drops down over the tabs; Esc or a click elsewhere closes it. Folded
  // to icons, it is one round badge with the name's first letter, which opens the sidebar and the list.
  // Where the character's look was kept (from the game's Character Info window), it stands in for
  // the letter, and beside the name.
  import { ChevronsUpDown, UserRound } from "@lucide/svelte";

  import CharacterList from "$lib/components/CharacterList.svelte";
  import { characters, switched, watchSwitches } from "$lib/characters.svelte";
  import { ago, shownCharacter, watchProfile } from "$lib/profile.svelte";
  import { tip } from "$lib/tip.svelte";
  import { lookOf, watchLooks } from "$lib/looks.svelte";
  import LookPicture from "$lib/ui/LookPicture.svelte";

  let { folded, onunfold }: { folded: boolean; onunfold: () => void } = $props();

  watchProfile();
  watchSwitches();
  watchLooks();

  let open = $state(false);
  let button = $state<HTMLButtonElement>();
  let box = $state<HTMLElement>();

  /** Closes the list, focus back on the switch. */
  function close() {
    open = false;
    button?.focus();
  }
  const shown = $derived(shownCharacter());
  const none = $derived(characters.known.length === 0);
  const sub = $derived([shown.level.value != null ? `Lv ${shown.level.value}` : null, shown.job.value].filter(Boolean).join(" · "));
  /** A switch made by logging in, said for a few seconds. */
  const fresh = $derived(switched.name != null && !characters.all && switched.name === characters.active);
  /** Their look, kept from the game's Character Info window. */
  const look = $derived(characters.all ? null : lookOf(shown.name));
  /** The folded badge's letter. */
  const initial = $derived(characters.all ? null : (shown.name?.trim().charAt(0).toUpperCase() ?? null));

  function toggle() {
    if (folded) {
      onunfold();
      open = true;
    } else open = !open;
  }

  function onkeydown(event: KeyboardEvent) {
    if (open && event.key === "Escape") {
      event.preventDefault();
      close();
    }
  }

  /** A click outside the switch closes the list, leaving focus where the click put it. */
  function onpointerdown(event: PointerEvent) {
    if (open && box && !box.contains(event.target as Node)) open = false;
  }
</script>

<svelte:window {onkeydown} {onpointerdown} />

{#if folded}
  <button
    class="badge-btn"
    class:fresh
    aria-label={none ? "Characters" : `Character: ${shown.name ?? ""}. Pick another`}
    use:tip={none ? "No character seen yet" : `${characters.all ? "All characters" : shown.name}${sub ? `, ${sub}` : ""}`}
    onclick={toggle}
  >
    {#if look}<span class="face" aria-hidden="true"><LookPicture {look} size={38} crop="bust" /></span>
    {:else if none || initial == null}<UserRound size={16} />{:else}<span class="letter" aria-hidden="true">{initial}</span>{/if}
  </button>
{:else}
  <section class="switch" class:open aria-label="Character" bind:this={box}>
    {#if none}
      <!-- Says, in one line, how a character comes to be known. -->
      <div class="empty"><CharacterList /></div>
    {:else}
      <button class="current" class:fresh bind:this={button} aria-haspopup="listbox" aria-expanded={open} aria-label="Character: {characters.all ? 'All characters' : shown.name}. Pick another" onclick={toggle}>
        {#if look}<span class="face" aria-hidden="true" use:tip={`Look as of ${ago(look.readAt)}`}><LookPicture {look} size={34} crop="bust" /></span>{/if}
        <span class="id">
          <b>{characters.all ? "All characters" : shown.name}</b>
          {#if characters.all}
            <span class="sub">Goal and potions: {characters.active}</span>
          {:else if fresh}
            <span class="sub now">Switched: in game now</span>
          {:else}
            {#if sub}<span class="sub">{sub}</span>{/if}
            {#if shown.seenAt != null}<span class="sub stale">as of {ago(shown.seenAt)}</span>{/if}
          {/if}
        </span>
        <ChevronsUpDown size={15} />
      </button>
      {#if open}
        <div class="menu">
          <CharacterList onpick={close} />
        </div>
      {/if}
    {/if}
  </section>
{/if}

<style>
  .switch {
    position: relative;
    margin-bottom: var(--space-2);
  }

  /* Drops down over the tabs, so they don't move while it's open. */
  .menu {
    position: absolute;
    top: calc(100% + var(--space-1));
    left: 0;
    right: 0;
    z-index: 5;
    max-height: 60vh;
    overflow-y: auto;
    padding: var(--space-1);
    border: var(--line-w) solid var(--line);
    border-radius: var(--radius);
    background: var(--surface);
    box-shadow: var(--card-shadow);
  }

  .current {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
    min-height: 44px;
    padding: var(--space-1-5) var(--space-2) var(--space-1-5) var(--space-3);
    border: var(--line-w) solid var(--control-line);
    border-radius: var(--radius);
    background: var(--surface);
    color: var(--muted);
    font: inherit;
    text-align: left;
    cursor: pointer;
    transition: border-color 0.3s ease;
  }

  .current:hover,
  .open .current {
    border-color: var(--border-strong);
    color: var(--text);
  }

  /* The HUD switched character by itself: said, and outlined, for a few seconds. */
  .current.fresh,
  .badge-btn.fresh {
    border-color: var(--accent);
  }

  .current:focus-visible,
  .badge-btn:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  .id {
    display: grid;
    flex: 1;
    min-width: 0;
  }

  .id b {
    overflow: hidden;
    color: var(--text);
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

  /* Read earlier, not now. */
  .sub.stale {
    color: var(--faint);
  }

  .sub.now {
    color: var(--good);
  }

  .empty {
    padding: var(--space-1) var(--space-3);
  }

  .badge-btn {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    margin: 0 auto var(--space-2);
    border: var(--line-w) solid var(--control-line);
    border-radius: 50%;
    background: var(--surface);
    color: var(--text);
    cursor: pointer;
  }

  /* The look in a round frame, like the badge's letter. */
  .face {
    display: grid;
    flex: none;
    place-items: center;
    overflow: hidden;
    border-radius: 50%;
    background: var(--surface-2);
  }

  .current .face {
    width: 34px;
    height: 34px;
    margin-left: calc(-1 * var(--space-1));
  }

  .badge-btn .face {
    width: 38px;
    height: 38px;
  }

  .badge-btn:has(.face) {
    overflow: hidden;
    padding: 0;
  }

  .letter {
    font-family: var(--font-display);
    font-size: var(--text-md);
    font-weight: var(--weight-strong);
  }
</style>
