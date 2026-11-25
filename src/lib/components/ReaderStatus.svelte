<script lang="ts">
  // What Leafline sees: each reader with when it last read, or why it isn't reading and what fills
  // it; and a check that watches for a few seconds with the game in front and ticks off what it
  // reads. The check only looks: the player never teaches Leafline anything. The readings left out
  // as doubtful are listed below, with why. All of it stays on this PC.
  import { Check, CircleAlert, Clock, Minus, Play } from "@lucide/svelte";

  import { backend } from "$lib/api";
  import { can } from "$lib/platform";
  import { live } from "$lib/readings.svelte";
  import { READERS, pictureNotes, pictureProblem, readSince, sight, verdict, watchSight, type Tone } from "$lib/sight.svelte";
  import { formatAgo } from "$lib/format";
  import Button from "$lib/ui/Button.svelte";

  let {
    /** Setup's reading check: the check is the point, so it leads and its result stays. */
    setup = false,
  }: { setup?: boolean } = $props();

  watchSight();

  /** How long a check watches the game. */
  const CHECK_SECONDS = 12;

  /** When the check started, Unix seconds; null with none run. */
  let checkFrom = $state<number | null>(null);
  let starting = $state(false);
  let startError = $state<string | null>(null);

  const running = $derived(live.liveStatus.state === "reading" || live.liveStatus.state === "waiting");
  const checking = $derived(checkFrom != null && sight.now - checkFrom < CHECK_SECONDS);
  const left = $derived(checkFrom == null ? 0 : Math.max(0, Math.ceil(CHECK_SECONDS - (sight.now - checkFrom))));
  const problem = $derived(pictureProblem());
  const notes = $derived(pictureNotes());

  const required = READERS.filter((r) => !r.optional);
  const optional = READERS.filter((r) => r.optional);
  const since = $derived(checkFrom ?? sight.value?.since ?? 0);
  const passed = $derived(required.filter((r) => readSince(r.reader, since)).length);

  async function check() {
    startError = null;
    if (!running) {
      if (!can.liveReading) {
        startError = "Share the game with this page first (Start reading, above).";
        return;
      }
      starting = true;
      try {
        await backend.startLive();
      } catch (err) {
        startError = String(err);
      } finally {
        starting = false;
      }
    }
    checkFrom = Date.now() / 1000;
  }

  const ICON: Record<Tone, typeof Check> = { good: Check, stale: Clock, bad: CircleAlert, quiet: Minus };
  const rejected = $derived(sight.value?.rejected ?? []);
  const LABEL = Object.fromEntries(READERS.map((r) => [r.reader, r.label]));
</script>

<div class="sees">
  <div class="head">
    <div class="state">
      {#if checkFrom != null}
        {#if checking}
          <span class="lead">Checking… {left} s</span>
          <span class="help">Keep the game in front with your character in a map.</span>
        {:else}
          <span class="lead">{passed} of {required.length} read</span>
          <span class="help">
            {passed === required.length
              ? "Leafline reads the status bar and the map. The rest read when you open them in game."
              : "What didn't read says why below. Fix that, then check again."}
          </span>
        {/if}
      {:else if setup}
        <span class="lead">Check that Leafline reads your game</span>
        <span class="help">
          Put the game in front with your character in a map, then check. Leafline watches for {CHECK_SECONDS} seconds and ticks off
          what it reads. You don't need to do anything else.
        </span>
      {:else}
        <span class="help">When each part of the game was last read, or why it isn't.</span>
      {/if}
    </div>
    <Button size="sm" variant={setup && checkFrom == null ? "primary" : "ghost"} onclick={check} disabled={checking || starting}>
      <Play size={12} />{checkFrom == null ? "Check reading" : "Check again"}
    </Button>
  </div>

  {#if startError}<p class="warn">{startError}</p>{/if}
  {#if !setup || checkFrom != null}
  {#if problem}<p class="warn">{problem}</p>{/if}
  {#each notes as note (note)}<p class="note">{note}</p>{/each}

  <ul>
    {#each required as info (info.reader)}
      {@const v = verdict(info, since)}
      {@const Icon = ICON[checking && v.tone !== "good" ? "quiet" : v.tone]}
      <li class={checking && v.tone !== "good" ? "quiet" : v.tone}>
        <span class="icon"><Icon size={13} /></span>
        <span class="name">{info.label}</span>
        <span class="why">{checking && v.tone !== "good" ? "Looking…" : v.text}</span>
      </li>
    {/each}
  </ul>
  <p class="sub">When you open them in game</p>
  <ul>
    {#each optional as info (info.reader)}
      {@const v = verdict(info, since)}
      {@const Icon = ICON[v.tone === "bad" && checking ? "quiet" : v.tone]}
      <li class={v.tone}>
        <span class="icon"><Icon size={13} /></span>
        <span class="name">{info.label}</span>
        <span class="why">{v.text}</span>
      </li>
    {/each}
  </ul>

  {#if rejected.length}
    <details>
      <summary>Left out as doubtful this run ({rejected.length})</summary>
      <p class="help">
        Readings that didn't fit the ones around them, so they weren't counted. A few are normal; many in a row mean the picture is
        hard to read.
      </p>
      <ul class="log">
        {#each rejected as r, i (i)}
          <li>
            <span class="when">{formatAgo(sight.now - r.at)}</span>
            <span class="name">{LABEL[r.reader] ?? r.reader}</span>
            <span class="why">{r.reason}</span>
          </li>
        {/each}
      </ul>
    </details>
  {/if}
  {/if}
</div>

<style>
  .sees {
    display: grid;
    gap: var(--space-2);
  }

  .head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--space-3);
  }

  .state {
    display: grid;
    gap: 2px;
  }

  .lead {
    font-weight: 600;
  }

  .help,
  .sub {
    color: var(--muted);
    font-size: var(--text-sm);
  }

  .sub {
    margin: var(--space-2) 0 0;
  }

  .warn,
  .note {
    margin: 0;
    font-size: var(--text-sm);
  }

  .warn {
    color: var(--warn, var(--accent));
  }

  .note {
    color: var(--muted);
  }

  ul {
    display: grid;
    gap: 1px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li {
    display: grid;
    grid-template-columns: 18px minmax(110px, 0.35fr) 1fr;
    align-items: baseline;
    gap: var(--space-2);
    padding: 5px 0;
    border-bottom: var(--line-w, 1px) solid var(--line);
    font-size: var(--text-sm);
  }

  .icon {
    display: inline-flex;
    align-self: center;
  }

  .name {
    font-weight: 500;
  }

  .why {
    color: var(--muted);
  }

  li.good .icon {
    color: var(--good, #5fbf7f);
  }

  li.stale .icon {
    color: var(--muted);
  }

  li.bad .icon,
  li.bad .why {
    color: var(--warn, var(--accent));
  }

  li.quiet .icon {
    color: var(--faint);
  }

  details summary {
    cursor: pointer;
    color: var(--muted);
    font-size: var(--text-sm);
  }

  .log li {
    grid-template-columns: 64px minmax(110px, 0.35fr) 1fr;
  }

  .when {
    color: var(--faint);
    font-variant-numeric: tabular-nums;
  }
</style>
