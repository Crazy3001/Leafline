<script lang="ts">
  // The build planner's own window.
  import BuildPlanner from "$lib/components/planner/BuildPlanner.svelte";
</script>

<svelte:head><title>Leafline — Build planner</title></svelte:head>

<BuildPlanner />

<style>
  :global(body) {
    background: var(--bg);
  }
</style>
