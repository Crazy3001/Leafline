<script lang="ts">
  import "../app.css";
  import PeekHost from "$lib/components/peek/PeekHost.svelte";
  import TipHost from "$lib/ui/TipHost.svelte";
  import { followTheme } from "$lib/ui/theme.svelte";
  import { watchWorn } from "$lib/builds.svelte";

  let { children } = $props();

  followTheme();
  watchWorn();
</script>

{@render children()}
<PeekHost />
<TipHost />
