<script lang="ts">
  // The window's header: who you are and how far through the level, with four actions. The level
  // sits in a ring that fills with EXP; under it one quiet line gives the exact numbers and when
  // the level comes. On tabs that don't show the session, that line gives the two rates in place
  // of the exact EXP, so they're always in view. The name opens the list of characters, to pick
  // whose numbers the tabs show. The whole header drags the window.
  import { can } from "$lib/platform";
  import { ChevronDown, LayoutGrid, Lock, LockOpen, Pin, PinOff, Search } from "@lucide/svelte";

  import Logo from "$lib/components/Logo.svelte";
  import { characters, switched, watchSwitches } from "$lib/characters.svelte";
  import { formatClockIn, formatCompact, formatDuration, formatNumber } from "$lib/format";
  import { ago, lastKnown, profile, watchProfile } from "$lib/profile.svelte";
  import { live } from "$lib/readings.svelte";
  import { place, regionHue, watchRegion } from "$lib/region.svelte";
  import { hotkey, keyHint } from "$lib/settings.svelte";
  import { tip } from "$lib/tip.svelte";
  import IconButton from "$lib/ui/IconButton.svelte";
  import LevelRing from "$lib/ui/LevelRing.svelte";

  let {
    liveTitle,
    pinned,
    pinTitle,
    searching,
    tooling,
    choosing = false,
    rates = false,
    onlive,
    onsearch,
    onwho,
    onpin,
    onlock,
    ontools,
  }: {
    /** The tooltip of the Live / Offline switch. */
    liveTitle: string;
    pinned: boolean;
    pinTitle: string;
    /** The search sheet is open. */
    searching: boolean;
    /** The Tools list is open. */
    tooling: boolean;
    /** The list of characters is open. */
    choosing?: boolean;
    /** Show the session's two rates, for a tab that doesn't show them itself. */
    rates?: boolean;
    onlive: () => void;
    onsearch: () => void;
    /** Opens the list of characters. */
    onwho: () => void;
    onpin: () => void;
    onlock: () => void;
    ontools: () => void;
  } = $props();

  watchProfile();
  watchSwitches();
  /** The HUD switched character by itself (a log-in): said for a few seconds. */
  const fresh = $derived(switched.name != null && !characters.all && switched.name === characters.active);

  // Read now when this character is on screen; else what was read of them last, faded and dated.
  const hud = $derived(profile.live ? (live.analysis?.hud ?? null) : null);
  const session = $derived(profile.live ? live.session : null);
  const known = $derived(profile.known);
  const levelShown = $derived(lastKnown(hud?.level ?? session?.level, known?.level, known?.seenAt));
  const level = $derived(levelShown.value);
  const name = $derived(session?.name ?? hud?.name ?? profile.name);
  const job = $derived(lastKnown(hud?.job ?? session?.job, known?.job, known?.seenAt));
  const percentShown = $derived(
    lastKnown(hud?.exp?.percent ?? hud?.expPercent ?? session?.percent, known?.expPercent, known?.seenAt),
  );
  const percent = $derived(percentShown.value);
  const exp = $derived(
    percentShown.at == null ? (hud?.exp ?? session?.exp ?? null) : known?.expPoints != null ? { points: known.expPoints } : null,
  );
  /** When what is shown was read, for a character not on screen now. */
  const seenAt = $derived(levelShown.at ?? percentShown.at);

  watchRegion();
  const hue = $derived(regionHue(place.region));
  const state = $derived(live.liveStatus.state);
  const liveOn = $derived(state === "reading" || state === "waiting");
  const locked = $derived(live.status.clickThrough);

  /** When the level comes at this pace; nothing while paused or before a pace is known. */
  const toLevel = $derived(session && !session.paused && session.secondsToLevel != null ? session.secondsToLevel : null);
  /** The session's EXP and mesos an hour, short, once a session is under way. */
  const rateLine = $derived.by(() => {
    if (!rates || !session || session.activeSeconds <= 0) return null;
    const exp = session.percentOnly
      ? session.percentPerHour != null
        ? `${session.percentPerHour.toFixed(1)}%`
        : null
      : session.expPerHour != null
        ? formatCompact(session.expPerHour)
        : null;
    const mesos = session.mesosPerHour != null ? formatCompact(session.mesosPerHour) : null;
    return exp || mesos ? { exp, mesos, levelRate: session.percentOnly } : null;
  });
  const lockTitle = $derived(
    locked
      ? hotkey("ToggleClickThrough")
        ? `Unlock with ${hotkey("ToggleClickThrough")}`
        : "Unlock: set a Lock hotkey in Settings, or use the tray menu"
      : `Lock, so clicks pass through to the game${keyHint("ToggleClickThrough")}`,
  );
</script>

<header class:washed={hue != null} style:--region-hue={hue} data-tauri-drag-region>
  <div class="top" data-tauri-drag-region>
    <span class="ring" class:stale={levelShown.at != null}><LevelRing {level} {percent}><Logo size={20} /></LevelRing></span>
    <div class="who" data-tauri-drag-region>
      <button class="name" class:fresh aria-haspopup="dialog" aria-expanded={choosing} aria-label="{characters.all ? 'All characters' : (name ?? 'No character yet')}. Pick a character" onclick={onwho}>
        <span>{characters.all ? "All characters" : (name ?? "Leafline")}</span><ChevronDown size={15} />
      </button>
      <div class="under" data-tauri-drag-region>
        {#if fresh}<span class="switched" use:tip={`The game shows ${name} now, so Leafline switched to them`} data-tauri-drag-region>Switched</span>
        {:else if job.value}<span class:stale={job.at != null} data-tauri-drag-region>{job.value}</span>{/if}
        {#if seenAt != null}<span class="stale" use:tip={`Last read ${ago(seenAt)}. Live reading brings it up to date.`} data-tauri-drag-region>{ago(seenAt)}</span>{/if}
        {#if can.liveReading}<button class="live" class:on={state === "reading"} class:waiting={state === "waiting"} use:tip={liveTitle} onclick={onlive}>
          <span class="dot" aria-hidden="true"></span>{liveOn ? "Live" : "Offline"}
        </button>{/if}
      </div>
    </div>
    <div class="actions">
      <IconButton tone="header" label="Search everything" active={searching} onclick={onsearch}><Search size={17} /></IconButton>
      {#if can.pins}<IconButton tone="header" label={pinTitle} active={pinned} onclick={onpin}>
        {#if pinned}<PinOff size={17} />{:else}<Pin size={17} />{/if}
      </IconButton>{/if}
      {#if can.clickThrough}<IconButton tone="header" label={locked ? "Unlock" : "Lock"} title={lockTitle} active={locked} onclick={onlock}>
        {#if locked}<Lock size={17} />{:else}<LockOpen size={17} />{/if}
      </IconButton>{/if}
      <IconButton tone="header" label="Tools" title="Tools, and Leafline's windows and settings" active={tooling} onclick={ontools}>
        <LayoutGrid size={17} />
      </IconButton>
    </div>
  </div>

  {#if percent != null}
    <div class="exp" class:stale={percentShown.at != null} data-tauri-drag-region>
      <span class="amount" data-tauri-drag-region>
        <span class="pct">{percent.toFixed(percent >= 100 ? 0 : 1)}%</span>
        {#if rateLine}
          <span class="points rates">
            {#if rateLine.exp}<span><b>{rateLine.exp}</b> {rateLine.levelRate ? "a level" : "EXP"}/hr</span>{/if}
            {#if rateLine.mesos}<span><b class="mesos">{rateLine.mesos}</b> mesos/hr</span>{/if}
          </span>
        {:else if exp}
          <span class="points">
            {formatNumber(exp.points)}{#if session?.expNeeded}{" "}/ {formatNumber(session.expNeeded)}{/if} EXP
          </span>
        {/if}
      </span>
      {#if toLevel != null && level != null}
        <span class="when" use:tip={formatClockIn(toLevel) ? `About ${formatClockIn(toLevel)} at this pace` : null}>
          Lv {level + 1} in <b>~{formatDuration(toLevel)}</b>
        </span>
      {/if}
    </div>
  {/if}
</header>

<style>
  header {
    position: relative;
    isolation: isolate;
    flex: none;
    margin-bottom: var(--header-gap);
    border-radius: var(--header-radius);
    background: var(--header-bg);
    color: var(--header-text);
    user-select: none;
    cursor: grab;
  }

  /* Remembered from an earlier read, not read now. */
  .stale {
    opacity: 0.55;
  }

  .ring {
    display: contents;
  }

  .ring.stale {
    display: block;
    flex: none;
  }

  /* The region you're in, as a soft glow from the top left, fading to the next when you travel. */
  header::before {
    content: "";
    position: absolute;
    inset: 0;
    z-index: -1;
    border-radius: inherit;
    background: radial-gradient(90% 140% at 0% 0%, hsl(var(--region-hue) 62% 52% / 0.22), transparent 70%);
    opacity: 0;
    transition:
      opacity 0.8s ease,
      --region-hue 1.2s ease;
    pointer-events: none;
  }

  header.washed::before {
    opacity: 1;
  }

  .top {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-2) var(--space-2) 14px;
  }

  .who {
    flex: 1;
    min-width: 0;
  }

  /* The name is the character switch: it opens the list of characters. */
  .name {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    max-width: 100%;
    margin: -2px 0 -2px calc(-1 * var(--space-1-5));
    padding: 2px var(--space-1-5);
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: inherit;
    font-family: var(--font-display);
    font-size: var(--text-lg);
    font-weight: var(--weight-strong);
    line-height: 19px;
    text-align: left;
    cursor: pointer;
  }

  .name span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .name :global(svg) {
    flex: none;
    color: var(--header-faint);
  }

  .name:hover,
  .name[aria-expanded="true"] {
    background: var(--header-hover);
  }

  .name:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  /* A switch the HUD made by itself, outlined while it is said. */
  .name.fresh {
    box-shadow: inset 0 0 0 1px var(--header-accent);
  }

  .switched {
    color: var(--header-good);
    font-weight: var(--weight-strong);
  }

  .under {
    display: flex;
    align-items: center;
    gap: var(--space-1-5);
    margin-top: 1px;
    color: var(--header-muted);
    font-size: var(--text-md);
    white-space: nowrap;
  }

  /* Live reading's state, and the switch for it. */
  .live {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    padding: 1px var(--space-1-5) 1px var(--space-1);
    margin-left: -4px;
    border: 0;
    border-radius: var(--radius-pill);
    background: transparent;
    color: var(--header-faint);
    font: inherit;
    font-weight: var(--weight-strong);
    cursor: pointer;
  }

  .live:hover {
    background: var(--header-hover);
  }

  .live:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  .live.on {
    color: var(--header-good);
  }

  .live.waiting {
    color: var(--header-accent);
  }

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: currentColor;
  }

  .live.on .dot {
    animation: pulse 1.6s ease-in-out infinite;
  }

  @keyframes pulse {
    50% {
      opacity: 0.35;
    }
  }

  .actions {
    display: flex;
    flex: none;
  }

  .exp {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--space-2);
    padding: 0 var(--space-4) var(--space-3);
    font-size: var(--text-xs);
    white-space: nowrap;
  }

  .amount {
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
    min-width: 0;
  }

  .pct {
    color: var(--header-exp);
    font-family: var(--font-display);
    font-size: var(--text-lg);
    font-weight: var(--weight-strong);
    font-variant-numeric: tabular-nums;
  }

  .points {
    overflow: hidden;
    color: var(--header-faint);
    font-variant-numeric: tabular-nums;
    text-overflow: ellipsis;
  }

  .rates {
    display: flex;
    gap: var(--space-3);
    color: var(--header-muted);
  }

  .rates b {
    color: var(--header-text);
    font-family: var(--font-display);
    font-weight: var(--weight-strong);
  }

  /* The header stays dark in the light looks, so the mesos take the header's own amber. */
  .rates b.mesos {
    color: var(--header-accent);
  }

  .when {
    color: var(--header-muted);
  }

  .when b {
    color: var(--header-text);
    font-weight: var(--weight-strong);
  }

  /* Dragged narrow: the exact points give way before the time to level does. */
  @media (max-width: 340px) {
    .points {
      display: none;
    }
  }
</style>
