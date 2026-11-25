<script lang="ts">
  // The one button: its look is the global `.btn` in app.css. Primary is for the single main
  // action on a view; the rest are secondary, ghost (no frame) or danger.
  import type { Snippet } from "svelte";

  import { tip } from "$lib/tip.svelte";

  let {
    variant = "secondary",
    size = "md",
    title,
    disabled = false,
    pressed,
    onclick,
    children,
  }: {
    variant?: "primary" | "secondary" | "ghost" | "danger";
    size?: "md" | "sm";
    title?: string;
    disabled?: boolean;
    pressed?: boolean;
    onclick?: (event: MouseEvent) => void;
    children: Snippet;
  } = $props();
</script>

<button class="btn {variant} {size === 'sm' ? 'sm' : ''}" use:tip={title} {disabled} aria-pressed={pressed} {onclick}>{@render children()}</button>
