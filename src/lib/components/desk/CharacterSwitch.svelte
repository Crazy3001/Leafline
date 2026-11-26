<script lang="ts">
  // The desk sidebar's character switch, in view on every page: whose numbers Leafline shows, with
  // when they were read when it isn't reading them now, and the list of every character to pick
  // another. The list opens upward, over the reading status. Folded to icons, it is one round
  // badge with the level, which opens the sidebar and the list.
  import { ChevronsUpDown, UserRound } from "@lucide/svelte";

  import CharacterList from "$lib/components/CharacterList.svelte";
  import { characters, switched, watchSwitches } from "$lib/characters.svelte";
  import { ago, shownCharacter, watchProfile } from "$lib/profile.svelte";
  import { tip } from "$lib/tip.svelte";

  let { folded, onunfold }: { folded: boolean; onunfold: () => void } = $props();

  watchProfile();
  watchSwitches();

  let open = $state(false);
  let button = $state<HTMLButtonElement>();

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
</script>

<svelte:window {onkeydown} />

{#if folded}
  <button
    class="badge-btn"
    class:fresh
    aria-label={none ? "Characters" : `Character: ${shown.name ?? ""}. Pick another`}
    use:tip={none ? "No character seen yet" : `${characters.all ? "All characters" : shown.name}${sub ? `, ${sub}` : ""}`}
    onclick={toggle}
  >
    {#if none || characters.all || shown.level.value == null}<UserRound size={16} />{:else}<span class="num">{shown.level.value}</span>{/if}
  </button>
{:else}
  <section class="switch" class:open aria-label="Character">
    {#if none}
      <!-- Says, in one line, how a character comes to be known. -->
      <div class="empty"><CharacterList /></div>
    {:else}
      {#if open}
        <div class="menu">
          <CharacterList onpick={close} />
        </div>
      {/if}
      <button class="current" class:fresh bind:this={button} aria-haspopup="listbox" aria-expanded={open} aria-label="Character: {characters.all ? 'All characters' : shown.name}. Pick another" onclick={toggle}>
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
    {/if}
  </section>
{/if}

<style>
  .switch {
    display: grid;
    gap: var(--space-1);
  }

  .menu {
    max-height: 50vh;
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
    margin: 0 auto var(--space-1);
    border: var(--line-w) solid var(--control-line);
    border-radius: 50%;
    background: var(--surface);
    color: var(--text);
    cursor: pointer;
  }

  .num {
    font-family: var(--font-display);
    font-size: var(--text-md);
    font-weight: var(--weight-strong);
    font-variant-numeric: tabular-nums;
  }
</style>
