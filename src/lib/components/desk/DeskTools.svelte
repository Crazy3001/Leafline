<script lang="ts">
  // The desk view's Tools page: everything that isn't one of the five tabs or Look up, as a grid
  // of named tiles. The sidebar has one Tools entry instead of a list of them; search finds the
  // same tools by name.
  import { TOOLS, type ToolInfo } from "$lib/shell/tabs";
  import { DESK_TOOL_PAGES } from "$lib/desk";

  let { ontool }: { ontool: (tool: ToolInfo) => void } = $props();

  /** The tools the desk view can open: the character switch lives on its pages already. */
  const shown = TOOLS.filter((t) => !("page" in t.action) || (DESK_TOOL_PAGES as string[]).includes(t.action.page));
</script>

<section class="tools" aria-label="Tools">
  <h2 class="title">Tools</h2>
  <ul class="grid">
    {#each shown as tool (tool.id)}
      {@const Icon = tool.icon}
      <li>
        <button class="tile" onclick={() => ontool(tool)}>
          <span class="glyph"><Icon size={18} /></span>
          <span class="words">
            <span class="name">{tool.title}</span>
            <span class="line">{tool.line}</span>
          </span>
        </button>
      </li>
    {/each}
  </ul>
</section>

<style>
  .tools {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .title {
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--text-xl);
    font-weight: var(--weight-strong);
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: var(--space-2);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .tile {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    width: 100%;
    padding: var(--space-3);
    border: var(--line-w) solid var(--line);
    border-radius: var(--radius);
    background: var(--surface);
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .tile:hover {
    background: var(--surface-2);
  }

  .tile:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  .glyph {
    display: grid;
    flex: none;
    place-items: center;
    width: 36px;
    height: 36px;
    border-radius: var(--radius-sm);
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .words {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .name {
    font-weight: var(--weight-strong);
  }

  .line {
    overflow: hidden;
    color: var(--muted);
    font-size: var(--text-xs);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
