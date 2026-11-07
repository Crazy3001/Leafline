<script lang="ts">
  // The training log in a window of its own, opened from the Train tab.
  import TrainingLog from "$lib/components/TrainingLog.svelte";
</script>

<svelte:head><title>Leafline — Training log</title></svelte:head>

<TrainingLog />

<style>
  :global(body) {
    background: var(--bg);
  }
</style>
