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

<button class="icon-btn {tone}" class:sm={size === "sm"} class:active aria-label={label} aria-pressed={active || undefined} use:tip={title ?? label} {onclick}>
  {@render children()}
</button>

<style>
  /* The look is the global `.icon-btn` in app.css; the header's own colours stay here. */
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
</style>
