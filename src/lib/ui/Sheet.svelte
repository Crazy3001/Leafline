<script lang="ts">
  // A sheet that rises over the tab you're on: search, an item check, the Tools list, one tool.
  // The tab stays where it was underneath, so closing the sheet puts you back mid-thought. It
  // fills the nearest positioned parent (the window's stage, between the header and the tab bar).
  import { ChevronLeft, X } from "@lucide/svelte";
  import type { Snippet } from "svelte";

  import IconButton from "./IconButton.svelte";

  let {
    title,
    sub,
    onclose,
    onback,
    pad = false,
    actions,
    children,
  }: {
    title: string;
    /** A quiet line beside the title. */
    sub?: string;
    onclose: () => void;
    /** Shows a back arrow, for a sheet opened from another sheet. */
    onback?: () => void;
    /** Pads the body, for content that brings no margins of its own. */
    pad?: boolean;
    /** Extra buttons before the close button. */
    actions?: Snippet;
    children: Snippet;
  } = $props();

  function onkeydown(event: KeyboardEvent) {
    if (event.key !== "Escape" || event.defaultPrevented) return;
    event.preventDefault();
    if (onback) onback();
    else onclose();
  }
</script>

<svelte:window {onkeydown} />

<div class="scrim" aria-hidden="true" onclick={onclose}></div>
<div class="sheet" role="dialog" aria-label={title}>
  <header>
    {#if onback}
      <IconButton label="Back" size="sm" onclick={onback}><ChevronLeft size={17} /></IconButton>
    {/if}
    <h2>{title}</h2>
    {#if sub}<span class="sub">{sub}</span>{/if}
    <span class="grow"></span>
    {@render actions?.()}
    <IconButton label="Close" title="Close (Esc)" size="sm" onclick={onclose}><X size={16} /></IconButton>
  </header>
  <div class="body" class:pad>
    {@render children()}
  </div>
</div>

<style>
  .scrim {
    position: absolute;
    inset: 0;
    z-index: 20;
    background: var(--scrim);
    animation: fade 0.16s ease;
  }

  .sheet {
    position: absolute;
    inset: 6px 0 0;
    z-index: 21;
    display: flex;
    flex-direction: column;
    min-height: 0;
    border: var(--line-w) solid var(--line);
    border-bottom: 0;
    border-radius: var(--radius-lg) var(--radius-lg) 0 0;
    background: var(--bg);
    box-shadow: 0 -8px 28px rgba(0, 0, 0, 0.35);
    animation: rise 0.2s var(--ease);
  }

  header {
    display: flex;
    flex: none;
    align-items: center;
    gap: 6px;
    min-height: 42px;
    padding: 6px 8px 6px 14px;
    border-bottom: 1px solid var(--border);
  }

  h2 {
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--text-lg);
    font-weight: var(--weight-strong);
    line-height: 1.2;
  }

  .sub {
    min-width: 0;
    overflow: hidden;
    color: var(--faint);
    font-size: var(--text-md);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .grow {
    flex: 1;
  }

  .body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }

  .body.pad {
    padding: var(--space-3);
  }

  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(14px);
    }
  }

  @keyframes fade {
    from {
      opacity: 0;
    }
  }
</style>
