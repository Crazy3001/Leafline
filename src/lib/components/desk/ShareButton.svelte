<script lang="ts">
  // The website's way into live reading, on the desk's top row: share the game window and Leafline
  // reads the game from it (platform/live.ts); Stop reading while it does. Its state is the same
  // live status the sidebar shows. What it reads and how to share well is in its tooltip; the ban
  // risk line is in the sidebar's foot.
  import { MonitorPlay, Square } from "@lucide/svelte";
  import { onMount } from "svelte";

  import { backend } from "$lib/api";
  import { characters } from "$lib/characters.svelte";
  import { can } from "$lib/platform";
  import { support } from "$lib/platform/capture";
  import { live } from "$lib/readings.svelte";
  import { tip } from "$lib/tip.svelte";

  const status = $derived(live.liveStatus);
  const on = $derived(status.state === "reading" || status.state === "waiting");
  let problem = $state<string | null>(null);

  // Sessions go under the name read from the HUD; until one is read, under the character picked.
  onMount(() => {
    if (__LEAFLINE_WEB__) void import("$lib/platform/live").then((m) => m.setLiveCharacter(() => characters.viewing));
  });

  /** A browser without a frame-by-frame stream reader (Firefox's route) shares screens, not a game's own window. */
  const screensOnly = support.share && !support.trackFrames;

  const SHARE_TIP =
    "Leafline reads your level, EXP, mesos, map, quests and loot from the game window. The picture stays on this computer.\n" +
    "Keep the HUD and minimap in view." +
    (screensOnly ? "\nThis browser shares a whole screen, not one window: pick the screen the game is on." : "");
  const READING_TIP =
    "Keep the HUD and minimap in view. Mesos are read while your inventory is open, quests from the Quest Helper and loot from the lines at the bottom right. The session saves itself after 5 minutes.";

  async function start() {
    problem = null;
    try {
      await backend.startLive();
    } catch (err) {
      problem = err instanceof Error ? err.message : String(err);
    }
  }
</script>

{#if can.readWindow}
  {#if on}
    <!-- Whether it's reading, and the way to stop, in one place. -->
    <span class="live {status.state}" role="status" use:tip={status.state === "waiting" ? status.reason : READING_TIP}>
      <span class="dot" aria-hidden="true"></span>{status.state === "reading" ? "Reading live" : "Waiting…"}
      <button class="stop" onclick={() => backend.stopLive()}><Square size={11} />Stop</button>
    </span>
  {:else}
    <button class="pill share" use:tip={SHARE_TIP} onclick={start}><MonitorPlay size={14} />Share the game window</button>
    {#if problem || status.state === "failed"}
      <span class="problem" role="status" use:tip={problem ?? (status.state === "failed" ? status.message : "")}
        >{problem ?? (status.state === "failed" ? status.message : "")}</span
      >
    {/if}
  {/if}
{/if}

<style>
  /* The look is the desk's top-row pill. */
  .pill {
    display: inline-flex;
    flex: none;
    align-items: center;
    gap: var(--space-1-5);
    height: var(--control-h);
    padding: 0 var(--space-3);
    border: var(--line-w) solid var(--control-line);
    border-radius: var(--radius-pill);
    background: var(--surface);
    color: var(--text);
    font: inherit;
    font-size: var(--text-md);
    white-space: nowrap;
    cursor: pointer;
  }

  .pill:hover {
    background: var(--surface-2);
  }

  .share {
    border-color: transparent;
    background: var(--accent);
    color: var(--accent-ink);
    font-weight: var(--weight-strong);
  }

  .share:hover {
    background: var(--accent-strong);
  }

  .live {
    display: inline-flex;
    flex: none;
    align-items: center;
    gap: var(--space-1-5);
    height: var(--control-h);
    padding: 0 var(--space-1) 0 var(--space-3);
    border: var(--line-w) solid color-mix(in srgb, var(--good) 45%, transparent);
    border-radius: var(--radius-pill);
    background: color-mix(in srgb, var(--good) 10%, var(--surface));
    color: var(--good);
    font-size: var(--text-md);
    font-weight: var(--weight-strong);
    white-space: nowrap;
  }

  .live.waiting {
    border-color: color-mix(in srgb, var(--warn) 45%, transparent);
    background: color-mix(in srgb, var(--warn) 10%, var(--surface));
    color: var(--warn);
  }

  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: currentColor;
  }

  .live.reading .dot {
    animation: pulse 1.6s ease-in-out infinite;
  }

  @keyframes pulse {
    50% {
      opacity: 0.35;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .live.reading .dot {
      animation: none;
    }
  }

  .stop {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    height: calc(var(--control-h) - 8px);
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

  .stop:hover,
  .stop:focus-visible {
    background: var(--surface-3);
    outline: none;
  }

  .problem {
    max-width: 260px;
    min-width: 0;
    overflow: hidden;
    color: var(--warn);
    font-size: var(--text-xs);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
