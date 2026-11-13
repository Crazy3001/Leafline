<script lang="ts">
  import "../app.css";
  import PeekHost from "$lib/components/peek/PeekHost.svelte";

  let { children } = $props();
</script>

{@render children()}
<PeekHost />
