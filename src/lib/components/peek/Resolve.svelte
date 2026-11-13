<script lang="ts" generics="T">
  // Finds what a link card shows, by id or by name, and says so plainly when the data has
  // nothing by that name or isn't downloaded yet.
  import { LoaderCircle } from "@lucide/svelte";
  import type { Snippet } from "svelte";

  import { mapData, watchMapData } from "$lib/mapdata.svelte";

  let {
    find,
    what,
    name,
    children,
  }: {
    find: () => Promise<T | null>;
    /** "item", "monster": for the line saying nothing was found. */
    what: string;
    name: string | null;
    children: Snippet<[T]>;
  } = $props();

  watchMapData();
  const ready = $derived(mapData.status?.ready ?? false);

  let found = $state<T | null>(null);
  let done = $state(false);
  let error = $state<string | null>(null);

  $effect(() => {
    if (!ready) return;
    find()
      .then((result) => (found = result))
      .catch((e) => (error = String(e)))
      .finally(() => (done = true));
  });
</script>

{#if found}
  {@render children(found)}
{:else if mapData.status && !ready}
  <p class="note">{name ? `${name}: the` : "The"} map data isn't downloaded yet. Download it in Settings to see more.</p>
{:else if error}
  <p class="note bad">{error}</p>
{:else if done}
  <p class="note">No {what} {name ? `called “${name}”` : "like that"} in the data.</p>
{:else}
  <p class="note loading"><LoaderCircle size={13} />Looking it up…</p>
{/if}

<style>
  .note {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0;
    color: var(--muted);
    font-size: var(--text-sm);
  }

  .bad {
    color: var(--bad);
  }

  .loading :global(svg) {
    animation: spin 0.9s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
