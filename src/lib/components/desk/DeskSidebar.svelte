<script lang="ts">
  // The desk view's sidebar: every view and tool in one place. Who you are and how the session
  // goes are in the player bar over the views. It folds to icons in a narrow window.
  import { tip } from "$lib/tip.svelte";
  import {
    ChartLine,
    ChevronDown,
    ChevronRight,
    FlaskConical,
    Gem,
    Hammer,
    House,
    LayoutGrid,
    ListChecks,
    ListTree,
    Map as MapIcon,
    PackageSearch,
    PanelLeftClose,
    PanelLeftOpen,
    Route,
    Scale,
    Search,
    Settings,
    Share2,
    Timer,
    UserRound,
  } from "@lucide/svelte";

  import { backend } from "$lib/api";
  import Logo from "$lib/components/Logo.svelte";
  import { ZOOMS, type DeskView } from "$lib/desk";
  import { live } from "$lib/readings.svelte";
  import Segmented from "$lib/ui/Segmented.svelte";

  let {
    view,
    onview,
    folded,
    onfold,
    zoom,
    onzoom,
  }: {
    view: DeskView;
    onview: (view: DeskView) => void;
    folded: boolean;
    onfold: () => void;
    zoom: number;
    onzoom: (zoom: number) => void;
  } = $props();

  const VIEWS: { id: DeskView; title: string; icon: typeof House }[] = [
    { id: "now", title: "Now", icon: House },
    { id: "quests", title: "Quests", icon: ListChecks },
    { id: "map", title: "Map", icon: MapIcon },
    { id: "train", title: "Train", icon: ChartLine },
    { id: "me", title: "Me", icon: UserRound },
    { id: "lookup", title: "Look up", icon: Search },
  ];
  const TOOLS: { id: DeskView; title: string; icon: typeof House }[] = [
    { id: "planner", title: "Build planner", icon: ListTree },
    { id: "crafting", title: "Crafting", icon: Hammer },
    { id: "drops", title: "Drop log", icon: Gem },
    { id: "timers", title: "Timers", icon: Timer },
  ];
  const MORE: { id: DeskView; title: string; icon: typeof House }[] = [
    { id: "sell", title: "Sell or keep", icon: Scale },
    { id: "tracked", title: "Tracked items", icon: PackageSearch },
    { id: "potions", title: "Potion costs", icon: FlaskConical },
    { id: "route", title: "Route planner", icon: Route },
    { id: "share", title: "Share card", icon: Share2 },
  ];

  /** The less-used tools open under "More tools"; the one shown keeps them open. */
  let more = $state(false);
  const moreOpen = $derived(more || MORE.some((t) => t.id === view));

  const zoomOptions = ZOOMS.map((z) => ({ id: z, label: `${Math.round(z * 100)}%` }));
</script>

{#snippet item(entry: { id: DeskView; title: string; icon: typeof House }, count?: string | null)}
  {@const Icon = entry.icon}
  <button
    class="nav"
    class:on={view === entry.id}
    aria-current={view === entry.id ? "page" : undefined}
    aria-label={folded ? entry.title : undefined}
    use:tip={folded ? entry.title : undefined}
    onclick={() => onview(entry.id)}
  >
    <Icon size={16} />
    {#if !folded}<span class="label">{entry.title}</span>{#if count}<span class="count num">{count}</span>{/if}{/if}
  </button>
{/snippet}

<aside class:folded aria-label="Desk view">
  <div class="brand">
    {#if !folded}<Logo size={18} /><span class="word">Leafline</span>{/if}
    <button class="icon-btn sm fold" aria-label={folded ? "Show the sidebar" : "Fold the sidebar"} use:tip={folded ? "Show the sidebar" : "Fold the sidebar"} onclick={onfold}>
      {#if folded}<PanelLeftOpen size={16} />{:else}<PanelLeftClose size={16} />{/if}
    </button>
  </div>

  <nav aria-label="Views">
    {#if !folded}<p class="group">Views</p>{/if}
    {#each VIEWS as v (v.id)}
      {@render item(v, v.id === "quests" ? String((live.analysis?.quest?.quests ?? []).length || "") || null : null)}
    {/each}

    {#if !folded}<p class="group">Tools</p>{:else}<span class="rule"></span>{/if}
    {#each TOOLS as t (t.id)}
      {@render item(t)}
    {/each}
    {#if folded || moreOpen}
      {#each MORE as t (t.id)}
        {@render item(t)}
      {/each}
    {/if}
    {#if !folded}
      <button class="nav more" aria-expanded={moreOpen} onclick={() => (more = !moreOpen)}>
        {#if moreOpen}<ChevronDown size={14} />{:else}<ChevronRight size={14} />{/if}<span class="label">{moreOpen ? "Fewer tools" : "More tools"}</span>
      </button>
    {/if}
    {@render item({ id: "layout", title: "My layout", icon: LayoutGrid })}
  </nav>

  <div class="foot">
    {#if !folded}
      <div class="text-size">
        <span class="faint">Text</span>
        <Segmented label="Text size" size="sm" options={zoomOptions} value={zoom} onchange={onzoom} />
      </div>
    {/if}
    <button class="nav" aria-label={folded ? "Settings" : undefined} use:tip={folded ? "Settings" : undefined} onclick={() => backend.showSettings()}>
      <Settings size={16} />{#if !folded}<span class="label">Settings</span>{/if}
    </button>
    {#if !folded}
      <p class="credits">
        Unofficial: not made or approved by Nexon.<br />
        Data: OSMS (CC BY 4.0), mapleclassic.wiki (CC BY-NC-SA 4.0).
      </p>
    {/if}
  </div>
</aside>

<style>
  aside {
    display: flex;
    flex-direction: column;
    gap: var(--space-0-5);
    width: 216px;
    flex: none;
    min-height: 0;
    padding: var(--space-3) var(--space-3);
    overflow-y: auto;
    border-right: var(--line-w) solid var(--border);
    background: color-mix(in srgb, var(--surface) 35%, var(--bg));
  }

  aside.folded {
    width: 60px;
    align-items: center;
    padding: var(--space-3) var(--space-2);
  }

  .brand {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-0-5) var(--space-1) var(--space-2) var(--space-2);
  }

  .folded .brand {
    padding: var(--space-0-5) 0 var(--space-2);
  }

  .word {
    font-family: var(--font-display);
    font-size: var(--text-lg);
    font-weight: var(--weight-strong);
  }

  .fold {
    margin-left: auto;
  }

  .folded .fold {
    margin: 0;
  }

  .num {
    font-family: var(--font-display);
    font-variant-numeric: tabular-nums;
  }

  nav {
    display: flex;
    flex-direction: column;
    gap: 1px;
    width: 100%;
  }

  .group {
    margin: var(--space-2) 0 var(--space-0-5);
    padding: 0 var(--space-3);
    color: var(--faint);
    font-size: var(--text-xs);
    font-weight: var(--weight-body);
  }

  .rule {
    display: block;
    margin: var(--space-2) var(--space-1-5);
    border-top: 1px solid var(--border);
  }

  .nav {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
    height: 34px;
    padding: 0 var(--space-3);
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--muted);
    font: inherit;
    font-size: var(--text-md);
    font-weight: var(--weight-body);
    text-align: left;
    cursor: pointer;
  }

  .folded .nav {
    justify-content: center;
    width: 40px;
    padding: 0;
  }

  .nav:hover {
    background: var(--hover);
    color: var(--text);
  }

  .nav.on {
    background: var(--accent-soft);
    color: var(--accent-text-strong);
  }

  .nav:focus-visible,
  .fold:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }

  .label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .count {
    color: var(--faint);
    font-size: var(--text-xs);
  }

  .nav.on .count {
    color: inherit;
  }

  .nav.more {
    height: 28px;
    color: var(--faint);
    font-size: var(--text-md);
  }

  .foot {
    display: grid;
    gap: var(--space-1);
    width: 100%;
    margin-top: auto;
    padding-top: var(--space-2);
  }

  .text-size {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-1) var(--space-1-5);
    padding: 0 0 0 var(--space-3);
  }

  .faint {
    color: var(--faint);
    font-size: var(--text-xs);
  }

  .credits {
    margin: var(--space-1-5) 0 0;
    padding: var(--space-2) var(--space-3) 0;
    border-top: 1px solid var(--border);
    color: var(--faint);
    font-size: var(--text-2xs);
    line-height: 15px;
  }
</style>
