<script lang="ts">
  // The website's desk, over the views, while it isn't reading the game: what sharing the game
  // window does and the button to start it. While the browser's list is open, which one to pick.
  // Shared but the game can't be read for a few seconds (not a loading screen's moment), amber,
  // with what to fix; the browser stopping the share, red. Gone while reading. "Not now" puts the
  // first one away until the page is opened again; the sidebar's red pill stays.
  import { TriangleAlert, MonitorPlay, RotateCcw, Square, X } from "@lucide/svelte";

  import { can } from "$lib/platform";
  import { screensOnly, shareState, startSharing, stopSharing } from "$lib/sharing.svelte";
  import { watchSight } from "$lib/sight.svelte";

  /** How long the game can't be read before the banner says so: a map change or loading screen passes. */
  const GRACE_MS = 5000;
  const NOT_NOW = "leafline.share.notNow";

  if (can.readWindow) watchSight();
  const s = $derived(shareState());

  let notNow = $state(readNotNow());
  function readNotNow() {
    try {
      return sessionStorage.getItem(NOT_NOW) === "1";
    } catch {
      return false;
    }
  }
  function putAway() {
    notNow = true;
    try {
      sessionStorage.setItem(NOT_NOW, "1");
    } catch {
      // For this page only, then.
    }
  }

  let stuckLong = $state(false);
  $effect(() => {
    if (s.tone !== "stuck") {
      stuckLong = false;
      return;
    }
    const timer = setTimeout(() => (stuckLong = true), GRACE_MS);
    return () => clearTimeout(timer);
  });

  const shown = $derived(
    can.readWindow && ((s.tone === "off" && !notNow) || s.tone === "picking" || s.tone === "failed" || (s.tone === "stuck" && stuckLong)),
  );
</script>

{#if shown}
  <div class="banner {s.tone}" role={s.tone === "off" ? "region" : "status"} aria-label="Reading the game">
    {#if s.tone === "off"}
      <span class="icon"><MonitorPlay size={18} /></span>
      <div class="text">
        <p class="title">Leafline isn't reading your game yet</p>
        <p>
          Share the MapleStory window and it reads your level, EXP, mesos, map, quests and loot as you play. The picture stays
          on this computer.{#if screensOnly}{" "}This browser shares whole screens: pick the one the game is on.{/if}
        </p>
      </div>
      <div class="actions">
        <button class="primary" onclick={() => void startSharing()}><MonitorPlay size={14} />Share game window</button>
        <button class="quiet" aria-label="Not now" onclick={putAway}><X size={14} />Not now</button>
      </div>
    {:else if s.tone === "picking"}
      <span class="icon"><MonitorPlay size={18} /></span>
      <div class="text">
        <p class="title">{screensOnly ? "Pick the screen the game is on" : "Pick the MapleStory window"}</p>
        <p>In your browser's list, then press Share. The game window must not be minimised.</p>
      </div>
    {:else if s.tone === "stuck"}
      <span class="icon"><TriangleAlert size={18} /></span>
      <div class="text">
        <p class="title">Sharing, but Leafline can't read the game</p>
        <p>{s.detail}</p>
      </div>
      <div class="actions">
        <button class="plain" onclick={stopSharing}><Square size={12} />Stop reading</button>
      </div>
    {:else}
      <span class="icon"><TriangleAlert size={18} /></span>
      <div class="text">
        <p class="title">Sharing stopped</p>
        <p>{s.detail}</p>
      </div>
      <div class="actions">
        <button class="primary" onclick={() => void startSharing()}><RotateCcw size={14} />Try again</button>
      </div>
    {/if}
  </div>
{/if}

<style>
  .banner {
    --tone: var(--accent);
    display: flex;
    align-items: center;
    gap: var(--space-3);
    margin: var(--space-3) var(--space-4) 0;
    padding: var(--space-2) var(--space-3) var(--space-2) var(--space-4);
    border: var(--line-w) solid color-mix(in srgb, var(--tone) 40%, transparent);
    border-left-width: 4px;
    border-radius: var(--radius-lg);
    background: color-mix(in srgb, var(--tone) 9%, var(--surface));
  }

  .banner.off {
    --tone: var(--bad);
  }

  .banner.stuck,
  .banner.picking {
    --tone: var(--warn);
  }

  .banner.failed {
    --tone: var(--bad);
  }

  .icon {
    display: inline-grid;
    flex: none;
    place-items: center;
    color: var(--tone);
  }

  .text {
    flex: 1;
    min-width: 0;
  }

  p {
    margin: 0;
    color: var(--muted);
    font-size: var(--text-md);
    line-height: 1.4;
  }

  .title {
    margin-bottom: 2px;
    color: var(--text);
    font-size: var(--text-md);
    font-weight: var(--weight-strong);
  }

  .actions {
    display: flex;
    flex: none;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
  }

  button {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1-5);
    height: var(--control-h);
    padding: 0 var(--space-3);
    border: var(--line-w) solid transparent;
    border-radius: var(--radius-pill);
    font: inherit;
    font-size: var(--text-md);
    white-space: nowrap;
    cursor: pointer;
  }

  .primary {
    background: var(--accent);
    color: var(--accent-ink);
    font-weight: var(--weight-strong);
  }

  .primary:hover {
    background: var(--accent-strong);
  }

  .plain {
    border-color: var(--control-line);
    background: var(--surface);
    color: var(--text);
  }

  .plain:hover {
    background: var(--surface-2);
  }

  .quiet {
    padding: 0 var(--space-2);
    background: transparent;
    color: var(--muted);
  }

  .quiet:hover {
    color: var(--text);
  }

  button:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  /* A narrow window: the buttons go under the words. */
  @container desk-main (max-width: 760px) {
    .banner {
      flex-wrap: wrap;
    }

    .actions {
      flex-basis: 100%;
      padding-left: calc(18px + var(--space-3));
    }
  }
</style>
