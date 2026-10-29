<script lang="ts">
  // Each window opens its own route; the root only matters when browsing the dev server.
  import { goto } from "$app/navigation";
  import { onMount } from "svelte";

  onMount(() => goto("/workbench", { replaceState: true }));
</script>
