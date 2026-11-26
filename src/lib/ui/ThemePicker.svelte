<script lang="ts">
  // The four looks as cards to pick from. Each card draws a tiny Leafline window in its own
  // colours, fonts and corners, so the choice is made by eye; picking one changes every window
  // at once.
  import { Check } from "@lucide/svelte";

  import { changeSettings, display } from "$lib/settings.svelte";
  import type { Theme } from "$lib/types";

  import { THEMES } from "./theme.svelte";

  let { onpick }: { onpick?: (theme: Theme) => void } = $props();

  function pick(theme: Theme) {
    if (display.theme !== theme) void changeSettings({ theme });
    onpick?.(theme);
  }

  function onkeydown(event: KeyboardEvent) {
    const i = THEMES.findIndex((t) => t.id === display.theme);
    const step = event.key === "ArrowRight" || event.key === "ArrowDown" ? 1 : event.key === "ArrowLeft" || event.key === "ArrowUp" ? -1 : 0;
    if (!step) return;
    event.preventDefault();
    const next = THEMES[(i + step + THEMES.length) % THEMES.length];
    pick(next.id);
    (event.currentTarget as HTMLElement).querySelector<HTMLElement>(`[data-id="${next.id}"]`)?.focus();
  }
</script>

<div class="picker" role="radiogroup" aria-label="Look" tabindex="-1" {onkeydown}>
  {#each THEMES as t (t.id)}
    {@const on = display.theme === t.id}
    <button class="choice" class:on role="radio" aria-checked={on} tabindex={on ? 0 : -1} data-id={t.id} onclick={() => pick(t.id)}>
      <!-- A small window in this look. Decoration only: the name under it says which. -->
      <!-- The other looks are written as changes to Ember Dark, so each preview starts from it. -->
      <span class="base" data-theme="ember-dark" aria-hidden="true">
      <span class="mini" data-theme={t.id}>
        <span class="m-head">
          <span class="m-ring"></span>
          <span class="m-lines"><i></i><i class="short"></i></span>
        </span>
        <span class="m-body">
          <span class="m-go"><i></i></span>
          <span class="m-tiles"><b class="one">6,840</b><b class="two">4,210</b></span>
        </span>
        <span class="m-nav"><i class="pill"></i><i></i><i></i><i></i><i></i></span>
      </span>
      </span>
      <span class="words">
        <span class="name">{t.name}</span>
        <span class="line">{t.line}</span>
      </span>
      {#if on}<span class="tick"><Check size={13} strokeWidth={3} /></span>{/if}
    </button>
  {/each}
</div>

<style>
  .picker {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--space-2);
    outline: none;
  }

  .choice {
    position: relative;
    display: grid;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-2) var(--space-2);
    border: var(--line-w) solid var(--line);
    border-radius: var(--radius-lg);
    background: var(--surface);
    box-shadow: var(--card-shadow);
    color: var(--text);
    text-align: left;
    cursor: pointer;
    transition: border-color 0.12s ease;
  }

  .choice:hover {
    border-color: var(--border-strong);
  }

  .choice.on {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  .choice:focus-visible {
    outline: 2px solid var(--accent-text);
    outline-offset: 2px;
  }

  .words {
    display: grid;
    padding: 0 var(--space-1);
  }

  .name {
    font-family: var(--font-display);
    font-weight: var(--weight-strong);
  }

  .line {
    color: var(--faint);
    font-size: var(--text-xs);
  }

  .tick {
    position: absolute;
    right: 10px;
    bottom: 12px;
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: var(--accent);
    color: var(--accent-ink);
  }

  .base {
    display: block;
  }

  /* The tiny window. Everything in it uses the look named on .mini, not the page's. */
  .mini {
    display: flex;
    flex-direction: column;
    height: 104px;
    border: var(--line-w) solid var(--window-line);
    border-radius: 10px;
    background: var(--bg);
    overflow: hidden;
  }

  .mini i,
  .mini b {
    display: block;
  }

  .m-head {
    display: flex;
    align-items: center;
    gap: var(--space-1-5);
    padding: var(--space-1-5) var(--space-2) var(--space-1-5);
    background: var(--header-bg);
    border-radius: var(--header-radius);
  }

  .m-ring {
    width: 16px;
    height: 16px;
    border: 3px solid var(--header-track);
    border-top-color: var(--exp);
    border-right-color: var(--exp);
    border-radius: 50%;
  }

  .m-lines {
    display: grid;
    gap: var(--space-0-5);
  }

  .m-lines i {
    width: 44px;
    height: 4px;
    border-radius: 2px;
    background: var(--header-text);
  }

  .m-lines i.short {
    width: 26px;
    background: var(--header-faint);
  }

  .m-body {
    display: grid;
    flex: 1;
    align-content: center;
    gap: var(--space-1);
    padding: var(--space-1) var(--space-2);
  }

  .m-go {
    padding: var(--space-1) var(--space-1-5);
    border: var(--line-w) solid var(--tile-border);
    border-radius: 6px;
    background: var(--accent);
  }

  .m-go i {
    width: 60%;
    height: 4px;
    border-radius: 2px;
    background: var(--accent-ink);
  }

  .m-tiles {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-1);
  }

  .m-tiles b {
    padding: var(--space-0-5) var(--space-1-5);
    border: var(--line-w) solid var(--tile-border);
    border-radius: 6px;
    color: var(--tile-text);
    font-family: var(--font-display);
    font-size: var(--text-2xs);
    font-weight: var(--weight-strong);
    line-height: 14px;
  }

  .m-tiles .one {
    background: var(--tile-1);
  }

  .m-tiles .two {
    background: var(--tile-2);
    color: var(--tile-mesos);
  }

  .m-nav {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    padding: var(--space-1) var(--space-2);
    border-top: var(--line-w) solid var(--nav-line);
    background: var(--nav-bg);
  }

  .m-nav i {
    flex: 1;
    height: 5px;
    border-radius: 3px;
    background: var(--nav-text);
    opacity: 0.55;
  }

  .m-nav i.pill {
    height: 9px;
    border-radius: 4px;
    background: var(--accent);
    opacity: 1;
  }

  @media (max-width: 380px) {
    .mini {
      height: 92px;
    }
  }
</style>
