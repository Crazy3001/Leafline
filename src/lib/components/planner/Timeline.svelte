<script lang="ts">
  // The plan level by level, beside every tab. Short folds runs of levels that spend points the
  // same way into one line; job advancements, gear and the character's own level keep their own.
  import { Check, ChevronDown, ChevronRight, TriangleAlert } from "@lucide/svelte";

  import IconRef from "$lib/components/IconRef.svelte";
  import { tip } from "$lib/tip.svelte";
  import Pic from "$lib/ui/Pic.svelte";

  import { timeline, type TimelineRow } from "$lib/planner";
  import type { Expansion, SkillLevel } from "$lib/types";
  import Segmented from "$lib/ui/Segmented.svelte";

  let {
    expansion,
    goals,
    level,
  }: {
    expansion: Expansion | null;
    goals: SkillLevel[];
    /** The character's level, read from the game. */
    level: number | null;
  } = $props();

  type Mode = "short" | "every";
  const MODE_KEY = "leafline.planner.timeline";
  let mode = $state<Mode>(storedMode());
  /** Folded rows opened to their levels, by first level. */
  let opened = $state<number[]>([]);

  const steps = $derived(expansion?.steps ?? []);
  const rows = $derived(timeline(steps, goals, level, mode === "short"));
  const warnings = $derived(expansion?.problems.filter((p) => p.severity === "warning") ?? []);
  const notes = $derived(expansion?.problems.filter((p) => p.severity === "note") ?? []);

  const here = (row: TimelineRow) => level != null && row.from <= level && level <= row.to;
  const span = (row: TimelineRow) => (row.from === row.to ? `Lv ${row.from}` : `Lv ${row.from}–${row.to}`);

  function storedMode(): Mode {
    try {
      return localStorage.getItem(MODE_KEY) === "every" ? "every" : "short";
    } catch {
      return "short";
    }
  }

  function pick(next: Mode) {
    mode = next;
    opened = [];
    try {
      localStorage.setItem(MODE_KEY, next);
    } catch {
      // A convenience only.
    }
  }

  function toggle(row: TimelineRow) {
    opened = opened.includes(row.from) ? opened.filter((f) => f !== row.from) : [...opened, row.from];
  }
</script>

{#snippet lines(row: TimelineRow)}
  {#each row.lines as line, i (i)}
    {#if line.items}
      <span class="line gear" aria-label={line.text}>
        {#each line.items as name (name)}<IconRef kind="item" {name} size={22} />{/each}
      </span>
    {:else}
      <span class="line {line.kind}" class:with-icon={line.skillId != null}
        >{#if line.skillId != null}<Pic kind="skill" id={line.skillId} size={16} />{/if}<span class="text">{line.text}</span
        >{#if line.kind === "met"}<span class="tick" use:tip={"Goal met"} aria-label="Goal met"><Check size={12} /></span>{/if}</span
      >
    {/if}
  {/each}
{/snippet}

<aside aria-label="The plan level by level">
  {#if warnings.length || notes.length}
    <div class="problems">
      {#each warnings as problem (problem.message)}
        <p class="warning"><TriangleAlert size={12} /><span>{problem.message}</span></p>
      {/each}
      {#each notes as problem (problem.message)}<p class="note">{problem.message}</p>{/each}
    </div>
  {/if}

  <div class="head">
    <h2>Level by level</h2>
    <Segmented
      label="Timeline"
      options={[
        { id: "short" as Mode, label: "Short" },
        { id: "every" as Mode, label: "Every level" },
      ]}
      value={mode}
      size="sm"
      onchange={pick}
    />
  </div>

  {#if rows.length}
    <ol class="rows">
      {#each rows as row (row.from)}
        {@const folded = row.from !== row.to}
        {@const open = opened.includes(row.from)}
        <li class:here={here(row)} class:advance={row.lines.some((l) => l.kind === "advance")}>
          {#if folded}
            <button class="row" aria-expanded={open} onclick={() => toggle(row)}>
              <span class="at tabular">{span(row)}</span>
              <span class="what">
                {#if here(row)}<span class="line you">You are here, Lv {level}</span>{/if}
                {@render lines(row)}
              </span>
              <span class="chev" aria-hidden="true">{#if open}<ChevronDown size={12} />{:else}<ChevronRight size={12} />{/if}</span>
            </button>
            {#if open}
              <ol class="inner">
                {#each timeline(row.steps, goals, level, false) as one (one.from)}
                  <li class:here={here(one)}>
                    <span class="at tabular">Lv {one.from}</span>
                    <span class="what">{@render lines(one)}</span>
                  </li>
                {/each}
              </ol>
            {/if}
          {:else}
            <div class="row">
              <span class="at tabular">{span(row)}</span>
              <span class="what">
                {#if here(row)}<span class="line you">You are here</span>{/if}
                {@render lines(row)}
              </span>
            </div>
          {/if}
        </li>
      {/each}
    </ol>
    {#if mode === "short"}
      <p class="key">Click a run of levels to open it. Gold lines are job advancements; a tick marks a goal met.</p>
    {/if}
  {:else}
    <p class="key">Your plan shows here as you add goals.</p>
  {/if}
</aside>

<style>
  aside {
    min-height: 0;
    padding: 14px 14px 24px;
    border-left: 1px solid var(--border);
    background: var(--surface);
    overflow-y: auto;
  }

  .problems {
    display: grid;
    gap: 6px;
    margin-bottom: 12px;
    padding-bottom: 12px;
    border-bottom: 1px solid var(--border);
    font-size: var(--text-sm);
  }

  .problems p {
    margin: 0;
  }

  .warning {
    display: flex;
    gap: 6px;
    color: var(--warn);
  }

  .warning :global(svg) {
    flex: none;
    margin-top: 3px;
  }

  .note {
    color: var(--muted);
  }

  .head {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 8px;
  }

  h2 {
    flex: 1;
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--text-lg);
    font-weight: 600;
  }

  .rows,
  .inner {
    margin: 0;
    padding: 0;
    list-style: none;
    font-size: var(--text-sm);
  }

  .rows > li {
    border-top: 1px solid var(--border);
  }

  .rows > li.advance {
    background: var(--surface-2);
  }

  .rows > li.here {
    background: var(--accent-soft);
    box-shadow: inset 2px 0 0 var(--accent);
  }

  .row {
    display: grid;
    grid-template-columns: 62px minmax(0, 1fr) 14px;
    gap: 8px;
    width: 100%;
    padding: 6px 6px;
    border: 0;
    background: transparent;
    color: var(--text);
    font: inherit;
    text-align: left;
  }

  button.row {
    cursor: pointer;
  }

  button.row:hover {
    background: rgba(255, 255, 255, 0.04);
  }

  .at {
    color: var(--faint);
    font-family: var(--font-display);
  }

  .here .at,
  .advance .at {
    color: var(--accent-strong);
  }

  .what {
    display: grid;
    min-width: 0;
  }

  .line.sub {
    color: var(--muted);
  }

  .line.advance,
  .line.you {
    color: var(--accent-strong);
    font-weight: 600;
  }

  /* New gear as a row of its pictures; each names itself on hover and opens its card. */
  .line.gear {
    display: flex;
    flex-wrap: wrap;
    gap: 2px;
    margin: 1px 0 2px -2px;
  }

  .line.with-icon {
    display: flex;
    align-items: center;
    gap: 5px;
  }

  .line .text {
    min-width: 0;
  }

  .tick {
    display: inline-grid;
    flex: none;
    margin-left: 4px;
    color: var(--good);
  }

  .chev {
    align-self: center;
    color: var(--faint);
  }

  .inner {
    margin: 0 6px 6px 18px;
    border-left: 1px solid var(--border);
  }

  .inner li {
    display: grid;
    grid-template-columns: 44px minmax(0, 1fr);
    gap: 8px;
    padding: 3px 8px;
  }

  .inner li.here .at {
    color: var(--accent-strong);
  }

  .key {
    margin: 10px 0 0;
    color: var(--faint);
    font-size: var(--text-xs);
  }
</style>
