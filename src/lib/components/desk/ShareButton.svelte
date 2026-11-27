<script lang="ts">
  // The website's way into live reading, at the top of the desk's sidebar: share the game window
  // and Leafline reads the game from it (platform/live.ts); Stop reading while it does. It is where
  // the website says whether it's reading; the player bar leaves that to it. What it reads and how
  // to share well is in its tooltip; the ban risk line is in the sidebar's foot. Folded, it is one
  // icon: share, or the reading dot that stops.
  import { MonitorPlay, Square } from "@lucide/svelte";
  import { onMount } from "svelte";

  import { backend } from "$lib/api";
  import { characters } from "$lib/characters.svelte";
  import { can } from "$lib/platform";
  import { support } from "$lib/platform/capture";
  import { live } from "$lib/readings.svelte";
  import { pictureNotes, pictureProblem, sight, watchSight } from "$lib/sight.svelte";
  import { tip } from "$lib/tip.svelte";

  let { folded = false }: { folded?: boolean } = $props();

  const status = $derived(live.liveStatus);
  const on = $derived(status.state === "reading" || status.state === "waiting");
  let problem = $state<string | null>(null);

  if (can.readWindow) watchSight();
  /**
   * While reading, a picture Leafline can't use (black, frozen, washed out by Windows HDR, no
   * status bar), or one it reads less well: the button turns amber and its tooltip says what to do.
   */
  const unusable = $derived(status.state === "reading" && sight.value?.picture ? pictureProblem() : null);
  const issue = $derived(unusable ?? (status.state === "reading" ? (pictureNotes()[0] ?? null) : null));

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

  const label = $derived(status.state === "waiting" ? "Waiting…" : unusable ? "Can't read the picture" : "Reading live");
  const liveTip = $derived(status.state === "waiting" ? status.reason : (issue ?? READING_TIP));
  const failed = $derived(problem ?? (status.state === "failed" ? status.message : null));

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
  {#if folded}
    {#if on}
      <button
        class="fold-btn live {status.state}"
        class:warn={issue}
        aria-label="{label}. Stop reading"
        use:tip={`${label}. Click to stop.\n${liveTip}`}
        onclick={() => backend.stopLive()}><span class="dot" aria-hidden="true"></span></button
      >
    {:else}
      <button class="fold-btn share" class:failed aria-label="Share the game window" use:tip={failed ?? SHARE_TIP} onclick={start}
        ><MonitorPlay size={16} /></button
      >
    {/if}
  {:else if on}
    <!-- Whether it's reading, and the way to stop, in one place. -->
    <div class="live {status.state}" class:warn={issue} role="status" use:tip={liveTip}>
      <span class="dot" aria-hidden="true"></span><span class="what">{label}</span>
      <button class="stop" onclick={() => backend.stopLive()}><Square size={11} />Stop</button>
    </div>
  {:else}
    <button class="share" use:tip={SHARE_TIP} onclick={start}><MonitorPlay size={15} />Share game window</button>
    {#if failed}
      <p class="problem" role="status">{failed}</p>
    {/if}
  {/if}
{/if}

<style>
  /* As wide as the sidebar, and as tall as its rows of tabs and more. */
  .share,
  .live {
    display: flex;
    align-items: center;
    gap: var(--space-1-5);
    width: 100%;
    min-height: 36px;
    border-radius: var(--radius-pill);
    font: inherit;
    font-size: var(--text-md);
    font-weight: var(--weight-strong);
  }

  .share {
    justify-content: center;
    padding: var(--space-1) var(--space-3);
    border: 0;
    background: var(--accent);
    color: var(--accent-ink);
    text-align: center;
    cursor: pointer;
  }

  .share:hover {
    background: var(--accent-strong);
  }

  .share:focus-visible,
  .fold-btn:focus-visible,
  .stop:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .live {
    padding: 0 var(--space-1) 0 var(--space-3);
    border: var(--line-w) solid color-mix(in srgb, var(--good) 45%, transparent);
    background: color-mix(in srgb, var(--good) 10%, var(--surface));
    color: var(--good);
  }

  .what {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .live.waiting,
  .live.warn {
    border-color: color-mix(in srgb, var(--warn) 45%, transparent);
    background: color-mix(in srgb, var(--warn) 10%, var(--surface));
    color: var(--warn);
  }

  .dot {
    flex: none;
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

  .stop:hover {
    background: var(--surface-3);
  }

  .problem {
    margin: var(--space-1) var(--space-1) 0;
    color: var(--warn);
    font-size: var(--text-xs);
    line-height: 1.35;
  }

  /* Folded: one round icon, the same green to share, the reading dot to stop. */
  .fold-btn {
    display: inline-flex;
    flex: none;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    min-height: 0;
    padding: 0;
    border-radius: 50%;
    cursor: pointer;
  }

  .fold-btn.share.failed {
    box-shadow: 0 0 0 2px var(--warn);
  }

  .fold-btn.live {
    gap: 0;
  }

  .fold-btn.live .dot {
    width: 9px;
    height: 9px;
  }

  .fold-btn.live:hover .dot {
    border-radius: 2px;
  }
</style>
