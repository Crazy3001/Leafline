<script lang="ts">
  // The website's live reading at the top of the desk's sidebar: one pill that says where it stands
  // by its colour, with the way to start or stop beside it. Red: not reading, and Share starts it.
  // Amber: shared, but the game can't be read (or reads less well). Green: reading live. The banner
  // over the views explains it and says what to fix (ShareBanner); the tooltip says the same here.
  // The ban risk line is in the sidebar's foot. Folded, it is the coloured dot alone.
  import { MonitorPlay, RotateCcw, Square } from "@lucide/svelte";
  import { onMount } from "svelte";

  import { characters } from "$lib/characters.svelte";
  import { can } from "$lib/platform";
  import { shareState, startSharing, stopSharing } from "$lib/sharing.svelte";
  import { watchSight } from "$lib/sight.svelte";
  import { tip } from "$lib/tip.svelte";

  let { folded = false }: { folded?: boolean } = $props();

  if (can.readWindow) watchSight();
  const s = $derived(shareState());
  /** Sharing is under way: the button stops it. */
  const sharing = $derived(s.tone === "picking" || s.tone === "stuck" || s.tone === "reading");
  const colour = $derived(s.tone === "reading" ? (s.note ? "amber" : "green") : s.tone === "off" || s.tone === "failed" ? "red" : "amber");

  // Sessions go under the name read from the HUD; until one is read, under the character picked.
  onMount(() => {
    if (__LEAFLINE_WEB__) void import("$lib/platform/live").then((m) => m.setLiveCharacter(() => characters.viewing));
  });
</script>

{#if can.readWindow}
  {#if folded}
    <button
      class="fold-btn {colour}"
      class:pulse={s.tone === "reading"}
      aria-label="{s.label}. {sharing ? 'Stop reading' : 'Share the game window'}"
      use:tip={`${s.label}. Click to ${sharing ? "stop" : "share the game window"}.\n${s.detail}`}
      onclick={() => (sharing ? stopSharing() : void startSharing())}><span class="dot" aria-hidden="true"></span></button
    >
  {:else}
    <div class="pill {colour}" class:pulse={s.tone === "reading"} role="status" use:tip={s.detail}>
      <span class="dot" aria-hidden="true"></span><span class="what">{s.label}</span>
      {#if sharing}
        <button class="act" onclick={stopSharing}><Square size={11} />Stop</button>
      {:else}
        <button class="act go" onclick={() => void startSharing()}
          >{#if s.tone === "failed"}<RotateCcw size={12} />Try again{:else}<MonitorPlay size={12} />Share{/if}</button
        >
      {/if}
    </div>
  {/if}
{/if}

<style>
  /* As wide as the sidebar, and as tall as its rows of tabs and more. */
  .pill {
    display: flex;
    align-items: center;
    gap: var(--space-1-5);
    width: 100%;
    min-height: 36px;
    padding: 0 var(--space-1) 0 var(--space-3);
    border: var(--line-w) solid color-mix(in srgb, var(--tone) 45%, transparent);
    border-radius: var(--radius-pill);
    background: color-mix(in srgb, var(--tone) 10%, var(--surface));
    color: var(--tone);
    font-size: var(--text-md);
    font-weight: var(--weight-strong);
  }

  .green {
    --tone: var(--good);
  }

  .amber {
    --tone: var(--warn);
  }

  .red {
    --tone: var(--bad);
  }

  .what {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .dot {
    flex: none;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: currentColor;
  }

  .pulse .dot {
    animation: pulse 1.6s ease-in-out infinite;
  }

  @keyframes pulse {
    50% {
      opacity: 0.35;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .pulse .dot {
      animation: none;
    }
  }

  .act {
    display: inline-flex;
    flex: none;
    align-items: center;
    gap: var(--space-1);
    height: 26px;
    padding: 0 var(--space-2);
    border: 0;
    border-radius: var(--radius-pill);
    background: var(--surface-2);
    color: var(--text);
    font: inherit;
    font-size: var(--text-xs);
    font-weight: var(--weight-body);
    cursor: pointer;
  }

  .act:hover {
    background: var(--surface-3);
  }

  /* Not reading: the way to start stands out. */
  .act.go {
    background: var(--accent);
    color: var(--accent-ink);
    font-weight: var(--weight-strong);
  }

  .act.go:hover {
    background: var(--accent-strong);
  }

  .act:focus-visible,
  .fold-btn:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  /* Folded: the coloured dot alone, in a ring of its colour. */
  .fold-btn {
    display: inline-grid;
    flex: none;
    place-items: center;
    width: 36px;
    height: 36px;
    padding: 0;
    border: var(--line-w) solid color-mix(in srgb, var(--tone) 45%, transparent);
    border-radius: 50%;
    background: color-mix(in srgb, var(--tone) 10%, var(--surface));
    color: var(--tone);
    cursor: pointer;
  }

  .fold-btn .dot {
    width: 10px;
    height: 10px;
  }
</style>
