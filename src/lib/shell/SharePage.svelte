<script lang="ts">
  // Tools → Share card: pick a saved session and make its card, a picture you copy or save and
  // post yourself. Leafline sends nothing. Every session is in the training log too.
  import { ChevronRight, Share2 } from "@lucide/svelte";
  import { onMount } from "svelte";

  import ShareDialog from "$lib/components/ShareDialog.svelte";
  import { backend } from "$lib/api";
  import { characters } from "$lib/characters.svelte";
  import { dropsBetween, watchDrops } from "$lib/drops.svelte";
  import { formatCompact, formatDuration, splitMapLabel } from "$lib/format";
  import { history, watchHistory } from "$lib/history.svelte";
  import { openTrainingLog } from "$lib/nav.svelte";
  import { longEnough, shortDay } from "$lib/training";
  import type { LoggedDrop, SavedSession, SessionTimeline } from "$lib/types";

  onMount(() => {
    watchHistory();
    watchDrops();
  });

  const SHOWN = 8;
  const sessions = $derived(characters.sessions(history.value?.sessions ?? []).filter(longEnough).slice(0, SHOWN));

  let open = $state<{ session: SavedSession; timeline: SessionTimeline | null; drops: LoggedDrop[] } | null>(null);

  async function make(session: SavedSession) {
    const start = session.startedAt ?? session.savedAt - session.elapsedSeconds;
    const drops = dropsBetween(start, start + session.elapsedSeconds + 60, session.character);
    open = { session, timeline: null, drops };
    try {
      const timeline = await backend.sessionTimeline(session.id);
      if (open?.session.id === session.id) open = { ...open, timeline };
    } catch {
      // The card shows without its graph.
    }
  }

  const where = (s: SavedSession) => s.label || (s.maps.length ? splitMapLabel(s.maps[0].map).name : "No map read");
  const rate = (s: SavedSession) =>
    s.expPerHour == null && s.percentPerHour != null ? `${s.percentPerHour.toFixed(1)}% / hr` : `${formatCompact(s.expPerHour)} EXP / hr`;
</script>

{#if open}
  <ShareDialog session={open.session} timeline={open.timeline} drops={open.drops} onclose={() => (open = null)} />
{/if}

<section class="card page">
  <p class="lead">Pick a session for its card: a picture you copy or save and post yourself.</p>
  {#if !history.value}
    <p class="quiet">Loading…</p>
  {:else if !sessions.length}
    <p class="quiet">No sessions saved yet. One saves itself when it ends.</p>
  {:else}
    <ul>
      {#each sessions as s (s.id)}
        <li>
          <button onclick={() => make(s)}>
            <span class="glyph"><Share2 size={13} /></span>
            <span class="what">
              <span class="name">{where(s)}</span>
              <span class="sub">{shortDay(s.startedAt ?? s.savedAt)} · {formatDuration(s.activeSeconds)} · {rate(s)}</span>
            </span>
            <ChevronRight size={14} />
          </button>
        </li>
      {/each}
    </ul>
  {/if}
  <button class="link" onclick={openTrainingLog}>Every session, in the training log</button>
</section>

<style>
  .page {
    display: grid;
    gap: var(--space-2);
    margin: 0 var(--space-3) var(--space-2);
    padding: var(--space-3) var(--space-4) var(--space-3);
  }

  .lead,
  .quiet {
    margin: 0;
    color: var(--muted);
    font-size: var(--text-md);
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li + li {
    border-top: 1px solid var(--border);
  }

  li button {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
    padding: var(--space-2) var(--space-1);
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--faint);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  li button:hover {
    background: var(--hover);
    color: var(--text);
  }

  .glyph {
    display: grid;
    flex: none;
    place-items: center;
    width: 28px;
    height: 28px;
    border-radius: var(--radius-sm);
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .what {
    display: grid;
    flex: 1;
    min-width: 0;
  }

  .name {
    overflow: hidden;
    color: var(--text);
    font-weight: var(--weight-strong);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .sub {
    color: var(--faint);
    font-size: var(--text-xs);
  }

  .link {
    justify-self: start;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--accent-text);
    font: inherit;
    font-size: var(--text-md);
    font-weight: var(--weight-strong);
    cursor: pointer;
  }

  .link:hover {
    color: var(--accent-text-strong);
    text-decoration: underline;
    text-underline-offset: 2px;
  }
</style>
