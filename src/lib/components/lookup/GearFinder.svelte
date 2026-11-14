<script lang="ts">
  // Equipment for one character, slot by slot, the best first: what their job can use at their
  // level, going by their build where they have one. Anything it assumes can be changed here.
  import LookupRow from "./LookupRow.svelte";
  import { tip } from "$lib/tip.svelte";
  import Chips from "./Chips.svelte";
  import Section from "../map/Section.svelte";
  import { backend } from "$lib/api";
  import { buildState, watchBuilds } from "$lib/builds.svelte";
  import { display } from "$lib/settings.svelte";
  import { gearView, gives, itemView, setGender } from "$lib/items.svelte";
  import type { BuildCatalog, GearFound, ItemRecord } from "$lib/types";

  let {
    level = null,
    character = null,
    job = null,
    slots,
    slot = null,
    onslot,
  }: {
    level?: number | null;
    character?: string | null;
    /** The job the HUD shows. */
    job?: string | null;
    slots: string[];
    slot?: string | null;
    onslot: (slot: string | null) => void;
  } = $props();

  /** Levels ahead the player can look with one click, and every level at once. */
  const AHEAD: (number | "all")[] = [0, 5, 10, 20, "all"];
  /** "All levels" asks for what the job can wear at the top level, so nothing is held back by level. */
  const TOP_LEVEL = 200;
  /** Items each slot lists before "Show all". */
  const SHOWN = 3;

  watchBuilds();

  let catalog = $state<BuildCatalog | null>(null);
  let found = $state<GearFound | null>(null);

  const jobs = $derived(catalog?.jobs ?? []);
  const baseLevel = $derived(gearView.level ?? level);
  const allLevels = $derived(gearView.ahead === "all");
  const asked = $derived(baseLevel == null ? null : gearView.ahead === "all" ? TOP_LEVEL : baseLevel + gearView.ahead);
  const key = $derived(JSON.stringify([asked, gearView.job, gearView.gender, character, job, buildState.version, display.buildRules]));
  const changed = $derived(gearView.level != null || gearView.job != null || gearView.ahead !== 0);
  const shownSlots = $derived((found?.gear.slots ?? []).filter((s) => !slot || s.slot === slot));
  const beginner = $derived(found != null && found.level < found.firstJobLevel);
  const offered = $derived(slots.filter((name) => found?.gear.slots.some((s) => s.slot === name)));
  const jobName = $derived(jobs.find((j) => j.id === found?.job)?.name ?? null);

  $effect(() => {
    backend.buildCatalog().then((c) => (catalog = c));
  });

  // Follows the key alone: every live reading brings new objects with the same content.
  $effect(() => {
    const ask = key;
    backend.findGear({ level: asked, job: gearView.job, gender: gearView.gender }, character, job).then((result) => {
      if (ask === key) found = result;
    });
  });

  const number = (value: string) => (value === "" ? null : Math.max(1, Math.floor(Number(value) || 1)));

  function line(item: ItemRecord) {
    return [item.weaponType, item.reqLevel ? `Lv ${item.reqLevel}` : "No level needed", gives(item, 3)].filter(Boolean).join(" · ");
  }

  function reset() {
    gearView.level = null;
    gearView.job = null;
    gearView.ahead = 0;
  }

  const ranked = $derived.by(() => {
    const names = found?.gear.rankedBy ?? [];
    return names.length > 1 ? `${names.slice(0, -1).join(", ")}, then ${names[names.length - 1]}` : names.join("");
  });
</script>

<div class="who">
  <label class="level">
    <span>Lv</span>
    <input
      class="tabular"
      type="number"
      min="1"
      aria-label="Level to find gear for"
      value={baseLevel ?? found?.level ?? ""}
      oninput={(e) => (gearView.level = number(e.currentTarget.value))}
    />
  </label>
  <select aria-label="Job" value={gearView.job ?? found?.job ?? 0} onchange={(e) => (gearView.job = Number(e.currentTarget.value))}>
    <option value={0}>Beginner</option>
    {#each jobs as choice (choice.id)}
      <option value={choice.id}>{choice.parent ? `  ${choice.name}` : choice.name}</option>
    {/each}
  </select>
  <select aria-label="Character's gender" value={gearView.gender ?? ""} onchange={(e) => setGender(e.currentTarget.value || null)}>
    <option value="">M + F</option>
    <option value="male">Male</option>
    <option value="female">Female</option>
  </select>
</div>

<div class="ahead">
  {#if baseLevel != null}
    <div role="radiogroup" aria-label="Levels to look ahead">
      {#each AHEAD as levels (levels)}
        <button role="radio" aria-checked={gearView.ahead === levels} onclick={() => (gearView.ahead = levels)}
          >{levels === "all" ? "All levels" : levels ? `+${levels}` : "Now"}</button
        >
      {/each}
    </div>
  {/if}
  {#if changed}<button class="plain" onclick={reset}>Reset</button>{/if}
  <button class="plain everything" use:tip={"Every piece of equipment, for any job, in the item search"} onclick={() => (gearView.forMe = false)}>Any job</button>
</div>

<Chips label="Slot" options={offered} value={slot} onselect={onslot} />

{#if found}
  {#if beginner && found.job && jobName && !allLevels}
    <p class="note">Until Lv {found.firstJobLevel} a character wears only gear for anyone. Look ahead to see {jobName} gear.</p>
  {/if}

  {#each shownSlots as group (group.slot)}
    {#if slot}
      <p class="count tabular">{group.total.toLocaleString()} for this slot{group.total > group.picks.length ? `, the best ${group.picks.length} shown` : ""}</p>
      <ul class="rows">
        {#each group.picks as pick (pick.item.id)}
          <li>
            <LookupRow name={pick.item.name} picture={{ kind: "item", id: pick.item.id }} line={line(pick.item)} small={pick.item.gender?.[0].toUpperCase()} goal={pick.goal} onclick={() => (itemView.id = pick.item.id)} />
          </li>
        {/each}
      </ul>
    {:else}
      <Section title={group.slot} count={group.total}>
        <ul class="rows">
          {#each group.picks.slice(0, SHOWN) as pick (pick.item.id)}
            <li>
              <LookupRow name={pick.item.name} picture={{ kind: "item", id: pick.item.id }} line={line(pick.item)} small={pick.item.gender?.[0].toUpperCase()} goal={pick.goal} onclick={() => (itemView.id = pick.item.id)} />
            </li>
          {/each}
        </ul>
        {#if group.total > SHOWN}
          <button class="more" onclick={() => onslot(group.slot)}>Show all {group.total}</button>
        {/if}
      </Section>
    {/if}
  {:else}
    <p class="note">Nothing here{allLevels ? "" : ` for Lv ${found.level}`}{slot ? ` in this slot` : ""}.</p>
  {/each}

  <p class="how">
    {#if allLevels}Every level this job can reach, so the strongest come first; each row says the level it needs.{/if}
    Best first by {ranked}. {#if found.build}Uses your stats from {found.build}.{:else}Goes by level and job only: stat requirements aren't checked without a build.{/if}
    Staff items are left out. Nobody has checked these in game, and it doesn't know prices.
  </p>
{/if}

<style>
  .who {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) 72px;
    gap: 4px;
    margin-bottom: 4px;
  }

  .level {
    display: flex;
    align-items: center;
    gap: 4px;
    color: var(--muted);
    font-size: var(--text-sm);
  }

  .level input {
    width: 44px;
  }

  input,
  select {
    min-width: 0;
    height: 24px;
    padding: 0 4px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
  }

  .ahead {
    display: flex;
    align-items: center;
    gap: 3px;
    margin-bottom: 5px;
    color: var(--faint);
    font-size: var(--text-xs);
  }

  .ahead > div {
    display: flex;
    gap: 3px;
  }

  .ahead button {
    min-width: 30px;
    height: 20px;
    padding: 0 5px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--muted);
    font-size: var(--text-xs);
    cursor: pointer;
  }

  .ahead button:hover,
  .ahead button:focus-visible {
    color: var(--text);
    outline: none;
  }

  .ahead [aria-checked="true"] {
    border-color: color-mix(in srgb, var(--accent) 45%, transparent);
    background: var(--accent-soft);
    color: var(--accent-strong);
  }

  .ahead .plain {
    padding: 0 2px;
    border: 0;
  }

  .ahead .everything {
    margin-left: auto;
  }

  .rows {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .rows li + li {
    border-top: 1px solid var(--border);
  }

  .more {
    margin-top: 2px;
    padding: 0 4px;
    border: 0;
    background: transparent;
    color: var(--muted);
    font-size: var(--text-xs);
    cursor: pointer;
  }

  .more:hover,
  .more:focus-visible {
    color: var(--text);
    outline: none;
  }

  .count {
    margin: 0 0 4px;
    color: var(--faint);
    font-size: var(--text-xs);
  }

  .note {
    margin: 4px 0 6px;
    color: var(--muted);
    font-size: var(--text-sm);
  }

  .how {
    margin: 10px 0 0;
    color: var(--faint);
    font-size: var(--text-xs);
  }
</style>
