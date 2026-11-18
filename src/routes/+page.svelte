<script lang="ts">
  // Each of the program's windows opens its own route; the root matters on the website, where it
  // picks the desk view on a computer and the overlay's tabs at phone width, and on the dev server.
  import { goto } from "$app/navigation";
  import { base } from "$app/paths";
  import { onMount } from "svelte";

  import { isWeb } from "$lib/platform";

  onMount(() => goto(isWeb && window.innerWidth >= 760 ? `${base}/desk` : `${base}/overlay`, { replaceState: true }));
</script>
