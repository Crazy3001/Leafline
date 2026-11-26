<script lang="ts">
  // Countdown timers the player starts themselves, for anything they want to keep time on.
  // A finished timer just reads "Done": no sound, popup or flashing asks the player to act.
  import { clipped, tip } from "$lib/tip.svelte";
  import { Plus, RotateCcw, Timer, X } from "@lucide/svelte";
  import { onMount } from "svelte";

  import Card from "./Card.svelte";

  interface Countdown {
    id: number;
    name: string;
    seconds: number;
    /** When it was started, in ms. */
    startedAt: number;
  }

  const KEY = "leafline.timers";
  const PRESETS = [1, 5, 10, 30];

  let {
    explain = false,
  }: {
    /** Shown on its own (a tool page), so with no timers it says what they are for. */
    explain?: boolean;
  } = $props();

  let timers = $state<Countdown[]>(load());
  let now = $state(Date.now());
  let adding = $state(false);
  let name = $state("");
  let minutes = $state(5);
  let input = $state<HTMLInputElement>();

  onMount(() => {
    const tick = setInterval(() => (now = Date.now()), 1000);
    // Other windows (pinned panels) share the same timers.
    const sync = (event: StorageEvent) => {
      if (event.key === KEY) timers = load();
    };
    window.addEventListener("storage", sync);
    return () => {
      clearInterval(tick);
      window.removeEventListener("storage", sync);
    };
  });

  $effect(() => {
    if (adding) input?.focus();
  });

  function load(): Countdown[] {
    try {
      return JSON.parse(localStorage.getItem(KEY) ?? "[]");
    } catch {
      return [];
    }
  }

  function store(next: Countdown[]) {
    timers = next;
    try {
      localStorage.setItem(KEY, JSON.stringify(next));
    } catch {
      // Timers still run in this window.
    }
  }

  function start(event?: SubmitEvent) {
    event?.preventDefault();
    const seconds = Math.round(Math.max(0.1, Math.min(600, minutes || 0)) * 60);
    const id = Math.max(0, ...timers.map((t) => t.id)) + 1;
    store([...timers, { id, name: name.trim() || `Timer ${id}`, seconds, startedAt: Date.now() }]);
    adding = false;
    name = "";
  }

  function left(timer: Countdown) {
    return Math.max(0, timer.seconds - Math.floor((now - timer.startedAt) / 1000));
  }

  function clock(seconds: number) {
    const h = Math.floor(seconds / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    const s = seconds % 60;
    return h ? `${h}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}` : `${m}:${String(s).padStart(2, "0")}`;
  }

  function onKey(event: KeyboardEvent) {
    if (event.key === "Escape") adding = false;
  }
</script>

{#if timers.length || adding}
  <Card title="Timers">
    {#snippet aside()}
      {#if !adding}
        <button class="icon-btn xs icon" use:tip={"Add a timer"} aria-label="Add a timer" onclick={() => (adding = true)}
          ><Plus size={13} /></button
        >
      {/if}
    {/snippet}

    {#if timers.length}
      <ul>
        {#each timers as timer (timer.id)}
          {@const remaining = left(timer)}
          <li class:done={remaining === 0}>
            <div class="row">
              <span use:clipped class="name">{timer.name}</span>
              <span class="time tabular">{remaining === 0 ? "Done" : clock(remaining)}</span>
              <button
                class="icon-btn xs icon"
                use:tip={"Start again"}
                aria-label="Start {timer.name} again"
                onclick={() => store(timers.map((t) => (t.id === timer.id ? { ...t, startedAt: Date.now() } : t)))}
                ><RotateCcw size={11} /></button
              >
              <button
                class="icon-btn xs icon"
                use:tip={"Remove"}
                aria-label="Remove {timer.name}"
                onclick={() => store(timers.filter((t) => t.id !== timer.id))}><X size={12} /></button
              >
            </div>
            <div class="track"><div class="fill" style:width="{(remaining / timer.seconds) * 100}%"></div></div>
          </li>
        {/each}
      </ul>
    {/if}

    {#if adding}
      <form class="form" onsubmit={start}>
        <input class="label" bind:this={input} bind:value={name} onkeydown={onKey} placeholder="Name (optional)" maxlength="30" />
        <div class="presets" role="group" aria-label="Minutes">
          {#each PRESETS as preset}
            <button type="button" aria-pressed={minutes === preset} onclick={() => (minutes = preset)}>{preset}m</button>
          {/each}
          <input
            class="minutes tabular"
            type="number"
            min="0.1"
            max="600"
            step="any"
            aria-label="Minutes"
            bind:value={minutes}
            onkeydown={onKey}
          />
        </div>
        <div class="actions">
          <button type="button" class="link" onclick={() => (adding = false)}>Cancel</button>
          <button type="submit" class="go">Start</button>
        </div>
      </form>
    {/if}
  </Card>
{:else if explain}
  <Card title="Timers">
    <p class="explain">
      Countdowns for anything you keep time on: a buff, a boss, a quest you can take again later. Each counts down here while
      Leafline is open.
    </p>
    <button class="btn ghost sm" onclick={() => (adding = true)}><Timer size={12} /> Start a timer</button>
  </Card>
{:else}
  <div class="add-row"><button class="btn ghost sm" onclick={() => (adding = true)}><Timer size={12} /> Start a timer</button></div>
{/if}

<style>
  ul {
    display: grid;
    gap: var(--space-2);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .row {
    display: flex;
    align-items: center;
    gap: var(--space-1-5);
    margin-bottom: var(--space-1);
  }

  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .time {
    font-weight: var(--weight-strong);
  }

  .done .time {
    color: var(--good);
    font-weight: var(--weight-body);
  }

  .done .name {
    color: var(--muted);
  }

  .track {
    height: 3px;
    border-radius: var(--radius-sm);
    background: var(--hover);
    overflow: hidden;
  }

  .fill {
    height: 100%;
    background: var(--muted);
    transition: width 1s linear;
  }

  .explain {
    margin: 0 0 var(--space-2);
    color: var(--muted);
    font-size: var(--text-sm);
  }

  .add-row {
    padding: var(--space-2) var(--space-3) var(--space-3);
    border-top: 1px solid var(--border);
  }

  .form {
    display: grid;
    gap: var(--space-1-5);
    margin-top: var(--space-2);
  }

  input {
    padding: var(--space-1) var(--space-2);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--bg);
    color: var(--text);
    font: inherit;
    font-size: var(--text-md);
  }

  input:focus {
    outline: none;
    border-color: var(--accent);
  }

  .presets {
    display: flex;
    gap: var(--space-1);
  }

  .presets button {
    padding: var(--space-0-5) var(--space-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--muted);
    font-size: var(--text-md);
    cursor: pointer;
  }

  .presets button[aria-pressed="true"] {
    border-color: var(--accent);
    color: var(--text);
  }

  .minutes {
    width: 56px;
    margin-left: auto;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: var(--space-3);
  }

  .link {
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--muted);
    font-size: var(--text-md);
    cursor: pointer;
  }

  .go {
    padding: var(--space-1) var(--space-4);
    border: 0;
    border-radius: var(--radius-sm);
    background: var(--accent);
    color: var(--accent-ink);
    font-size: var(--text-md);
    font-weight: var(--weight-strong);
    cursor: pointer;
  }

  .icon:focus-visible,
  .presets button:focus-visible,
  .go:focus-visible,
  .link:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }
</style>
