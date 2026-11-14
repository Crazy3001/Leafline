<script lang="ts">
  import "../app.css";
  import PeekHost from "$lib/components/peek/PeekHost.svelte";
  import TipHost from "$lib/ui/TipHost.svelte";

  let { children } = $props();
</script>

{@render children()}
<PeekHost />
<TipHost />
