<script lang="ts">
  import "../app.css";
  import PeekHost from "$lib/components/peek/PeekHost.svelte";
  import TipHost from "$lib/ui/TipHost.svelte";
  import { followTheme } from "$lib/ui/theme.svelte";

  let { children } = $props();

  followTheme();
</script>

{@render children()}
<PeekHost />
<TipHost />
