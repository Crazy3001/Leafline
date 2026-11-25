<script lang="ts">
  // The desk view's "My layout": the player's own columns of panels, in tabs they name and
  // arrange. This was the whole desk view before it had views of its own.
  import { tip } from "$lib/tip.svelte";
  import { ChartLine, ChevronDown, Milestone, Pencil, Plus, RotateCcw, X } from "@lucide/svelte";

  import PanelView from "$lib/components/PanelView.svelte";
  import TrainingLog from "$lib/components/TrainingLog.svelte";
  import BuildPlanner from "$lib/components/planner/BuildPlanner.svelte";
  import {
    DESK_PANELS,
    MAX_COLUMNS,
    MAX_NAME,
    defaultTab,
    find,
    place,
    remove,
    split,
    unused,
    weight,
    withColumns,
    type DeskColumn,
    type DeskPanel,
    type DeskPrefs,
  } from "$lib/desk";
  import { panelInfo } from "$lib/panels";
  import { live } from "$lib/readings.svelte";
  import Segmented from "$lib/ui/Segmented.svelte";

  let { prefs = $bindable(), onsave }: { prefs: DeskPrefs; onsave: () => void } = $props();

  const tab = $derived(prefs.tabs.find((t) => t.id === prefs.active) ?? prefs.tabs[0]);
  const columns = $derived(tab.columns);
  /** The tab being renamed, and what has been typed so far. */
  let renaming = $state<{ id: string; text: string } | null>(null);
  /** The column and place whose Change menu is open. */
  let menu = $state<{ column: number; slot: number } | null>(null);

  function change(next: DeskColumn[]) {
    menu = null;
    tab.columns = next;
    onsave();
  }

  function activate(id: string) {
    menu = null;
    renaming = null;
    prefs.active = id;
    onsave();
  }

  /** Puts the shown tab back to its name and layout from the start. */
  function reset() {
    const fresh = defaultTab(tab.id);
    tab.name = fresh.name;
    change(fresh.columns);
  }

  function startRename(id: string) {
    const target = prefs.tabs.find((t) => t.id === id);
    if (!target) return;
    activate(id);
    renaming = { id, text: target.name };
  }

  function finishRename(keep: boolean) {
    if (!renaming) return;
    const target = prefs.tabs.find((t) => t.id === renaming!.id);
    const name = renaming.text.trim().slice(0, MAX_NAME);
    renaming = null;
    if (keep && target && name) {
      target.name = name;
      onsave();
    }
  }

  /** Focuses and selects a box as it appears. */
  function selectAll(node: HTMLInputElement) {
    node.focus();
    node.select();
  }

  const columnOptions = Array.from({ length: MAX_COLUMNS }, (_, i) => ({ id: i + 1, label: `${i + 1}` }));

  /** Where else a panel is, for the Change menu. */
  function whereElse(panel: DeskPanel, column: number, slot: number): string | null {
    const at = find(columns, panel);
    if (!at || (at[0] === column && at[1] === slot)) return null;
    return at[0] === column ? (at[1] === 0 ? "Above" : "Below") : `Column ${at[0] + 1}`;
  }

  /** A panel's name and icon. The training log and the planner aren't overlay tabs. */
  function deskInfo(panel: DeskPanel) {
    if (panel === "log") return { title: "Training log", icon: ChartLine };
    if (panel === "planner") return { title: "Build planner", icon: Milestone };
    return panelInfo(panel);
  }

  const template = $derived(columns.map((c) => `minmax(0, ${weight(c)}fr)`).join(" "));

  function closeMenuOutside(event: MouseEvent) {
    if (menu && !(event.target as Element).closest?.(".menu, .change")) menu = null;
  }
</script>

<svelte:window
  onclick={closeMenuOutside}
  onkeydown={(e) => {
    if (e.key === "Escape") menu = null;
  }}
/>

<div class="layout">
  <nav class="tabs" aria-label="Your layouts">
    <div class="tablist" role="tablist">
      {#each prefs.tabs as t (t.id)}
        {#if renaming?.id === t.id}
          <input
            class="rename"
            aria-label="Layout name"
            maxlength={MAX_NAME}
            bind:value={renaming.text}
            use:selectAll
            onblur={() => finishRename(true)}
            onkeydown={(e) => {
              if (e.key === "Enter") finishRename(true);
              else if (e.key === "Escape") finishRename(false);
            }}
          />
        {:else}
          <button
            role="tab"
            class="tab"
            aria-selected={t.id === tab.id}
            use:tip={t.id === tab.id ? "Double-click to rename" : undefined}
            onclick={() => activate(t.id)}
            ondblclick={() => startRename(t.id)}>{t.name}</button
          >
        {/if}
      {/each}
    </div>
    <span class="tab-actions">
      <span class="field">
        <span class="faint">Columns</span>
        <Segmented label="Columns" size="sm" options={columnOptions} value={columns.length} onchange={(n) => change(withColumns(columns, n))} />
      </span>
      <button class="small" onclick={() => startRename(tab.id)}><Pencil size={12} />Rename</button>
      <button class="small" use:tip={"Put this layout back to how it started"} onclick={reset}><RotateCcw size={12} />Reset</button>
    </span>
  </nav>

  <div class="columns" style:grid-template-columns={template}>
    {#each columns as column, c (c)}
      <div class="column">
        {#each column as panel, s (s)}
          {@const info = deskInfo(panel)}
          {@const Icon = info.icon}
          <section class="panel pane" aria-label={info.title}>
            <div class="panel-head">
              <h2><Icon size={14} />{info.title}</h2>
              <button
                class="small change"
                aria-expanded={menu?.column === c && menu?.slot === s}
                onclick={() => (menu = menu?.column === c && menu?.slot === s ? null : { column: c, slot: s })}
                >Change<ChevronDown size={12} /></button
              >
              {#if menu?.column === c && menu?.slot === s}
                <div class="menu" role="menu">
                  <p class="menu-label">Show here</p>
                  {#each DESK_PANELS as option (option)}
                    {@const elsewhere = whereElse(option, c, s)}
                    <button
                      role="menuitemradio"
                      aria-checked={option === panel}
                      class="item"
                      class:current={option === panel}
                      onclick={() => change(place(columns, c, s, option))}
                    >
                      <span>{deskInfo(option).title}</span>
                      {#if option === panel}<span class="note">Here now</span>
                      {:else if elsewhere}<span class="note">{elsewhere}, swaps</span>{/if}
                    </button>
                  {/each}
                  <div class="menu-rule"></div>
                  {#if column.length < 2 && unused(columns).length}
                    <button role="menuitem" class="item" onclick={() => change(split(columns, c))}
                      ><span><Plus size={12} /> Add {deskInfo(unused(columns)[0]).title} below</span></button
                    >
                  {/if}
                  {#if columns.length > 1 || column.length > 1}
                    <button role="menuitem" class="item" onclick={() => change(remove(columns, c, s))}
                      ><span><X size={12} /> Remove {info.title} from this layout</span></button
                    >
                  {/if}
                </div>
              {/if}
            </div>
            <div class="panel-body">
              {#key panel}
                {#if panel === "log"}
                  <TrainingLog embedded />
                {:else if panel === "planner"}
                  <BuildPlanner embedded />
                {:else}
                  <PanelView {panel} analysis={live.analysis} session={live.session} roomy />
                {/if}
              {/key}
            </div>
          </section>
        {/each}
      </div>
    {/each}
  </div>
</div>

<style>
  .layout {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    height: 100%;
    min-height: 0;
  }

  .tabs {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2) var(--space-3);
  }

  .tablist,
  .tab-actions,
  .field {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-1-5);
    min-width: 0;
  }

  .tab-actions {
    gap: var(--space-2) var(--space-3);
  }

  .faint {
    color: var(--faint);
    font-size: var(--text-md);
  }

  .tab,
  .rename {
    height: calc(var(--control-h) - 2px);
    padding: 0 var(--space-4);
    border: var(--line-w) solid var(--control-line);
    border-radius: var(--radius-pill);
    font-size: var(--text-md);
    white-space: nowrap;
  }

  .tab {
    background: var(--surface);
    color: var(--muted);
    font-weight: var(--weight-body);
    cursor: pointer;
  }

  .tab:hover {
    background: var(--surface-2);
    color: var(--text);
  }

  .tab[aria-selected="true"] {
    background: var(--accent);
    color: var(--accent-ink);
    font-weight: var(--weight-strong);
  }

  .rename {
    width: 150px;
    background: var(--surface);
    border-color: var(--accent);
    color: var(--text);
    outline: none;
  }

  button.small {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    height: var(--control-h-sm);
    padding: 0 var(--space-3);
    border: var(--line-w) solid var(--control-line);
    border-radius: var(--radius-pill);
    background: var(--surface);
    color: var(--text);
    font-size: var(--text-xs);
    white-space: nowrap;
    cursor: pointer;
  }

  button.small:hover {
    background: var(--surface-2);
  }

  .columns {
    flex: 1;
    min-height: 0;
    display: grid;
    gap: var(--space-3);
  }

  .column {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    min-width: 0;
    min-height: 0;
  }

  .panel {
    flex: 1 1 0;
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
  }

  .pane {
    border: var(--line-w) solid var(--line);
    border-radius: var(--radius-lg);
    background: color-mix(in srgb, var(--surface) 45%, var(--bg));
    box-shadow: var(--card-shadow);
  }

  .panel-head {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    height: 44px;
    flex: none;
    padding: 0 var(--space-3) 0 var(--space-4);
  }

  h2 {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1-5);
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--text-lg);
    font-weight: var(--weight-strong);
  }

  h2 :global(svg) {
    color: var(--accent-text);
  }

  .menu {
    position: absolute;
    top: 36px;
    right: var(--space-3);
    z-index: 10;
    width: 260px;
    padding: var(--space-1-5);
    background: var(--surface-raised);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: var(--shadow-lg);
  }

  .menu-label {
    margin: 0;
    padding: var(--space-1) var(--space-3);
    font-size: var(--text-xs);
    color: var(--faint);
  }

  .menu-rule {
    margin: var(--space-1-5) 0;
    border-top: 1px solid var(--border);
  }

  .item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    height: 32px;
    padding: 0 var(--space-3);
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text);
    font-size: var(--text-md);
    text-align: left;
    cursor: pointer;
  }

  .item span {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1-5);
  }

  .item:hover {
    background: var(--hover);
  }

  .item.current {
    background: var(--accent-soft);
    color: var(--accent-text-strong);
  }

  .note {
    font-size: var(--text-xs);
    color: var(--faint);
  }

  .item.current .note {
    color: inherit;
  }

  .panel-body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 0 0 var(--space-4);
  }
</style>
