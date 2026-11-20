<script lang="ts">
  import "../app.css";
  import { onMount } from "svelte";
  import { page } from "$app/state";
  import { backend } from "$lib/api";
  import PeekHost from "$lib/components/peek/PeekHost.svelte";
  import TipHost from "$lib/ui/TipHost.svelte";
  import { followTheme } from "$lib/ui/theme.svelte";
  import { watchWorn } from "$lib/builds.svelte";
  import { watchMapData } from "$lib/mapdata.svelte";

  let { children } = $props();

  // The website's notice about which tab holds the records. Loaded only in the website's build:
  // it brings in the worker, which the program must not carry.
  const webNotice = __LEAFLINE_WEB__ ? import("$lib/components/WebStorageNotice.svelte") : null;
  // The capture test's pages use no records, and the notice would cover the test pattern.
  const testPage = $derived(page.route.id?.startsWith("/test/") ?? false);

  followTheme();
  watchWorn();
  // The website fetches the community data on a first visit (see mapdata.svelte.ts).
  if (__LEAFLINE_WEB__ && !page.route.id?.startsWith("/test/")) watchMapData();

  // Records replaced from a file, from this window or another: everything shown is out of date.
  onMount(() => {
    const stop = backend.onRecordsReplaced(() => location.reload());
    return () => void stop.then((unlisten) => unlisten());
  });
</script>

{@render children()}
<PeekHost />
{#if webNotice && !testPage}{#await webNotice then notice}<notice.default />{/await}{/if}
<TipHost />
