<script lang="ts">
  // The website's pop-out (platform/pip.ts): while the game window is being read, the program's
  // compact strip of live numbers; otherwise the session being typed in.
  import { live } from "$lib/readings.svelte";

  import CompactHud from "./CompactHud.svelte";
  import TypedSessionHud from "./TypedSessionHud.svelte";

  const reading = $derived(live.liveStatus.state === "reading" || live.liveStatus.state === "waiting");
</script>

{#if reading}<CompactHud />{:else}<TypedSessionHud />{/if}
