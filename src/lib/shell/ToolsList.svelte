<script lang="ts" module>
  export type WindowAction = "desk" | "compact" | "frame" | "misread" | "settings" | "hide";
</script>

<script lang="ts">
  // The Tools list: everything that isn't a tab, as a grid of named tiles, and the window's own
  // controls under it, with the light/dark switch. Search finds the same tools by name.
  import { Bug, Camera, EyeOff, Minimize2, MonitorUp, Moon, Settings, Sun } from "@lucide/svelte";

  import { can } from "$lib/platform";
  import { changeSettings, hotkey } from "$lib/settings.svelte";
  import { tip } from "$lib/tip.svelte";
  import { darkLook, lightSwitch } from "$lib/ui/theme.svelte";

  import { TOOLS, type ToolInfo } from "./tabs";

  let {
    canSaveFrame,
    ontool,
    onwindow,
  }: {
    /** Live reading is on, so there is a picture to save. */
    canSaveFrame: boolean;
    ontool: (tool: ToolInfo) => void;
    onwindow: (action: WindowAction) => void;
  } = $props();

  const WINDOW = $derived([
    { id: "desk" as const, title: "Desk view", icon: MonitorUp, key: "" },
    { id: "compact" as const, title: "Compact", icon: Minimize2, key: hotkey("ToggleCompact") },
    // The website can't save a picture by itself; This misread downloads one with what was read.
    ...(canSaveFrame && can.liveReading ? [{ id: "frame" as const, title: "Save frame", icon: Camera, key: hotkey("SaveFrame") }] : []),
    ...(canSaveFrame ? [{ id: "misread" as const, title: "This misread", icon: Bug, key: "" }] : []),
    { id: "settings" as const, title: "Settings", icon: Settings, key: "" },
    { id: "hide" as const, title: "Hide", icon: EyeOff, key: hotkey("ToggleOverlay") },
  ]);
  const flip = $derived(lightSwitch());
</script>

<div class="tools">
  <ul class="grid">
    {#each TOOLS as tool (tool.id)}
      {@const Icon = tool.icon}
      <li>
        <button class="tile" onclick={() => ontool(tool)}>
          <span class="glyph"><Icon size={18} /></span>
          <span class="words">
            <span class="title">{tool.title}</span>
            <span class="line">{tool.line}</span>
          </span>
        </button>
      </li>
    {/each}
  </ul>

  <h3>Leafline</h3>
  <ul class="row">
    {#each WINDOW as w (w.id)}
      {@const Icon = w.icon}
      <li>
        <button class="win" onclick={() => onwindow(w.id)}>
          <Icon size={17} />
          <span>{w.title}</span>
          {#if w.key}<kbd>{w.key}</kbd>{/if}
        </button>
      </li>
    {/each}
    <li>
      <button class="win" aria-label={flip.label} use:tip={flip.label} onclick={() => void changeSettings({ theme: flip.to })}>
        {#if darkLook()}<Sun size={17} />{:else}<Moon size={17} />{/if}
        <span>{darkLook() ? "Light" : "Dark"}</span>
      </button>
    </li>
  </ul>
</div>

<style>
  .tools {
    padding: var(--space-3);
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--space-2);
  }

  .tile {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
    padding: var(--space-2);
    border: var(--line-w) solid var(--line);
    border-radius: var(--radius-lg);
    background: var(--surface);
    box-shadow: var(--card-shadow);
    color: var(--text);
    text-align: left;
    cursor: pointer;
    transition:
      background 0.12s ease,
      transform 0.08s ease;
  }

  .tile:hover {
    background: var(--surface-2);
  }

  .tile:active {
    transform: translateY(1px);
  }

  .tile:focus-visible,
  .win:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  .glyph {
    display: grid;
    flex: none;
    place-items: center;
    width: 36px;
    height: 36px;
    border-radius: var(--radius);
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .words {
    display: grid;
    min-width: 0;
  }

  .title {
    font-weight: var(--weight-strong);
    line-height: 1.25;
  }

  .line {
    overflow: hidden;
    color: var(--faint);
    font-size: var(--text-xs);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  h3 {
    margin: var(--space-4) var(--space-0-5) var(--space-2);
    color: var(--muted);
    font-size: var(--text-xs);
    font-weight: var(--weight-strong);
  }

  .row {
    display: flex;
    gap: var(--space-1-5);
  }

  .row li {
    flex: 1;
    min-width: 0;
  }

  .win {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-1);
    width: 100%;
    padding: var(--space-2) var(--space-0-5) var(--space-1-5);
    border: var(--line-w) solid var(--line);
    border-radius: var(--radius);
    background: var(--surface);
    color: var(--muted);
    font-size: var(--text-xs);
    cursor: pointer;
    transition:
      background 0.12s ease,
      color 0.12s ease;
  }

  .win:hover {
    background: var(--surface-2);
    color: var(--text);
  }

  .win span {
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  kbd {
    color: var(--faint);
    font-family: var(--font-sans);
    font-size: var(--text-2xs);
  }
</style>
