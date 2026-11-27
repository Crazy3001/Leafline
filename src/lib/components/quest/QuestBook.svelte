<script lang="ts">
  // The Quests page's list: every quest in the data, by region, with where each stands for the
  // character. What is ready to hand in comes first, with who takes it. Quests the data chains
  // together fold into one questline row that opens, showing how far along it is and the quest
  // you're on; a switch shows them flat. Picking a quest opens it beside the list.
  // What the game's Quest window showed is only true as of the last look, which the card says.
  import { untrack, type Snippet } from "svelte";
  import { Check, ChevronDown, ChevronRight, CircleCheck, CircleDot, Info, Lock, Undo2 } from "@lucide/svelte";

  import Card from "../Card.svelte";
  import GoButton from "../GoButton.svelte";
  import Ref from "../Ref.svelte";
  import Segmented from "$lib/ui/Segmented.svelte";
  import Switch from "$lib/ui/Switch.svelte";
  import { backend } from "$lib/api";
  import { can } from "$lib/platform";
  import { npcPlace } from "$lib/destination.svelte";
  import { formatAgo } from "$lib/format";
  import { hasLog, look, questLog, watchQuestLog } from "$lib/questlog.svelte";
  import { tip } from "$lib/tip.svelte";
  import { currentStep, helperCounts, isReady, lineSteps, type Count } from "./questbook";
  import type { QuestReading, QuestSheet, QuestTab, Stand, WholeLog, WholeQuest } from "$lib/types";

  let {
    whole,
    helper = null,
    selected = null,
    onpick,
    tools,
    below,
  }: {
    whole: WholeLog | null;
    /** What the Quest Helper shows now: its counts are the freshest. */
    helper?: QuestReading | null;
    /** The quest open beside the list. */
    selected?: number | null;
    onpick: (id: number) => void;
    /** More controls at the end of the chips' row, and what they open under it: the desk's filters. */
    tools?: Snippet;
    below?: Snippet;
  } = $props();

  watchQuestLog();

  type Filter = "all" | "inProgress" | "canTake" | "notYet" | "done";
  const FILTER_KEY = "leafline.quests.whole";
  const GROUP_KEY = "leafline.quests.lines";
  const GROUP: Record<Stand, Filter> = {
    ready: "inProgress",
    inProgress: "inProgress",
    canTake: "canTake",
    notYet: "notYet",
    done: "done",
    gone: "done",
  };
  const LABEL: Record<Filter, string> = {
    all: "All",
    inProgress: "In progress",
    canTake: "Can take",
    notYet: "Not yet",
    done: "Done",
  };
  const STAND_LABEL: Record<Stand, string> = {
    ready: "Ready to hand in",
    inProgress: "In progress",
    canTake: "Can take",
    notYet: "Not yet",
    done: "Done",
    gone: "Handed in or dropped",
  };
  const TABS: { tab: QuestTab; label: string }[] = [
    { tab: "available", label: "Available" },
    { tab: "inProgress", label: "In Progress" },
    { tab: "completed", label: "Completed" },
  ];

  function stored<T extends string>(key: string, ok: (v: string) => boolean, fallback: T): T {
    try {
      const saved = localStorage.getItem(key);
      if (saved != null && ok(saved)) return saved as T;
    } catch {
      // A convenience only.
    }
    return fallback;
  }
  function keep(key: string, value: string) {
    try {
      localStorage.setItem(key, value);
    } catch {
      // A convenience only.
    }
  }

  let filter = $state<Filter>(stored(FILTER_KEY, (v) => v in LABEL, "all"));
  function setFilter(next: Filter) {
    filter = next;
    keep(FILTER_KEY, next);
  }
  /** Questlines fold into one row each, unless the player shows every quest flat. */
  let grouped = $state(stored(GROUP_KEY, (v) => v === "on" || v === "off", "on") === "on");
  function setGrouped(on: boolean) {
    grouped = on;
    keep(GROUP_KEY, on ? "on" : "off");
  }

  const all = $derived((whole?.regions ?? []).flatMap((r) => r.quests));
  /** The Quest Helper's counts for a quest, by its title. */
  const counts = (q: WholeQuest): Count[] => helperCounts(helper, q) ?? q.counters;
  /** Ready by the Quest Window, or by the Quest Helper's counts now. */
  const ready = (q: WholeQuest) => isReady(q, helper);
  const standOf = (q: WholeQuest): Stand => (q.stand === "inProgress" && ready(q) ? "ready" : q.stand);

  const tally = $derived.by(() => {
    const n: Record<Filter, number> = { all: 0, inProgress: 0, canTake: 0, notYet: 0, done: 0 };
    for (const q of all) {
      n.all += 1;
      n[GROUP[standOf(q)]] += 1;
    }
    return n;
  });
  const options = $derived((Object.keys(LABEL) as Filter[]).map((id) => ({ id, label: `${LABEL[id]} ${tally[id].toLocaleString()}` })));
  const passes = (q: WholeQuest) => filter === "all" || GROUP[standOf(q)] === filter;

  /** In progress first, then what can be taken, what can't yet, and what's done. */
  const ORDER: Record<Stand, number> = { ready: 0, inProgress: 1, canTake: 2, notYet: 3, gone: 4, done: 5 };

  type Line = { head: number; name: string; steps: WholeQuest[]; shown: WholeQuest[]; current: WholeQuest | null; done: number; of: number };
  type Item = { kind: "quest"; q: WholeQuest } | { kind: "line"; line: Line };

  /** Each line once, in the region of its first step, with every step from any region. */
  const lines = $derived.by(() => {
    const out = new Map<number, { line: Line; home: string }>();
    if (!grouped) return out;
    for (const r of whole?.regions ?? [])
      for (const q of r.quests) {
        if (!q.line) continue;
        let entry = out.get(q.line.head);
        if (!entry) {
          entry = { line: { head: q.line.head, name: q.line.name, steps: [], shown: [], current: null, done: 0, of: q.line.of }, home: r.region };
          out.set(q.line.head, entry);
        }
        if (q.line.step === 1) entry.home = r.region;
        entry.line.steps.push(q);
      }
    for (const { line } of out.values()) {
      line.steps = lineSteps(line.steps);
      line.shown = line.steps.filter(passes);
      line.current = currentStep(line.steps, helper);
      line.done = line.steps.filter((q) => q.line!.step > 0 && (q.stand === "done" || q.stand === "gone")).length;
    }
    return out;
  });

  const itemOrder = (item: Item) =>
    item.kind === "quest" ? ORDER[standOf(item.q)] : item.line.current ? ORDER[standOf(item.line.current)] : ORDER.done;
  const regions = $derived(
    (whole?.regions ?? [])
      .map((r) => {
        const items: Item[] = [];
        for (const q of r.quests) if (!(q.line && lines.has(q.line.head)) && passes(q)) items.push({ kind: "quest", q });
        for (const { line, home } of lines.values()) if (home === r.region && line.shown.length) items.push({ kind: "line", line });
        items.sort((a, b) => itemOrder(a) - itemOrder(b));
        const active = r.quests.filter((q) => GROUP[standOf(q)] === "inProgress").length;
        return { ...r, items, active, tally: tallyRegion(r.region, r.quests) };
      })
      .filter((r) => r.items.length)
      .sort((a, b) => b.active - a.active || a.region.localeCompare(b.region)),
  );

  // How far through a region you are, the way the game's Completed tab counts with questlines on:
  // a line once, done when every step is; flat, every quest. Done is what the Quest window showed;
  // what Leafline only takes to be done (a later quest is) is counted apart. The filter doesn't
  // change it.
  const finished = (q: WholeQuest) => q.stand === "done" || q.stand === "gone";
  function tallyRegion(region: string, quests: WholeQuest[]) {
    const units: WholeQuest[][] = [];
    for (const q of quests) if (!(q.line && lines.has(q.line.head))) units.push([q]);
    for (const { line, home } of lines.values()) if (home === region) units.push(line.steps.filter((q) => q.questId != null));
    let done = 0;
    let likely = 0;
    for (const steps of units) {
      if (!steps.length || !steps.every(finished)) continue;
      if (steps.some((q) => !q.inferred)) done += 1;
      else likely += 1;
    }
    return { done, likely, total: units.length };
  }

  function tallyTip(r: { completedTab: number | null }, t: { done: number; likely: number; total: number }) {
    const what = grouped
      ? `${t.total} questlines and quests the quest data lists here; a questline counts once, as in the game's Completed tab.`
      : `${t.total} quests the quest data lists here, each counted.`;
    const likely = t.likely ? ` ${t.likely} more look done, since a later quest is, but the Quest window hasn't shown them done.` : "";
    const game =
      r.completedTab != null
        ? ` Your Completed tab showed ${r.completedTab} (it counts a questline once and a repeated quest each time).`
        : "";
    return `${what} The data may list quests Classic doesn't have.${likely}${game}`;
  }

  /** Regions and lines opened or closed by hand; the rest follow the filter and what's open. */
  let opened = $state<Record<string, boolean>>({});
  const regionOpen = (region: string, active: number) => opened[`r:${region}`] ?? (filter !== "all" || active > 0);
  const lineOpen = (line: Line) => opened[`l:${line.head}`] ?? line.steps.some((q) => q.questId != null && q.questId === selected);
  function toggle(key: string, now: boolean) {
    opened = { ...opened, [key]: !now };
  }

  // Ready to hand in, first: who takes each one, from the data's page for it.
  const handIn = $derived(all.filter((q) => q.questId != null && (q.stand === "ready" || (q.stand === "inProgress" && ready(q)))));
  let sheets = $state<Record<number, QuestSheet>>({});
  const handInKey = $derived(handIn.map((q) => q.questId).join(","));
  $effect(() => {
    const asked = handInKey;
    untrack(() => {
      const missing = handIn.map((q) => q.questId!).filter((id) => !sheets[id]);
      if (!missing.length) return;
      Promise.all(missing.map((id) => backend.questSheet(id, null, null, null)))
        .then((found) => {
          if (asked !== handInKey) return;
          const next = { ...sheets };
          for (const where of found) if (where) next[where.sheet.row.id] = where.sheet;
          sheets = next;
        })
        .catch(() => {});
    });
  });

  const log = $derived(questLog.log);
  const lastLook = $derived(Math.max(0, ...(log?.looks ?? []).map((l) => l.seenAt)));
  /** The Quest window's tabs not yet scrolled through, by name. */
  const unseenTabs = $derived(TABS.filter(({ tab }) => !look(tab)?.wholeAt).map(({ label }) => label));
  const ago = (at: number) => formatAgo(Date.now() / 1000 - at);
  const repeatLabel = (r: string | null) => (r === "daily" ? "Daily" : r === "weekly" ? "Weekly" : r ? "Repeatable" : null);
  const key = (q: WholeQuest) => (q.questId != null ? `#${q.questId}` : `t:${q.title}`);
  const countLine = (list: Count[]) => list.map((c) => `${c.name} ${c.have}/${c.need}`).join(" · ");

  const HEDGE =
    "The quest data knows levels and earlier quests but not jobs or events, so \"Can take\" may be wrong until you look through your Available tab.";

  async function forget() {
    if (log?.character) await backend.forgetQuestLog(log.character);
  }
</script>

{#snippet icon(stand: Stand)}
  <span class="icon" aria-label={STAND_LABEL[stand]} use:tip={STAND_LABEL[stand]}>
    {#if stand === "done"}<Check size={12} strokeWidth={2.6} />
    {:else if stand === "ready"}<CircleCheck size={12} strokeWidth={2.4} />
    {:else if stand === "inProgress"}<CircleDot size={12} strokeWidth={2.4} />
    {:else if stand === "gone"}<Undo2 size={12} strokeWidth={2.4} />
    {:else if stand === "notYet"}<Lock size={11} strokeWidth={2.4} />
    {:else}<span class="bang">!</span>{/if}
  </span>
{/snippet}

{#snippet row(q: WholeQuest, inLine: boolean)}
  {@const stand = standOf(q)}
  {@const repeat = repeatLabel(q.repeat)}
  {@const shown = stand === "inProgress" || stand === "ready" ? counts(q) : []}
  <li class="stand-{stand}" class:inferred={q.inferred} class:picked={q.questId != null && q.questId === selected}>
    {@render icon(stand)}
    <button class="pick" disabled={q.questId == null} aria-current={q.questId === selected || undefined} onclick={() => q.questId != null && onpick(q.questId)}>
      <span class="name">{#if inLine && q.line?.step}<span class="step tabular">{q.line.step}.</span>{/if}{q.title}</span>
      <span class="sub">
        {#if shown.length}<span class="counts tabular">{countLine(shown)}</span>{/if}
        {#if q.series && !inLine}<span>Listed as {q.series}</span>{/if}
        {#if q.notYet?.level != null}<span class="warn">Lv {q.notYet.level}</span>{/if}
        {#if q.notYet?.after.length && !inLine}<span>After {q.notYet.after.map((b) => b.name).join(", ")}</span>{/if}
        {#if q.notYet?.notOfferedAt != null}<span>Not on your Available tab {ago(q.notYet.notOfferedAt)}</span>{/if}
        {#if q.inferred}<span>Probably done: a later quest is</span>{/if}
        {#if q.questId == null && q.line}<span>The game's name for {q.line.name}</span>{:else if q.questId == null}<span>Not in the quest data</span>{/if}
        {#if q.levelMin != null && q.notYet?.level == null && stand !== "done"}<span>Lv {q.levelMin}</span>{/if}
        {#if q.giver && (stand === "canTake" || stand === "notYet") && !inLine}<span>From {q.giver}</span>{/if}
        {#if repeat}<span>{repeat}</span>{/if}
      </span>
    </button>
  </li>
{/snippet}

<Card title="Every quest">
  {#snippet aside()}
    {#if lastLook}<span use:tip={"When the Quest window was last read"}>As of {ago(lastLook)}</span>{/if}
    {#if whole?.level != null}<span>Lv {whole.level}</span>{/if}
  {/snippet}

  {#if !whole}
    <p class="lead">Loading the quest data…</p>
  {:else if !whole.regions.length}
    <p class="lead">Every quest shows here once the map data is downloaded.</p>
  {:else}
    <p class="lead">
      {#if !hasLog()}
        {can.liveReading ? "Open your Quest window in game" : "Share the game window and open your Quest window"}: Leafline marks what you've done and what you're on.
      {:else if unseenTabs.length}
        Scroll {unseenTabs.length > 1 ? `${unseenTabs.slice(0, -1).join(", ")} and ${unseenTabs.at(-1)}` : unseenTabs[0]} to the end in your Quest
        window to see every quest.
      {:else}
        As your Quest window last showed it; the rest is from the quest data.
      {/if}
      <span class="help" use:tip={whole.level == null ? `${HEDGE} Your level isn't read yet, so levels don't count against a quest.` : HEDGE}
        ><Info size={12} aria-label="How sure this is" /></span
      >
    </p>

    {#if handIn.length}
      <section class="hand-in" aria-label="Ready to hand in">
        <h4><CircleCheck size={12} /> Ready to hand in</h4>
        <ul class="ready">
          {#each handIn as q (key(q))}
            {@const info = sheets[q.questId!]?.info}
            <li class:picked={q.questId === selected}>
              <button class="pick name" onclick={() => onpick(q.questId!)}>{q.title}</button>
              <p class="sub">
                {#if info?.turnIn}
                  <span>Turn in to <Ref kind="npc" name={info.turnIn} /></span>
                  {#if info.turnInMaps[0]}<Ref kind="map" id={info.turnInMaps[0].id} name={info.turnInMaps[0].name} />{/if}
                  <GoButton place={npcPlace(info.turnIn, info.turnInMaps)} />
                {:else if info?.giver}
                  <span>Hand it in <span class="faint">· given by</span> <Ref kind="npc" name={info.giver} /></span>
                  <GoButton place={npcPlace(info.giver, info.giverMaps)} />
                {:else if counts(q).length}
                  <span class="tabular">{countLine(counts(q))}</span>
                {/if}
              </p>
            </li>
          {/each}
        </ul>
      </section>
    {/if}

    <div class="bar-row">
      <div class="chips"><Segmented label="Which quests" size="sm" {options} value={filter} onchange={setFilter} /></div>
      <label class="lines-switch" use:tip={"Fold the quests of a questline into one row"}>
        <span>Questlines</span>
        <Switch checked={grouped} label="Group questlines" onchange={setGrouped} />
      </label>
      {#if tools}<span class="tools">{@render tools()}</span>{/if}
    </div>
    {@render below?.()}

    {#each regions as r (r.region)}
      {@const open = regionOpen(r.region, r.active)}
      {@const t = r.tally}
      <section>
        <button class="region" aria-expanded={open} onclick={() => toggle(`r:${r.region}`, open)}>
          {#if open}<ChevronDown size={13} />{:else}<ChevronRight size={13} />{/if}
          <span class="region-name">{r.region}</span>
          <span class="tabular done-count" use:tip={tallyTip(r, t)}
            >{t.done}<span class="faint">{` of ${t.total} done`}</span>{#if t.likely}<span class="faint">{` · ${t.likely} likely`}</span>{/if}{#if r.completedTab != null && r.completedTab !== t.done}<span
                class="game">{` · game ${r.completedTab}`}</span
              >{/if}</span
          >
          <span class="bar"><span style:width="{t.total ? Math.min(100, (100 * t.done) / t.total) : 0}%"></span></span>
        </button>
        {#if open}
          <ul class="rows">
            {#each r.items as item (item.kind === "quest" ? key(item.q) : `l:${item.line.head}`)}
              {#if item.kind === "quest"}
                {@render row(item.q, false)}
              {:else}
                {@const line = item.line}
                {@const lopen = lineOpen(line)}
                {@const cur = line.current}
                {@const stand = cur ? standOf(cur) : "done"}
                <li class="line stand-{stand}">
                  {@render icon(stand)}
                  <div class="line-body">
                    <button class="line-head" aria-expanded={lopen} onclick={() => toggle(`l:${line.head}`, lopen)}>
                      <span class="name">{line.name}</span>
                      <span class="line-count tabular">{line.done}<span class="faint">{` of ${line.of}`}</span></span>
                      {#if lopen}<ChevronDown size={13} />{:else}<ChevronRight size={13} />{/if}
                    </button>
                    {#if cur && cur.questId != null}
                      <button class="on-now" class:picked={cur.questId === selected} onclick={() => onpick(cur.questId!)}>
                        <span class="faint">{stand === "inProgress" || stand === "ready" ? "On" : "Next"}</span>
                        {cur.title}
                      </button>
                    {:else if !cur}
                      <span class="sub">All done</span>
                    {/if}
                    {#if lopen}
                      <ul class="rows steps">
                        {#each line.shown as q (key(q))}{@render row(q, true)}{/each}
                      </ul>
                    {/if}
                  </div>
                </li>
              {/if}
            {/each}
          </ul>
        {/if}
      </section>
    {/each}

    {#if hasLog()}
      <p class="foot"><button class="link" onclick={forget}>Forget this log</button></p>
    {/if}
  {/if}
</Card>

<style>
  .lead {
    margin: 0 0 var(--space-2);
    color: var(--muted);
    font-size: var(--text-md);
  }

  .help {
    display: inline-flex;
    margin-left: var(--space-1);
    color: var(--faint);
    vertical-align: -1px;
    cursor: help;
  }

  .hand-in {
    margin: 0 0 var(--space-2);
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius);
    background: color-mix(in srgb, var(--good) 9%, var(--surface-2));
  }

  h4 {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    margin: 0 0 var(--space-1);
    color: var(--good);
    font-size: var(--text-xs);
    font-weight: var(--weight-strong);
  }

  .ready {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .ready li + li {
    margin-top: var(--space-1-5);
  }

  .ready .sub {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-0-5) var(--space-2);
    margin: 0;
    color: var(--muted);
    font-size: var(--text-xs);
  }

  .bar-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    margin-bottom: var(--space-2);
  }

  /* The chips scroll sideways when the list is narrower than them (a phone). */
  .chips {
    max-width: 100%;
    overflow-x: auto;
    scrollbar-width: none;
  }

  .tools {
    display: inline-flex;
    gap: var(--space-1);
  }

  .lines-switch {
    display: inline-flex;
    margin-left: auto;
    align-items: center;
    gap: var(--space-1-5);
    color: var(--muted);
    font-size: var(--text-xs);
    cursor: pointer;
  }

  section + section {
    border-top: 1px solid var(--border);
  }

  .region {
    display: grid;
    grid-template-columns: auto 1fr auto 64px;
    align-items: center;
    gap: var(--space-1-5);
    width: 100%;
    padding: var(--space-1-5) 0;
    border: 0;
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: var(--text-md);
    text-align: left;
    cursor: pointer;
  }

  .region:hover .region-name,
  .region:focus-visible .region-name {
    text-decoration: underline;
    text-decoration-color: var(--border-strong);
  }

  .region:focus-visible {
    outline: none;
  }

  .region-name {
    font-weight: var(--weight-strong);
  }

  .done-count,
  .line-count {
    font-size: var(--text-xs);
  }

  .done-count {
    cursor: help;
  }

  /* The game's own count, where it differs from Leafline's. */
  .game {
    color: var(--warn);
  }

  /* Only taken to be done: a later quest is. */
  li.inferred > .icon {
    opacity: 0.55;
  }

  .bar {
    display: block;
    height: 4px;
    border-radius: var(--radius-pill);
    background: var(--surface-3);
    overflow: hidden;
  }

  .bar span {
    display: block;
    height: 100%;
    background: var(--good);
  }

  .rows {
    margin: 0 0 var(--space-1-5);
    padding: 0 0 0 19px;
    list-style: none;
  }

  .rows.steps {
    margin: var(--space-0-5) 0 0;
    padding: 0;
  }

  .rows li {
    display: grid;
    grid-template-columns: 16px minmax(0, 1fr);
    gap: var(--space-1);
    padding: var(--space-0-5) var(--space-1);
    border-radius: var(--radius-sm);
  }

  .rows li.picked,
  .ready li.picked {
    background: color-mix(in srgb, var(--accent) 14%, transparent);
  }

  .pick {
    display: grid;
    gap: 1px;
    min-width: 0;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .pick:disabled {
    cursor: default;
  }

  .pick:not(:disabled):hover .name,
  .pick:focus-visible .name,
  .pick.name:hover,
  .pick.name:focus-visible {
    text-decoration: underline;
    text-decoration-color: var(--border-strong);
    outline: none;
  }

  .icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    height: 18px;
    color: var(--faint);
  }

  .stand-done > .icon,
  .stand-ready > .icon {
    color: var(--good);
  }

  .stand-inProgress > .icon {
    color: var(--accent-text);
  }

  .stand-canTake > .icon {
    color: var(--warn);
  }

  .bang {
    font-size: var(--text-xs);
    font-weight: var(--weight-strong);
  }

  .name {
    font-size: var(--text-md);
    font-weight: var(--weight-strong);
  }

  .stand-done > .pick .name,
  .stand-gone > .pick .name {
    color: var(--muted);
    font-weight: var(--weight-body);
  }

  .step {
    margin-right: var(--space-1);
    color: var(--faint);
    font-weight: var(--weight-body);
  }

  .sub {
    display: flex;
    flex-wrap: wrap;
    gap: 0 var(--space-2);
    color: var(--muted);
    font-size: var(--text-xs);
  }

  .sub:empty {
    display: none;
  }

  .counts {
    color: var(--text);
  }

  .line-body {
    min-width: 0;
  }

  .line-head {
    display: flex;
    align-items: center;
    gap: var(--space-1-5);
    width: 100%;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .line-head .name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .line-head:hover .name,
  .line-head:focus-visible .name {
    text-decoration: underline;
    text-decoration-color: var(--border-strong);
    outline: none;
  }

  .line-head :global(svg) {
    flex: none;
    margin-left: auto;
    color: var(--faint);
  }

  .on-now {
    display: inline-flex;
    flex-wrap: wrap;
    gap: 0 var(--space-1);
    max-width: 100%;
    margin-top: 2px;
    padding: 1px var(--space-2);
    border: 1px solid color-mix(in srgb, var(--accent) 40%, transparent);
    border-radius: var(--radius-pill);
    background: color-mix(in srgb, var(--accent) 12%, transparent);
    color: var(--accent-text);
    font: inherit;
    font-size: var(--text-xs);
    font-weight: var(--weight-strong);
    text-align: left;
    cursor: pointer;
  }

  .on-now:hover,
  .on-now:focus-visible,
  .on-now.picked {
    background: color-mix(in srgb, var(--accent) 22%, transparent);
    outline: none;
  }

  .on-now .faint {
    font-weight: var(--weight-body);
  }

  .warn {
    color: var(--warn);
  }

  .faint {
    color: var(--faint);
  }

  .foot {
    margin: var(--space-2) 0 0;
    font-size: var(--text-xs);
  }

  .link {
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--muted);
    font: inherit;
    text-decoration: underline;
    text-decoration-color: var(--border-strong);
    cursor: pointer;
  }

  .link:hover {
    color: var(--text);
  }
</style>
