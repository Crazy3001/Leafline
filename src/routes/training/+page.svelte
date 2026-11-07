<script lang="ts">
  // The training log in a window of its own, opened from the Train tab.
  import { onMount } from "svelte";

  import TrainingLog from "$lib/components/TrainingLog.svelte";
  import { watchTyping } from "$lib/typing";

  // Clicks leave the game focused; typing in a box takes focus while it lasts.
  onMount(() => watchTyping());
</script>

<svelte:head><title>Leafline — Training log</title></svelte:head>

<TrainingLog />

<style>
  :global(body) {
    background: var(--bg);
  }
</style>
