<script lang="ts">
  // The five tabs along the bottom. The open one is a filled pill, easy to find at a glance; a
  // dot marks a tab whose content changed while you were elsewhere.
  import { tip } from "$lib/tip.svelte";

  import { TABS, type Tab } from "./tabs";

  let {
    tab,
    fresh,
    pinned,
    onselect,
  }: {
    tab: Tab;
    /** Tabs with something new since they were last open. */
    fresh: (tab: Tab) => boolean;
    /** Tabs whose panel is also pinned in its own window. */
    pinned: (tab: Tab) => boolean;
    onselect: (tab: Tab) => void;
  } = $props();

  function onkeydown(event: KeyboardEvent) {
    const i = TABS.findIndex((t) => t.id === tab);
    if (event.key === "ArrowRight") onselect(TABS[(i + 1) % TABS.length].id);
    else if (event.key === "ArrowLeft") onselect(TABS[(i - 1 + TABS.length) % TABS.length].id);
    else return;
    event.preventDefault();
  }
</script>

<nav aria-label="Sections">
  <div class="tablist" role="tablist" tabindex="-1" {onkeydown}>
    {#each TABS as t (t.id)}
      {@const Icon = t.icon}
      {@const on = tab === t.id}
      <button
        role="tab"
        id="tab-{t.id}"
        aria-selected={on}
        aria-controls="panel"
        tabindex={on ? 0 : -1}
        class:on
        use:tip={pinned(t.id) && t.id !== "now" ? `${t.title} (also pinned)` : null}
        onclick={() => onselect(t.id)}
      >
        <span class="icon">
          <Icon size={19} strokeWidth={on ? 2.2 : 2} />
          {#if fresh(t.id)}<span class="dot" aria-label="Updated"></span>{/if}
        </span>
        {t.title}
      </button>
    {/each}
  </div>
</nav>

<style>
  nav {
    flex: none;
    border-top: var(--line-w) solid var(--nav-line);
    background: var(--nav-bg);
    user-select: none;
  }

  .tablist {
    display: flex;
    height: 60px;
    padding: 0 4px;
    outline: none;
  }

  button {
    display: flex;
    flex: 1;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 3px;
    min-width: 0;
    margin: 6px 3px;
    padding: 0;
    border: var(--line-w) solid transparent;
    border-radius: 12px;
    background: transparent;
    color: var(--nav-text);
    font-family: var(--font-sans);
    font-size: var(--text-xs);
    font-weight: var(--weight-body);
    cursor: pointer;
    transition:
      background 0.14s ease,
      color 0.14s ease;
  }

  button:hover:not(.on) {
    background: color-mix(in srgb, var(--nav-text) 12%, transparent);
  }

  button.on {
    border-color: var(--nav-line);
    background: var(--accent);
    color: var(--accent-ink);
    font-weight: var(--weight-strong);
  }

  button:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  button.on:focus-visible {
    outline-color: var(--text);
  }

  .icon {
    position: relative;
    display: grid;
  }

  .dot {
    position: absolute;
    top: -2px;
    right: -5px;
    width: 8px;
    height: 8px;
    border: 2px solid var(--nav-bg);
    border-radius: 50%;
    background: var(--accent);
  }

  /* Dragged narrow, only the open tab keeps its name. */
  @media (max-width: 300px) {
    button:not(.on) {
      font-size: 0;
      gap: 0;
    }
  }
</style>
