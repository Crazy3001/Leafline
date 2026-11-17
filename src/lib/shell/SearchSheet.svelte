<script lang="ts">
  // Search, in one box: items, monsters, maps, NPCs, skills and Leafline's own tools, a few of
  // each as you type. Picking a result opens its page; "See all" hands the words to that kind's
  // own list, with its filters. With nothing typed the lists are there to browse, and that is also
  // where an item check or a link from elsewhere lands.
  import { ArrowLeft, LoaderCircle, Search, X } from "@lucide/svelte";
  import { onMount, untrack } from "svelte";

  import { backend } from "$lib/api";
  import LookupRow from "$lib/components/lookup/LookupRow.svelte";
  import LookupPanel from "$lib/components/LookupPanel.svelte";
  import { blankQuery, itemSlot, itemView, lookup } from "$lib/items.svelte";
  import { mapData, watchMapData } from "$lib/mapdata.svelte";
  import { blankMonsterQuery, monsterView } from "$lib/monsters.svelte";
  import { blankMapQuery, blankNpcQuery, mapView, npcView } from "$lib/places.svelte";
  import { live } from "$lib/readings.svelte";
  import { loadSkills, skillData, skillView } from "$lib/skills.svelte";
  import type { ItemRecord, MapRef, MonsterRow, NpcRow, SkillInfo } from "$lib/types";

  import { TOOLS, type ToolInfo } from "./tabs";

  let {
    focus = false,
    landed = 0,
    ontool,
  }: {
    /** Put the cursor in the box on opening: for the search button, not for a result that opens by itself. */
    focus?: boolean;
    /** Counts up each time something outside opens a page here, so the lists show instead of results. */
    landed?: number;
    ontool: (tool: ToolInfo) => void;
  } = $props();

  /** How many of each kind show before "See all". */
  const FEW = 4;

  type Kind = "items" | "monsters" | "maps" | "npcs" | "skills";
  interface Found {
    items: { rows: ItemRecord[]; total: number };
    monsters: { rows: MonsterRow[]; total: number };
    maps: { rows: MapRef[]; total: number };
    npcs: { rows: NpcRow[]; total: number };
  }

  let box = $state<HTMLInputElement>();
  let text = $state("");
  /** A page or a kind's list is showing over the results, which Back returns to. */
  let browsing = $state(false);
  let found = $state<Found | null>(null);
  let searching = $state(false);
  let asked = 0;

  const words = $derived(text.trim());
  const ready = $derived(mapData.status?.ready ?? false);
  const level = $derived(live.analysis?.hud?.level ?? live.session?.level ?? null);
  const character = $derived(live.analysis?.hud?.name ?? null);

  watchMapData();
  onMount(() => {
    void loadSkills();
    if (focus) box?.focus();
  });

  // Something outside opened a page (an item check, a link): show it, not the results.
  $effect(() => {
    if (landed) browsing = true;
  });

  const tools = $derived.by(() => {
    const q = words.toLowerCase();
    if (!q) return [];
    // A word of the tool's name starting with what was typed; its line counts once there is enough to go on.
    return TOOLS.filter(
      (t) =>
        t.title.toLowerCase().split(" ").some((word) => word.startsWith(q)) ||
        (q.length >= 3 && (t.title.toLowerCase().includes(q) || t.line.toLowerCase().includes(q))),
    );
  });

  const skills = $derived.by(() => {
    const q = words.toLowerCase();
    if (!q) return { rows: [] as SkillInfo[], total: 0 };
    const all = skillData.skills.filter((s) => s.name.toLowerCase().includes(q));
    // Names that start with the words first.
    all.sort((a, b) => Number(b.name.toLowerCase().startsWith(q)) - Number(a.name.toLowerCase().startsWith(q)));
    return { rows: all.slice(0, FEW), total: all.length };
  });

  // The four searches that go to the map data, a moment after the typing stops.
  $effect(() => {
    const q = words;
    const ok = ready;
    if (!q || !ok) {
      found = null;
      searching = false;
      return;
    }
    const mine = ++asked;
    searching = true;
    const timer = setTimeout(async () => {
      const [items, monsters, maps, npcs] = await Promise.all([
        untrack(() => backend.searchItems({ ...blankQuery(), text: q }, level, character)).catch(() => null),
        backend.searchMonsters({ ...blankMonsterQuery(), text: q }).catch(() => null),
        backend.searchMaps(q).catch(() => null),
        backend.searchNpcs({ ...blankNpcQuery(), text: q }).catch(() => null),
      ]);
      if (mine !== asked) return;
      // "Closest" pages are guesses for when nothing matched: fine in a list of one kind, noise here.
      found = {
        items: { rows: items && !items.closest ? items.items.slice(0, FEW) : [], total: items && !items.closest ? items.total : 0 },
        monsters: {
          rows: monsters && !monsters.closest ? monsters.monsters.slice(0, FEW) : [],
          total: monsters && !monsters.closest ? monsters.total : 0,
        },
        maps: { rows: (maps ?? []).filter((m) => m.known).slice(0, FEW), total: (maps ?? []).filter((m) => m.known).length },
        npcs: { rows: npcs && !npcs.closest ? npcs.npcs.slice(0, FEW) : [], total: npcs && !npcs.closest ? npcs.total : 0 },
      };
      searching = false;
    }, 160);
    return () => clearTimeout(timer);
  });

  const nothing = $derived(
    found != null &&
      !searching &&
      !tools.length &&
      !skills.total &&
      !found.items.total &&
      !found.monsters.total &&
      !found.maps.total &&
      !found.npcs.total,
  );

  function type(value: string) {
    text = value;
    browsing = false;
  }

  function clear() {
    text = "";
    browsing = false;
    box?.focus();
  }

  /** Opens one result's page in its kind's list. */
  function open(kind: Kind, id: number) {
    lookup.mode = kind;
    if (kind === "items") itemView.id = id;
    else if (kind === "monsters") {
      monsterView.trail = [];
      monsterView.id = id;
    } else if (kind === "maps") {
      mapView.trail = [];
      mapView.id = id;
    } else if (kind === "npcs") npcView.id = id;
    else skillView.id = id;
    browsing = true;
  }

  /** Hands the words to a kind's own list, where its filters are. */
  function seeAll(kind: Kind) {
    lookup.mode = kind;
    if (kind === "items") {
      itemView.id = null;
      itemView.query = { ...blankQuery(), text: words };
    } else if (kind === "monsters") {
      monsterView.id = null;
      monsterView.trail = [];
      monsterView.query = { ...blankMonsterQuery(), text: words };
    } else if (kind === "maps") {
      mapView.id = null;
      mapView.trail = [];
      mapView.query = { ...blankMapQuery(), text: words };
    } else if (kind === "npcs") {
      npcView.id = null;
      npcView.query = { ...blankNpcQuery(), text: words };
    } else skillView.id = null;
    browsing = true;
  }

  const npcLine = (n: NpcRow) => [n.map, n.maps > 1 ? `+${n.maps - 1} more` : null].filter(Boolean).join(" · ") || null;
  const skillLine = (s: SkillInfo) => `${s.className}${s.jobTier && s.jobTier !== s.className ? ` · ${s.jobTier}` : ""}${s.passive ? " · passive" : ""}`;
</script>

{#snippet group(title: string, total: number, kind: Kind)}
  <div class="group-head">
    <h3>{title}</h3>
    {#if total > FEW}
      <button class="all" onclick={() => seeAll(kind)}>See all {total.toLocaleString()}</button>
    {/if}
  </div>
{/snippet}

<div class="search">
  <label class="box">
    <Search size={15} />
    <input
      bind:this={box}
      type="search"
      placeholder="Items, monsters, maps, NPCs, skills, tools"
      aria-label="Search everything"
      value={text}
      oninput={(e) => type(e.currentTarget.value)}
    />
    {#if searching}<span class="spin" aria-hidden="true"><LoaderCircle size={14} /></span>{/if}
  </label>
  {#if text}
    <button class="clear" aria-label="Clear the search" onclick={clear}><X size={14} /></button>
  {/if}
</div>

{#if words && !browsing}
  <div class="results" aria-live="polite">
    {#if tools.length}
      <div class="group-head"><h3>Tools</h3></div>
      <ul>
        {#each tools as tool (tool.id)}
          {@const Icon = tool.icon}
          <li>
            <button class="tool nav-row" onclick={() => ontool(tool)}>
              <span class="glyph"><Icon size={14} /></span>
              <span class="tool-name">{tool.title}</span>
              <span class="tool-line">{tool.line}</span>
            </button>
          </li>
        {/each}
      </ul>
    {/if}

    {#if !ready}
      <p class="quiet">Items, monsters, maps and NPCs need the map data. Download it once in the Map tab.</p>
    {:else if found}
      {#if found.items.total}
        {@render group("Items", found.items.total, "items")}
        <ul>
          {#each found.items.rows as item (item.id)}
            <li>
              <LookupRow
                name={item.name}
                picture={{ kind: "item", id: item.id }}
                line={itemSlot(item)}
                aside={item.kind === "equip" && item.reqLevel ? `Lv ${item.reqLevel}` : null}
                onclick={() => open("items", item.id)}
              />
            </li>
          {/each}
        </ul>
      {/if}
      {#if found.monsters.total}
        {@render group("Monsters", found.monsters.total, "monsters")}
        <ul>
          {#each found.monsters.rows as mob (mob.id)}
            <li>
              <LookupRow
                name={mob.name}
                picture={{ kind: "mob", id: mob.id }}
                line={mob.boss ? "Boss" : mob.note}
                aside={`Lv ${mob.level}`}
                onclick={() => open("monsters", mob.id)}
              />
            </li>
          {/each}
        </ul>
      {/if}
      {#if found.maps.total}
        {@render group("Maps", found.maps.total, "maps")}
        <ul>
          {#each found.maps.rows as map (map.id)}
            <li><LookupRow name={map.name} picture={{ kind: "mark", id: map.id }} line={map.street} onclick={() => open("maps", map.id)} /></li>
          {/each}
        </ul>
      {/if}
      {#if found.npcs.total}
        {@render group("NPCs", found.npcs.total, "npcs")}
        <ul>
          {#each found.npcs.rows as npc (npc.id)}
            <li>
              <LookupRow
                name={npc.name}
                picture={{ kind: "npc", id: npc.id }}
                line={npcLine(npc)}
                aside={npc.quests ? `${npc.quests} quest${npc.quests === 1 ? "" : "s"}` : null}
                onclick={() => open("npcs", npc.id)}
              />
            </li>
          {/each}
        </ul>
      {/if}
    {/if}

    {#if skills.total}
      {@render group("Skills", skills.total, "skills")}
      <ul>
        {#each skills.rows as skill (skill.id)}
          <li>
            <LookupRow
              name={skill.name}
              picture={{ kind: "skill", id: skill.id }}
              line={skillLine(skill)}
              aside={`max ${skill.maxLevel}`}
              onclick={() => open("skills", skill.id)}
            />
          </li>
        {/each}
      </ul>
    {/if}

    {#if nothing}
      <p class="quiet">Nothing matches “{words}”.</p>
    {/if}
  </div>
{:else}
  {#if words}
    <button class="back" onclick={() => (browsing = false)}><ArrowLeft size={13} />Results for “{words}”</button>
  {/if}
  <LookupPanel
    tooltips={live.analysis?.tooltips ?? []}
    {level}
    {character}
    job={live.analysis?.hud?.job ?? null}
    map={live.analysis?.map ?? null}
    quest={live.analysis?.quest ?? null}
    session={live.session}
  />
{/if}

<style>
  .search {
    display: flex;
    gap: 6px;
    padding: var(--space-3) var(--space-3) 0;
  }

  .box {
    display: flex;
    flex: 1;
    align-items: center;
    gap: 8px;
    min-width: 0;
    height: 38px;
    padding: 0 12px;
    border: var(--line-w) solid var(--control-line);
    border-radius: var(--radius-lg);
    background: var(--surface);
    color: var(--faint);
    cursor: text;
  }

  .box:focus-within {
    border-color: var(--accent);
    outline: 2px solid color-mix(in srgb, var(--accent) 35%, transparent);
    outline-offset: 0;
  }

  input {
    flex: 1;
    min-width: 0;
    padding: 0;
    border: 0;
    outline: 0;
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: 14px;
  }

  input::placeholder {
    color: var(--faint);
  }

  input::-webkit-search-cancel-button {
    display: none;
  }

  .clear {
    display: grid;
    flex: none;
    place-items: center;
    width: 38px;
    height: 38px;
    padding: 0;
    border: var(--line-w) solid var(--control-line);
    border-radius: var(--radius-lg);
    background: var(--surface);
    color: var(--muted);
    cursor: pointer;
  }

  .clear:hover {
    background: var(--surface-2);
    color: var(--text);
  }

  .spin {
    display: inline-grid;
    animation: spin 0.9s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .results {
    padding: 4px var(--space-3) var(--space-3);
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .group-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 10px;
    margin: 12px 2px 4px;
  }

  h3 {
    margin: 0;
    color: var(--faint);
    font-family: var(--font-display);
    font-size: var(--text-sm);
    font-weight: 600;
  }

  .all,
  .back {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--accent-text);
    font: inherit;
    font-size: var(--text-sm);
    font-weight: 600;
    cursor: pointer;
  }

  .all:hover,
  .back:hover {
    color: var(--accent-text-strong);
    text-decoration: underline;
    text-underline-offset: 2px;
  }

  .back {
    margin: 10px var(--space-3) 0;
  }

  .tool {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    align-items: center;
    gap: 0 9px;
    width: 100%;
    padding: 6px 24px 6px 6px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text);
    text-align: left;
    cursor: pointer;
  }

  .tool:hover,
  .tool:focus-visible {
    background: var(--hover);
    outline: none;
  }

  .glyph {
    display: grid;
    grid-row: span 2;
    place-items: center;
    width: 28px;
    height: 28px;
    border-radius: var(--radius-sm);
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .tool-name {
    font-weight: var(--weight-strong);
    line-height: 1.25;
  }

  .tool-line {
    color: var(--faint);
    font-size: var(--text-xs);
  }

  .quiet {
    margin: 14px 2px 0;
    color: var(--faint);
    font-size: var(--text-sm);
  }
</style>
