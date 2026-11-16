<script lang="ts">
  // A square button that is only an icon: the header's actions, a sheet's close. It always has a
  // name for screen readers and a tooltip saying what it does.
  import type { Snippet } from "svelte";

  import { tip, type Tip } from "$lib/tip.svelte";

  let {
    label,
    title,
    active = false,
    tone = "plain",
    size = "md",
    onclick,
    children,
  }: {
    /** What the button does, for screen readers. */
    label: string;
    /** The tooltip; the label when left out. */
    title?: Tip;
    /** Lit, for a switch that is on (locked, pinned). */
    active?: boolean;
    /** `header` takes the header's colours, which differ from the body's in Ember Light. */
    tone?: "plain" | "header";
    size?: "md" | "sm";
    onclick?: (event: MouseEvent) => void;
    children: Snippet;
  } = $props();
</script>

<button class="ib {tone} {size}" class:active aria-label={label} aria-pressed={active || undefined} use:tip={title ?? label} {onclick}>
  {@render children()}
</button>

<style>
  .ib {
    display: grid;
    flex: none;
    place-items: center;
    width: 34px;
    height: 34px;
    padding: 0;
    border: 0;
    border-radius: var(--radius);
    background: transparent;
    color: var(--muted);
    cursor: pointer;
    transition:
      background 0.12s ease,
      color 0.12s ease;
  }

  .sm {
    width: 28px;
    height: 28px;
    border-radius: var(--radius-sm);
  }

  .ib:hover {
    background: var(--hover);
    color: var(--text);
  }

  .ib.active {
    color: var(--accent-text);
  }

  .header {
    color: var(--header-muted);
  }

  .header:hover {
    background: var(--header-hover);
    color: var(--header-text);
  }

  .header.active {
    color: var(--header-accent);
  }

  .ib:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }
</style>
