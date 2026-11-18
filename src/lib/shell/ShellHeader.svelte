<script lang="ts">
  // The window's header: who you are and how far through the level, with four actions. The level
  // sits in a ring that fills with EXP; under it one quiet line gives the exact numbers and when
  // the level comes. The whole header drags the window.
  import { LayoutGrid, Lock, LockOpen, Pin, PinOff, Search } from "@lucide/svelte";

  import Logo from "$lib/components/Logo.svelte";
  import { formatClockIn, formatDuration, formatNumber } from "$lib/format";
  import { live } from "$lib/readings.svelte";
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
    onlive,
    onsearch,
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
    onlive: () => void;
    onsearch: () => void;
    onpin: () => void;
    onlock: () => void;
    ontools: () => void;
  } = $props();

  const hud = $derived(live.analysis?.hud ?? null);
  const session = $derived(live.session);
  const level = $derived(hud?.level ?? session?.level ?? null);
  const name = $derived(session?.name ?? hud?.name ?? null);
  const job = $derived(hud?.job ?? session?.job ?? null);
  const exp = $derived(hud?.exp ?? session?.exp ?? null);
  const percent = $derived(hud?.exp?.percent ?? hud?.expPercent ?? session?.percent ?? null);
  const state = $derived(live.liveStatus.state);
  const liveOn = $derived(state === "reading" || state === "waiting");
  const locked = $derived(live.status.clickThrough);

  /** When the level comes at this pace; nothing while paused or before a pace is known. */
  const toLevel = $derived(session && !session.paused && session.secondsToLevel != null ? session.secondsToLevel : null);
  const lockTitle = $derived(
    locked
      ? hotkey("ToggleClickThrough")
        ? `Unlock with ${hotkey("ToggleClickThrough")}`
        : "Unlock: set a Lock hotkey in Settings, or use the tray menu"
      : `Lock, so clicks pass through to the game${keyHint("ToggleClickThrough")}`,
  );
</script>

<header data-tauri-drag-region>
  <div class="top" data-tauri-drag-region>
    <LevelRing {level} {percent}><Logo size={20} /></LevelRing>
    <div class="who" data-tauri-drag-region>
      <div class="name" data-tauri-drag-region>{name ?? "Leafline"}</div>
      <div class="under" data-tauri-drag-region>
        {#if job}<span data-tauri-drag-region>{job}</span>{/if}
        <button class="live" class:on={state === "reading"} class:waiting={state === "waiting"} use:tip={liveTitle} onclick={onlive}>
          <span class="dot" aria-hidden="true"></span>{liveOn ? "Live" : "Offline"}
        </button>
      </div>
    </div>
    <div class="actions">
      <IconButton tone="header" label="Search everything" active={searching} onclick={onsearch}><Search size={17} /></IconButton>
      <IconButton tone="header" label={pinTitle} active={pinned} onclick={onpin}>
        {#if pinned}<PinOff size={17} />{:else}<Pin size={17} />{/if}
      </IconButton>
      <IconButton tone="header" label={locked ? "Unlock" : "Lock"} title={lockTitle} active={locked} onclick={onlock}>
        {#if locked}<Lock size={17} />{:else}<LockOpen size={17} />{/if}
      </IconButton>
      <IconButton tone="header" label="Tools" title="Tools, and Leafline's windows and settings" active={tooling} onclick={ontools}>
        <LayoutGrid size={17} />
      </IconButton>
    </div>
  </div>

  {#if percent != null}
    <div class="exp" data-tauri-drag-region>
      <span class="amount" data-tauri-drag-region>
        <span class="pct">{percent.toFixed(percent >= 100 ? 0 : 1)}%</span>
        {#if exp}
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
    flex: none;
    margin-bottom: var(--header-gap);
    border-radius: var(--header-radius);
    background: var(--header-bg);
    color: var(--header-text);
    user-select: none;
    cursor: grab;
  }

  .top {
    display: flex;
    align-items: center;
    gap: 11px;
    padding: 12px 8px 10px 14px;
  }

  .who {
    flex: 1;
    min-width: 0;
  }

  .name {
    overflow: hidden;
    font-family: var(--font-display);
    font-size: var(--text-lg);
    font-weight: var(--weight-strong);
    line-height: 19px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .under {
    display: flex;
    align-items: center;
    gap: 7px;
    margin-top: 1px;
    color: var(--header-muted);
    font-size: var(--text-sm);
    white-space: nowrap;
  }

  /* Live reading's state, and the switch for it. */
  .live {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 1px 6px 1px 4px;
    margin-left: -4px;
    border: 0;
    border-radius: var(--radius-pill);
    background: transparent;
    color: var(--header-faint);
    font: inherit;
    font-weight: 600;
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
    gap: 10px;
    padding: 0 14px 11px;
    font-size: 11.5px;
    white-space: nowrap;
  }

  .amount {
    display: flex;
    align-items: baseline;
    gap: 8px;
    min-width: 0;
  }

  .pct {
    color: var(--header-exp);
    font-family: var(--font-display);
    font-size: 14px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }

  .points {
    overflow: hidden;
    color: var(--header-faint);
    font-variant-numeric: tabular-nums;
    text-overflow: ellipsis;
  }

  .when {
    color: var(--header-muted);
  }

  .when b {
    color: var(--header-text);
    font-weight: 600;
  }

  /* Dragged narrow: the exact points give way before the time to level does. */
  @media (max-width: 340px) {
    .points {
      display: none;
    }
  }
</style>
