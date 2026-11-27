<script lang="ts">
  // The desk's search box, typed in where it sits: matches drop down under it as you type, grouped
  // by kind with a picture and a line each, and chips with counts narrow them to one kind. Arrow
  // keys move, Enter opens the pick in Look up (a quest on the Quests page), Esc closes. With
  // nothing picked, Enter takes the words to Look up. The same searches Look up runs, on this PC.
  // On a page with a list of its own (Quests), it filters that list as you type instead, with no
  // drop-down over the list; Enter, or the button in the box, takes the words to Look up.
  import { Flag, LoaderCircle, MapPin, Package, Search, Skull, UserRound, X } from "@lucide/svelte";
  import { untrack, type Component } from "svelte";

  import { backend } from "$lib/api";
  import { blankQuery, itemSlot } from "$lib/items.svelte";
  import { mapData, watchMapData } from "$lib/mapdata.svelte";
  import { blankMonsterQuery } from "$lib/monsters.svelte";
  import { blankNpcQuery } from "$lib/places.svelte";
  import { blankSearch } from "$lib/quests.svelte";
  import { live } from "$lib/readings.svelte";
  import type { PictureKind } from "$lib/pictures";
  import Pic from "$lib/ui/Pic.svelte";

  export type Kind = "items" | "monsters" | "maps" | "npcs" | "quests";

  let {
    onopen,
    onwords,
    filter = null,
  }: {
    /** Opens one match: in Look up, or a quest on the Quests page. */
    onopen: (kind: Kind, id: number) => void;
    /** Takes the words to Look up's search, when nothing is picked. */
    onwords: (words: string) => void;
    /** The page's own list, filtered by what's typed, in place of the drop-down's matches. */
    filter?: { text: string; placeholder: string; label: string; onchange: (text: string) => void } | null;
  } = $props();

  watchMapData();

  /** How many of each kind show under All; a kind's chip shows more. */
  const FEW = 4;
  const MANY = 12;

  type Row = { kind: Kind; id: number; name: string; line: string | null; aside: string | null; picture: PictureKind | null };
  type Found = Record<Kind, { rows: Row[]; total: number }>;

  const KINDS: { id: Kind; label: string; icon: Component }[] = [
    { id: "items", label: "Items", icon: Package },
    { id: "monsters", label: "Monsters", icon: Skull },
    { id: "maps", label: "Maps", icon: MapPin },
    { id: "npcs", label: "NPCs", icon: UserRound },
    { id: "quests", label: "Quests", icon: Flag },
  ];
  const ICON = Object.fromEntries(KINDS.map((k) => [k.id, k.icon])) as Record<Kind, Component>;

  let box = $state<HTMLInputElement>();
  let text = $state("");
  let open = $state(false);
  let chip = $state<Kind | "all">("all");
  let active = $state(-1);
  let found = $state<Found | null>(null);
  let searching = $state(false);
  let asked = 0;

  /** What the box shows: the page's filter, or the words searched for everywhere. */
  const typed = $derived(filter ? filter.text : text);
  const words = $derived(typed.trim());
  function setTyped(value: string) {
    if (filter) filter.onchange(value);
    else text = value;
  }
  const ready = $derived(mapData.status?.ready ?? false);
  const level = $derived(live.analysis?.hud?.level ?? live.session?.level ?? null);
  const character = $derived(live.analysis?.hud?.name ?? null);

  const levelSpan = (min: number | null, max?: number | null) =>
    min == null ? null : max != null && max !== min ? `Lv ${min}–${max}` : `Lv ${min}`;

  // Every kind at once, a moment after the typing stops.
  $effect(() => {
    const q = words;
    if (!q || !ready || filter) {
      found = null;
      searching = false;
      return;
    }
    const mine = ++asked;
    searching = true;
    const timer = setTimeout(async () => {
      const [items, monsters, maps, npcs, quests] = await Promise.all([
        untrack(() => backend.searchItems({ ...blankQuery(), text: q }, level, character)).catch(() => null),
        backend.searchMonsters({ ...blankMonsterQuery(), text: q }).catch(() => null),
        backend.searchMaps(q).catch(() => null),
        backend.searchNpcs({ ...blankNpcQuery(), text: q }).catch(() => null),
        backend.searchQuests({ ...blankSearch(), text: q }).catch(() => null),
      ]);
      if (mine !== asked) return;
      // "Closest" pages are guesses for when nothing matched: noise in a drop-down.
      const realItems = items && !items.closest ? items : null;
      const realMobs = monsters && !monsters.closest ? monsters : null;
      const realNpcs = npcs && !npcs.closest ? npcs : null;
      const knownMaps = (maps ?? []).filter((m) => m.known);
      found = {
        items: {
          total: realItems?.total ?? 0,
          rows: (realItems?.items ?? []).slice(0, MANY).map((i) => ({
            kind: "items",
            id: i.id,
            name: i.name,
            line: itemSlot(i),
            aside: i.kind === "equip" && i.reqLevel ? `Lv ${i.reqLevel}` : null,
            picture: "item",
          })),
        },
        monsters: {
          total: realMobs?.total ?? 0,
          rows: (realMobs?.monsters ?? []).slice(0, MANY).map((m) => ({
            kind: "monsters",
            id: m.id,
            name: m.name,
            line: m.boss ? "Boss" : (m.note ?? null),
            aside: `Lv ${m.level}`,
            picture: "mob",
          })),
        },
        maps: {
          total: knownMaps.length,
          rows: knownMaps.slice(0, MANY).map((m) => ({ kind: "maps", id: m.id, name: m.name, line: m.street, aside: null, picture: "mark" })),
        },
        npcs: {
          total: realNpcs?.total ?? 0,
          rows: (realNpcs?.npcs ?? []).slice(0, MANY).map((n) => ({
            kind: "npcs",
            id: n.id,
            name: n.name,
            line: n.map ?? null,
            aside: n.quests ? `${n.quests} quest${n.quests === 1 ? "" : "s"}` : null,
            picture: "npc",
          })),
        },
        quests: {
          total: quests?.total ?? 0,
          rows: (quests?.quests ?? []).slice(0, MANY).map((r) => ({
            kind: "quests",
            id: r.id,
            name: r.name,
            line: [r.region, r.giver].filter(Boolean).join(" · ") || null,
            aside: levelSpan(r.levelMin),
            picture: null,
          })),
        },
      };
      searching = false;
      active = -1;
    }, 140);
    return () => clearTimeout(timer);
  });

  /** The groups that show, each cut to a few under All. */
  const groups = $derived(
    found
      ? KINDS.filter((k) => (chip === "all" || chip === k.id) && found![k.id].total > 0).map((k) => ({
          ...k,
          total: found![k.id].total,
          rows: found![k.id].rows.slice(0, chip === "all" ? FEW : MANY),
        }))
      : [],
  );
  /** Every row shown, in order, for the arrow keys. */
  const flat = $derived(groups.flatMap((g) => g.rows));
  const total = $derived(found ? KINDS.reduce((n, k) => n + found![k.id].total, 0) : 0);
  const showing = $derived(open && words !== "" && !filter);

  function pick(row: Row) {
    onopen(row.kind, row.id);
    close();
  }

  function close() {
    open = false;
    active = -1;
    box?.blur();
  }

  function keydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      if (typed && open) close();
      else {
        setTyped("");
        close();
      }
    } else if (filter && event.key === "Enter") {
      // Typing already filtered the list; Enter searches everything else.
      event.preventDefault();
      if (words) everywhere();
    } else if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      if (!flat.length) return;
      event.preventDefault();
      open = true;
      const step = event.key === "ArrowDown" ? 1 : -1;
      active = (active + step + flat.length + (active < 0 && step < 0 ? 1 : 0)) % flat.length;
      document.getElementById(`desk-search-${active}`)?.scrollIntoView({ block: "nearest" });
    } else if (event.key === "Enter") {
      event.preventDefault();
      if (active >= 0 && flat[active]) pick(flat[active]);
      else if (words) {
        onwords(words);
        close();
      }
    }
  }

  /** The page's filter words, searched for everywhere in Look up. */
  function everywhere() {
    onwords(words);
    close();
  }

  /** Focuses the box, from the "/" key. */
  export function focus() {
    box?.focus();
    box?.select();
  }

  function outside(node: HTMLElement) {
    const down = (e: PointerEvent) => {
      if (!node.contains(e.target as Node)) open = false;
    };
    document.addEventListener("pointerdown", down);
    return { destroy: () => document.removeEventListener("pointerdown", down) };
  }

  const index = (row: Row) => flat.indexOf(row);
</script>

<div class="wrap" use:outside>
  <label class="box" class:open={showing}>
    <Search size={15} />
    <input
      bind:this={box}
      type="search"
      placeholder={filter?.placeholder ?? "Search items, monsters, maps, NPCs, quests…"}
      aria-label={filter?.label ?? "Search everything"}
      role="combobox"
      aria-expanded={showing}
      aria-controls="desk-search-list"
      aria-activedescendant={active >= 0 ? `desk-search-${active}` : undefined}
      autocomplete="off"
      spellcheck="false"
      value={typed}
      oninput={(e) => {
        setTyped(e.currentTarget.value);
        open = true;
        active = -1;
      }}
      onfocus={() => (open = true)}
      onkeydown={keydown}
    />
    {#if searching}<span class="spin" aria-hidden="true"><LoaderCircle size={14} /></span>{/if}
    {#if filter && words}<button class="everywhere" onclick={everywhere}>Search everything <kbd>↵</kbd></button>{/if}
    {#if typed}
      <button class="clear" aria-label="Clear the search" onclick={() => (setTyped(""), box?.focus())}><X size={13} /></button>
    {:else}
      <kbd>/</kbd>
    {/if}
  </label>

  {#if showing}
    <div class="drop" id="desk-search-list" role="listbox" aria-label="Matches">
      {#if !ready}
        <p class="quiet">Searching needs the map data. Download it once in the Map tab.</p>
      {:else if found}
        <div class="chips" role="group" aria-label="Show">
          <button aria-pressed={chip === "all"} onclick={() => ((chip = "all"), (active = -1))}>All <span class="n">{total.toLocaleString()}</span></button>
          {#each KINDS as k (k.id)}
            {#if found[k.id].total}
              <button aria-pressed={chip === k.id} onclick={() => ((chip = k.id), (active = -1))}
                >{k.label} <span class="n">{found[k.id].total.toLocaleString()}</span></button
              >
            {/if}
          {/each}
        </div>
        {#if !groups.length}
          <p class="quiet">Nothing matches “{words}”.</p>
        {/if}
        {#each groups as g (g.id)}
          <div class="group-head">
            <h3>{g.label}</h3>
            {#if g.total > g.rows.length}<button class="more" onclick={() => ((chip = g.id), (active = -1))}>{chip === "all" ? `All ${g.total.toLocaleString()}` : `${g.rows.length} of ${g.total.toLocaleString()}`}</button>{/if}
          </div>
          <ul>
            {#each g.rows as row (`${row.kind}:${row.id}`)}
              {@const i = index(row)}
              {@const Icon = ICON[row.kind]}
              <li>
                <button
                  id="desk-search-{i}"
                  class="row"
                  class:active={i === active}
                  role="option"
                  aria-selected={i === active}
                  onpointerenter={() => (active = i)}
                  onclick={() => pick(row)}
                >
                  <span class="pic">
                    {#if row.picture}
                      <Pic kind={row.picture} id={row.id} size={24}>{#snippet fallback()}<Icon size={15} />{/snippet}</Pic>
                    {:else}<Icon size={15} />{/if}
                  </span>
                  <span class="text">
                    <span class="name">{row.name}</span>
                    {#if row.line}<span class="line">{row.line}</span>{/if}
                  </span>
                  {#if row.aside}<span class="aside tabular">{row.aside}</span>{/if}
                </button>
              </li>
            {/each}
          </ul>
        {/each}
        <p class="foot"><kbd>↑</kbd><kbd>↓</kbd> to move · <kbd>Enter</kbd> to open · <kbd>Esc</kbd> to close</p>
      {:else}
        <p class="quiet"><span class="spin"><LoaderCircle size={13} /></span> Searching…</p>
      {/if}
    </div>
  {/if}
</div>

<style>
  .wrap {
    position: relative;
    flex: 0 1 440px;
    min-width: 200px;
  }

  .box {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    height: var(--control-h);
    padding: 0 var(--space-2) 0 var(--space-3);
    border: var(--line-w) solid var(--control-line);
    border-radius: var(--radius);
    background: var(--surface);
    color: var(--faint);
    cursor: text;
  }

  .box:focus-within {
    border-color: color-mix(in srgb, var(--accent) 60%, transparent);
  }

  .box input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: 0;
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: var(--text-md);
  }

  .box input::-webkit-search-cancel-button {
    display: none;
  }

  kbd {
    padding: 0 var(--space-1-5);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    color: var(--faint);
    font-family: inherit;
    font-size: var(--text-xs);
  }

  .everywhere {
    display: inline-flex;
    flex: none;
    align-items: center;
    gap: var(--space-1);
    padding: 2px var(--space-1) 2px var(--space-2);
    border: 0;
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    color: var(--muted);
    font: inherit;
    font-size: var(--text-xs);
    white-space: nowrap;
    cursor: pointer;
  }

  .everywhere:hover,
  .everywhere:focus-visible {
    color: var(--text);
    outline: none;
  }

  .everywhere kbd {
    padding: 0 var(--space-1);
  }

  .clear {
    display: inline-grid;
    place-items: center;
    padding: 2px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--faint);
    cursor: pointer;
  }

  .clear:hover {
    color: var(--text);
  }

  .drop {
    position: absolute;
    z-index: 30;
    top: calc(100% + var(--space-1));
    left: 0;
    width: max(100%, min(560px, 90vw));
    max-height: min(70vh, 620px);
    overflow-y: auto;
    padding: var(--space-2);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg);
    background: var(--surface);
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.35);
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1);
    margin-bottom: var(--space-1-5);
  }

  .chips button {
    display: inline-flex;
    gap: var(--space-1);
    align-items: baseline;
    padding: 2px var(--space-2);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-pill);
    background: transparent;
    color: var(--muted);
    font: inherit;
    font-size: var(--text-xs);
    cursor: pointer;
  }

  .chips button[aria-pressed="true"] {
    border-color: transparent;
    background: var(--accent);
    color: var(--accent-ink);
  }

  .chips .n {
    opacity: 0.75;
    font-variant-numeric: tabular-nums;
  }

  .group-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    margin: var(--space-1-5) var(--space-1) var(--space-0-5);
  }

  h3 {
    margin: 0;
    color: var(--faint);
    font-size: var(--text-xs);
    font-weight: var(--weight-strong);
    letter-spacing: 0.02em;
  }

  .more {
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--accent-text);
    font: inherit;
    font-size: var(--text-xs);
    cursor: pointer;
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .row {
    display: grid;
    grid-template-columns: 28px minmax(0, 1fr) auto;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
    padding: var(--space-1) var(--space-1-5);
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .row.active {
    background: var(--accent-soft);
  }

  .pic {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    color: var(--faint);
  }

  .text {
    display: grid;
    min-width: 0;
  }

  .name,
  .line {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .name {
    font-size: var(--text-md);
    font-weight: var(--weight-strong);
  }

  .line {
    color: var(--muted);
    font-size: var(--text-xs);
  }

  .aside {
    color: var(--faint);
    font-size: var(--text-xs);
  }

  .quiet {
    margin: var(--space-2);
    color: var(--muted);
    font-size: var(--text-md);
  }

  .foot {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1);
    align-items: center;
    margin: var(--space-2) var(--space-1) 0;
    color: var(--faint);
    font-size: var(--text-xs);
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
</style>
