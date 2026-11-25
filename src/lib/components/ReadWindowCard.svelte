<script lang="ts">
  // The website's way into live reading: share the game window with this page and Leafline reads
  // level and EXP from it, tracking the session the way the program does (platform/live.ts). The
  // browser asks which window; the picture stays on this PC. Says plainly what it costs: the
  // same ban risk as the program. In a browser that can't share a window, says so and why.
  import { Square, MonitorPlay } from "@lucide/svelte";
  import { onMount } from "svelte";

  import { backend } from "$lib/api";
  import { can } from "$lib/platform";
  import { cantRead } from "$lib/platform/capture";
  import { characters } from "$lib/characters.svelte";
  import { live } from "$lib/readings.svelte";
  import Button from "$lib/ui/Button.svelte";

  import Card from "./Card.svelte";

  let { compact = false }: { compact?: boolean } = $props();

  const status = $derived(live.liveStatus);
  const on = $derived(status.state === "reading" || status.state === "waiting");
  let problem = $state<string | null>(null);

  // The website can't read the HUD's name yet, so sessions are saved under the character picked.
  onMount(() => {
    if (__LEAFLINE_WEB__) void import("$lib/platform/live").then((m) => m.setLiveCharacter(() => characters.viewing));
  });

  async function start() {
    problem = null;
    try {
      await backend.startLive();
    } catch (err) {
      problem = err instanceof Error ? err.message : String(err);
    }
  }
</script>

<Card title="Read the game">
  {#if !can.readWindow}
    <p class="lead">{cantRead}</p>
  {:else if on}
    <p class="lead">
      {#if status.state === "reading"}
        Reading level, EXP, mesos, map, quests and loot from <strong>{status.window || "the shared window"}</strong>{status.held ? ", paused" : ""}.
      {:else if status.state === "waiting"}
        {status.reason}
      {/if}
    </p>
    {#if !compact}<p class="hint">Mesos are read while your inventory is open, the map from the minimap, quests from the Quest Helper and loot from the lines at the bottom right. Rest the mouse on an item and its card comes up in Look up. The session saves itself to your training log after 5 minutes.</p>{/if}
    <div class="actions">
      <Button onclick={() => backend.stopLive()}><Square size={12} />Stop reading</Button>
    </div>
  {:else}
    <p class="lead">
      Share the MapleStory window with this page and Leafline reads your level, EXP, mesos, map, quests and loot as you play,
      the way the program does. In Firefox, share the screen the game is on instead: it can't film a game's window. The
      picture stays on this computer.
    </p>
    <div class="actions">
      <Button variant="primary" onclick={start}><MonitorPlay size={14} />Share the game window…</Button>
    </div>
    {#if status.state === "failed"}<p class="hint warn">{status.message}</p>{/if}
    {#if problem}<p class="hint warn">{problem}</p>{/if}
  {/if}
  {#if can.readWindow}
    <p class="risk">
      Reading a shared window carries the same ban risk as the program. Nexon has warned that third-party tools can mean a
      permanent ban and hasn't said whether a read-only page like this counts. Leafline is unofficial; use it at your own
      risk.
    </p>
  {/if}
</Card>

<style>
  .lead {
    margin: 0 0 10px;
    color: var(--muted);
    font-size: var(--text-sm);
  }

  .lead strong {
    color: var(--text);
    font-weight: 600;
  }

  .actions {
    display: flex;
    gap: 8px;
    margin-bottom: 10px;
  }

  .hint {
    margin: 0 0 10px;
    color: var(--faint);
    font-size: var(--text-xs);
  }

  .hint.warn {
    color: var(--warn);
  }

  .risk {
    margin: 0;
    padding-top: 8px;
    border-top: 1px solid var(--border);
    color: var(--faint);
    font-size: var(--text-xs);
  }
</style>
