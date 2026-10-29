<script lang="ts">
  // A collapsible part of the map card.
  import { ChevronRight } from "@lucide/svelte";
  import type { Snippet } from "svelte";

  let {
    title,
    count,
    open = $bindable(true),
    aside,
    children,
  }: { title: string; count?: number | string; open?: boolean; aside?: Snippet; children: Snippet } = $props();
</script>

<section class="section">
  <button class="head" aria-expanded={open} onclick={() => (open = !open)}>
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
    font-size: 10.5px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--muted);
  }

  .count {
    padding: 0 5px;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.06);
    font-size: 10px;
    color: var(--faint);
  }

  .aside {
    margin-left: auto;
    font-size: 11px;
    color: var(--faint);
  }

  .body {
    margin-top: 8px;
  }
</style>
