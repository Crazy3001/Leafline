<script lang="ts">
  // The website's capture test page (website plan, stage 2). Its code is the website's only:
  // the program's build leaves it out and says where the program's half is.
  import TestShell from "$lib/capture/TestShell.svelte";

  const page = __LEAFLINE_WEB__ ? import("$lib/capture/WebCapture.svelte") : null;
</script>

<TestShell title="Leafline capture test">
  {#if page}
    {#await page then view}<view.default />{/await}
  {:else}
    <h1>Capture test</h1>
    <p class="lead">
      This page is the website's half of the capture test. The program's half is its Capture test window: Settings ›
      Feedback › Capture test, in a dev build.
    </p>
  {/if}
</TestShell>
