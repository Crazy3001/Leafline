<script lang="ts">
  // A level goal for the character: how many levels are left and about how much play that is,
  // at the rate of this session or, between sessions, of recent play at this level. The EXP each
  // level takes is learned from the player's own HUD as they level; until enough is known, each
  // level is taken to need as much of an hour as this one.
  import { tip } from "$lib/tip.svelte";
  import { Pencil, X } from "@lucide/svelte";
  import { onMount } from "svelte";

  import { backend } from "$lib/api";
  import { formatDuration } from "$lib/format";
  import { history, watchHistory } from "$lib/history.svelte";
  import { dailyPace, goalEstimate, rateNear } from "$lib/training";
  import type { LevelExp } from "$lib/types";
  import Button from "$lib/ui/Button.svelte";
  import Meter from "$lib/ui/Meter.svelte";

  let {
    level,
    percent = null,
    character = null,
    rate = null,
  }: {
    level: number | null;
    percent?: number | null;
    /** Goals are kept per character, by the name read from the HUD. */
    character?: string | null;
    /** This session's rates, when one is running. */
    rate?: { expPerHour: number | null; percentPerHour: number | null } | null;
  } = $props();

  watchHistory();

  let table = $state<LevelExp[]>([]);
  let editing = $state(false);
  let draft = $state("");

  onMount(() => {
    const load = () =>
      backend
        .levelExp()
        .then((t) => (table = t))
        .catch(() => {});
    load();
    // Saves teach the table the level just played.
    const stop = backend.onHistory(load);
    return () => {
      stop.then((unlisten) => unlisten());
    };
  });

  const key = $derived(`leafline.goal.${(character ?? "").toLowerCase() || "_"}`);
  let stored = $state<{ target: number; from: number } | null>(null);
  $effect(() => {
    stored = read(key);
  });

  function read(k: string): { target: number; from: number } | null {
    try {
      const raw = JSON.parse(localStorage.getItem(k) ?? "null");
      return raw && Number.isFinite(raw.target) ? { target: Number(raw.target), from: Number(raw.from) || 1 } : null;
    } catch {
      return null;
    }
  }

  function save(next: { target: number; from: number } | null) {
    stored = next;
    try {
      if (next) localStorage.setItem(key, JSON.stringify(next));
      else localStorage.removeItem(key);
    } catch {
      // Kept for this window only.
    }
  }

  function startEditing() {
    draft = String(stored?.target ?? (level != null ? level + 5 : ""));
    editing = true;
  }

  function confirm() {
    const target = Math.round(Number(draft));
    if (!Number.isFinite(target) || target < 2 || target > 200) return;
    save({ target, from: Math.min(level ?? target - 1, target - 1) });
    editing = false;
  }

  const sessions = $derived(history.value?.sessions ?? []);
  const used = $derived.by(() => {
    if (rate && (rate.expPerHour || rate.percentPerHour)) return { ...rate, live: true };
    const near = level != null ? rateNear(sessions, level) : null;
    return near ? { ...near, live: false } : null;
  });
  const estimate = $derived(
    stored && level != null
      ? goalEstimate({
          level,
          percent,
          target: stored.target,
          table,
          expPerHour: used?.expPerHour ?? null,
          percentPerHour: used?.percentPerHour ?? null,
        })
      : null,
  );
  const pace = $derived(dailyPace(sessions));
  const progress = $derived.by(() => {
    if (!stored || level == null) return 0;
    const span = stored.target - stored.from;
    return span > 0 ? Math.min(1, Math.max(0, (level + (percent ?? 0) / 100 - stored.from) / span)) : 1;
  });
  const days = $derived(estimate?.seconds && pace ? Math.max(1, Math.ceil(estimate.seconds / pace)) : null);
  const levelsText = $derived.by(() => {
    if (!estimate) return "";
    const n = estimate.levels;
    const shown = n >= 10 ? Math.round(n) : Math.round(n * 10) / 10;
    return `${shown} level${shown === 1 ? "" : "s"} to go`;
  });
  const note = $derived.by(() => {
    if (!estimate || estimate.seconds == null) return null;
    const source = used?.live ? "this session's rate" : "your recent rate at this level";
    if (estimate.method === "percent")
      return `At ${source}, taking each level to need as long as this one. Leafline learns what each level takes from your HUD as you play.`;
    if (estimate.extrapolated)
      return `At ${source}. Levels you haven't reached yet are carried on from the ones you have.`;
    return `At ${source}, with the EXP each level took as your HUD showed it.`;
  });
</script>

<div class="goal">
  {#if editing}
    <div class="row">
      <label class="set">
        <span>Goal level</span>
        <input
          type="number"
          min="2"
          max="200"
          bind:value={draft}
          onkeydown={(e) => {
            if (e.key === "Enter") confirm();
            if (e.key === "Escape") editing = false;
          }}
        />
      </label>
      <span class="grow"></span>
      <Button variant="ghost" size="sm" onclick={() => (editing = false)}>Cancel</Button>
      <Button size="sm" onclick={confirm}>Set</Button>
    </div>
  {:else if !stored}
    <div class="row">
      <span class="muted">No level goal yet.</span>
      <span class="grow"></span>
      <Button variant="ghost" size="sm" onclick={startEditing}>Set a goal</Button>
    </div>
  {:else}
    <div class="row">
      <span class="target">Goal <span class="num">Lv {stored.target}</span></span>
      <span class="grow"></span>
      <button class="icon" aria-label="Change the goal" use:tip={"Change the goal"} onclick={startEditing}><Pencil size={12} /></button>
      <button class="icon" aria-label="Clear the goal" use:tip={"Clear the goal"} onclick={() => save(null)}><X size={12} /></button>
    </div>
    {#if level == null}
      <p class="muted">Your level shows once the HUD is read.</p>
    {:else if estimate && estimate.levels <= 0}
      <Meter value={1} height={6} color="var(--good)" label="Goal progress" />
      <p class="reached">Reached. Set the next one when you're ready.</p>
    {:else if estimate}
      <Meter value={progress} height={6} label="Progress from Lv {stored.from} to Lv {stored.target}" />
      <p class="line tabular">
        {levelsText}{#if estimate.seconds != null}{" · "}about <strong>{formatDuration(estimate.seconds)}</strong> of play{/if}{#if days}{" · "}{days === 1
            ? "a day"
            : `${days} days`} at your {formatDuration(pace)} a day{/if}
      </p>
      {#if note}<p class="note">{note}</p>{:else}<p class="note">Play ten minutes at this level for a time estimate.</p>{/if}
    {/if}
  {/if}
</div>

<style>
  .goal {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .grow {
    flex: 1;
  }

  .target {
    font-size: var(--text-sm);
    color: var(--muted);
  }

  .target .num {
    font-weight: 600;
    color: var(--text);
  }

  .muted {
    font-size: var(--text-sm);
    color: var(--muted);
  }

  p {
    margin: 0;
  }

  .line {
    font-size: var(--text-sm);
  }

  .line strong {
    font-weight: 600;
  }

  .note {
    font-size: var(--text-xs);
    color: var(--faint);
  }

  .reached {
    font-size: var(--text-sm);
    color: var(--good);
  }

  .set {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: var(--text-sm);
    color: var(--muted);
  }

  .set input {
    width: 52px;
    height: 22px;
    padding: 0 4px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--text);
    font: inherit;
    text-align: right;
  }

  .icon {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    padding: 0;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--muted);
    cursor: pointer;
  }

  .icon:hover {
    background: rgba(255, 255, 255, 0.07);
    color: var(--text);
  }
</style>
