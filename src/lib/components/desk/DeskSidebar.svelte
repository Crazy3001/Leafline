<script lang="ts">
  // The desk view's sidebar: the light/dark switch beside the fold button, the character switch
  // (whose numbers every page shows) under the logo, then the five tabs, Look up, Tools (one page
  // of every tool) and the player's own layout. Whether Leafline is reading is said by the player
  // bar over the views (on the website, by the Share button), as is how the session goes; the
  // data's credits are in Settings. Its foot keeps the ban-risk line. It folds to icons in a
  // narrow window.
  import { tip } from "$lib/tip.svelte";
  import {
    ChartLine,
    Download,
    House,
    LayoutGrid,
    ListChecks,
    Map as MapIcon,
    PanelLeftClose,
    PanelLeftOpen,
    Moon,
    Search,
    Settings,
    Shapes,
    Sun,
    Type,
    UserRound,
  } from "@lucide/svelte";

  import { backend } from "$lib/api";
  import Logo from "$lib/components/Logo.svelte";
  import CharacterSwitch from "./CharacterSwitch.svelte";
  import { DESK_TOOL_PAGES, ZOOMS, type DeskView } from "$lib/desk";
  import { isWeb } from "$lib/platform";
  import { noteRecordsSaved, recordsSavedAt } from "$lib/recordsfile";
  import { live } from "$lib/readings.svelte";
  import { changeSettings } from "$lib/settings.svelte";
  import { darkLook, lightSwitch } from "$lib/ui/theme.svelte";

  /** What the ban risk is, in the README's FAQ. */
  const RISK_URL = "https://github.com/Crazy3001/Leafline#is-it-allowed-can-i-get-banned";

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
    { id: "train", title: "Train", icon: ChartLine },
    { id: "quests", title: "Quests", icon: ListChecks },
    { id: "map", title: "Map", icon: MapIcon },
    { id: "me", title: "Me", icon: UserRound },
  ];
  const MORE: { id: DeskView; title: string; icon: typeof House }[] = [
    { id: "lookup", title: "Look up", icon: Search },
    { id: "tools", title: "Tools", icon: Shapes },
    { id: "layout", title: "My layout", icon: LayoutGrid },
  ];

  /** A tool's page or the build planner is under Tools. */
  const current = $derived<DeskView>(view === "planner" || (DESK_TOOL_PAGES as string[]).includes(view) ? "tools" : view);

  const dark = $derived(darkLook());
  const flip = $derived(lightSwitch());

  // The website keeps the records in this browser, which can clear them: saving a records file is
  // one click away, and goes quiet once one was saved in the last two weeks.
  let savedAt = $state(isWeb ? recordsSavedAt() : null);
  const saveDue = $derived(isWeb && (savedAt == null || Date.now() - savedAt > 14 * 86400_000));
  const saveTip = $derived(
    savedAt
      ? `Your records live in this browser. Last saved to a file on ${new Date(savedAt).toLocaleDateString()}.`
      : "Your records live in this browser, which can clear them. Save a copy to keep them safe.",
  );
  async function saveFile() {
    try {
      if (await backend.saveRecords()) {
        noteRecordsSaved();
        savedAt = recordsSavedAt();
      }
    } catch {
      // Settings → Data says what went wrong when tried there.
    }
  }

  /** The next text size, round and round. */
  const nextZoom = $derived(ZOOMS[(ZOOMS.indexOf(zoom) + 1) % ZOOMS.length]);

</script>

{#snippet item(entry: { id: DeskView; title: string; icon: typeof House }, count?: string | null, key?: string)}
  {@const Icon = entry.icon}
  <button
    class="nav"
    class:on={current === entry.id}
    aria-current={current === entry.id ? "page" : undefined}
    aria-label={folded ? entry.title : undefined}
    use:tip={folded ? entry.title : undefined}
    onclick={() => onview(entry.id)}
  >
    <Icon size={16} />
    {#if !folded}<span class="label">{entry.title}</span>{#if count}<span class="count num">{count}</span>{/if}{#if key}<kbd class="key" aria-hidden="true">{key}</kbd>{/if}{/if}
  </button>
{/snippet}

<aside class:folded aria-label="Desk view">
  <div class="brand">
    {#if !folded}<Logo size={18} /><span class="word">Leafline</span>{/if}
    <button class="icon-btn sm look" aria-label={flip.label} use:tip={flip.label} onclick={() => void changeSettings({ theme: flip.to })}>
      {#if dark}<Sun size={16} />{:else}<Moon size={16} />{/if}
    </button>
    <button class="icon-btn sm fold" aria-label={folded ? "Show the sidebar" : "Fold the sidebar"} use:tip={folded ? "Show the sidebar" : "Fold the sidebar"} onclick={onfold}>
      {#if folded}<PanelLeftOpen size={16} />{:else}<PanelLeftClose size={16} />{/if}
    </button>
  </div>

  <CharacterSwitch {folded} onunfold={onfold} />

  <nav aria-label="Views">
    {#each VIEWS as v, i (v.id)}
      {@render item(v, v.id === "quests" ? String((live.analysis?.quest?.quests ?? []).length || "") || null : null, String(i + 1))}
    {/each}
    <span class="rule"></span>
    {#each MORE as v (v.id)}
      {@render item(v)}
    {/each}
  </nav>

  <div class="foot">
    {#if isWeb}
      <button class="nav" class:due={saveDue} aria-label={folded ? "Save records file" : undefined} use:tip={saveTip} onclick={saveFile}>
        <Download size={16} />{#if !folded}<span class="label">Save records file</span>{/if}
      </button>
    {/if}
    <div class="foot-row">
      <button class="nav" aria-label={folded ? "Settings" : undefined} use:tip={folded ? "Settings" : undefined} onclick={() => backend.showSettings()}>
        <Settings size={16} />{#if !folded}<span class="label">Settings</span>{/if}
      </button>
      {#if !folded}
        <button class="icon-btn sm size" use:tip={`Text size ${Math.round(zoom * 100)}%. Click for ${Math.round(nextZoom * 100)}%`} aria-label="Text size {Math.round(zoom * 100)}%" onclick={() => onzoom(nextZoom)}>
          <Type size={14} /><span class="num">{Math.round(zoom * 100)}</span>
        </button>
      {/if}
    </div>
    <!-- The ban-risk line, kept in view on every page (see README's FAQ). -->
    {#if !folded}<p class="credits">
        Unofficial, and Nexon hasn't approved overlays. Use at your own risk.
        <button class="risk" onclick={() => backend.openLink(RISK_URL)}>Details</button>
      </p>{/if}
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

  /* Folded, the sun/moon and the fold button stand one over the other. */
  .folded .brand {
    flex-direction: column;
    gap: var(--space-1);
    padding: var(--space-0-5) 0 var(--space-2);
  }

  .word {
    font-family: var(--font-display);
    font-size: var(--text-lg);
    font-weight: var(--weight-strong);
  }

  .look {
    margin-left: auto;
  }

  .folded .look {
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

  /* No records file saved in two weeks. */
  .nav.due {
    color: var(--accent-text-strong);
  }

  .nav.on {
    background: var(--accent-soft);
    color: var(--accent-text-strong);
  }

  .nav:focus-visible {
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


  .foot {
    display: grid;
    gap: var(--space-1);
    width: 100%;
    margin-top: auto;
    padding-top: var(--space-2);
  }

  .key {
    margin-left: auto;
    min-width: 20px;
    padding: 0 var(--space-1);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    color: var(--faint);
    font-family: var(--font-sans);
    font-size: var(--text-2xs);
    line-height: 18px;
    text-align: center;
  }

  .count + .key {
    margin-left: var(--space-1-5);
  }

  .foot-row {
    display: flex;
    align-items: center;
    gap: var(--space-1);
  }

  .foot-row .nav {
    flex: 1;
  }

  .size {
    width: auto;
    gap: var(--space-1);
    padding: 0 var(--space-1-5);
    color: var(--faint);
    font-size: var(--text-2xs);
  }


  .risk {
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--muted);
    font: inherit;
    text-decoration: underline;
    text-decoration-color: var(--border-strong);
    cursor: pointer;
  }

  .risk:hover {
    color: var(--text);
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
