<script lang="ts">
  // A collapsible part of the map card. Opening or closing one is remembered by its title;
  // until then `open` decides.
  import { ChevronRight } from "@lucide/svelte";
  import type { Snippet } from "svelte";

  let {
    title,
    count,
    open: startOpen = true,
    aside,
    children,
  }: { title: string; count?: number | string; open?: boolean; aside?: Snippet; children: Snippet } = $props();

  function remembered() {
    try {
      const value = localStorage.getItem(`leafline.section.${title}`);
      return value === null ? null : value === "true";
    } catch {
      return null;
    }
  }

  // svelte-ignore state_referenced_locally
  let open = $state(remembered() ?? startOpen);

  function toggle() {
    open = !open;
    try {
      localStorage.setItem(`leafline.section.${title}`, String(open));
    } catch {
      // A convenience only.
    }
  }
</script>

<section class="section">
  <button class="head" aria-expanded={open} onclick={toggle}>
    <span class="chevron" class:open><ChevronRight size={12} /></span>
    <span class="title">{title}</span>
    {#if count != null}<span class="count tabular">{count}</span>{/if}
    {#if aside}<span class="aside">{@render aside()}</span>{/if}
  </button>
  {#if open}
    <div class="body">{@render children()}</div>
  {/if}
</section>

<style>
  .section {
    border-top: 1px solid var(--border);
    padding-top: 8px;
    margin-top: 10px;
  }

  .head {
    display: flex;
    align-items: center;
    gap: 5px;
    width: 100%;
    padding: 0;
    border: 0;
    background: transparent;
    cursor: pointer;
    text-align: left;
  }

  .chevron {
    display: grid;
    color: var(--faint);
    transition: transform 0.15s ease;
  }

  .chevron.open {
    transform: rotate(90deg);
  }

  .title {
    font-family: var(--font-display);
    font-size: var(--text-md);
    font-weight: 400;
    color: var(--muted);
  }

  .count {
    padding: 0 5px;
    border-radius: var(--radius-sm);
    background: rgba(255, 255, 255, 0.06);
    font-size: var(--text-xs);
    color: var(--faint);
  }

  .aside {
    margin-left: auto;
    font-size: var(--text-xs);
    color: var(--faint);
  }

  .body {
    margin-top: 8px;
  }
</style>
