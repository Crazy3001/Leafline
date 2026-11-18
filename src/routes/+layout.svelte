<script lang="ts">
  import "../app.css";
  import { onMount } from "svelte";
  import { backend } from "$lib/api";
  import PeekHost from "$lib/components/peek/PeekHost.svelte";
  import TipHost from "$lib/ui/TipHost.svelte";
  import WebStorageNotice from "$lib/components/WebStorageNotice.svelte";
  import { isWeb } from "$lib/platform";
  import { followTheme } from "$lib/ui/theme.svelte";
  import { watchWorn } from "$lib/builds.svelte";

  let { children } = $props();

  followTheme();
  watchWorn();

  // Records replaced from a file, from this window or another: everything shown is out of date.
  onMount(() => {
    const stop = backend.onRecordsReplaced(() => location.reload());
    return () => void stop.then((unlisten) => unlisten());
  });
</script>

{@render children()}
<PeekHost />
{#if isWeb}<WebStorageNotice />{/if}
<TipHost />
