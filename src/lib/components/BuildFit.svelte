<script lang="ts">
  // One line under an item: whether the build can wear it, from its base stats and the goal
  // gear it has on by then.
  import { untrack } from "svelte";

  import { backend } from "$lib/api";
  import { buildState, watchBuilds } from "$lib/builds.svelte";
  import { live } from "$lib/readings.svelte";
  import { display } from "$lib/settings.svelte";
  import type { ItemFit, ItemTooltip } from "$lib/types";

  let { item }: { item: ItemTooltip } = $props();

  watchBuilds();

  let fit = $state<ItemFit | null>(null);

  const level = $derived(live.analysis?.hud?.level ?? live.session?.level ?? null);
  const character = $derived(live.analysis?.hud?.name ?? null);
  const key = $derived(
    JSON.stringify([item.requirements, item.jobs, item.itemType, level, character, buildState.version, display.buildRules]),
  );

  // Follows the key alone: every live reading brings new objects with the same content.
  $effect(() => {
    const asked = key;
    untrack(() => backend.buildItemFit(item, level, character))
      .then((result) => {
        if (asked === key) fit = result;
      })
      .catch(() => (fit = null));
  });

  const short = $derived(fit?.short.map((s) => (s.label === "Lv" ? `${s.amount} levels` : `${s.amount} ${s.label}`)).join(", "));
</script>

{#if fit}
  <p
    class="fit"
    class:good={fit.wearableNow && fit.jobOk !== false}
    class:bad={fit.jobOk === false}
  >
    {#if fit.jobOk === false}
      Not for this build's job
    {:else if fit.wearableNow}
      Fits this build now
    {:else if fit.wearableAt != null}
      Fits this build at Lv {fit.wearableAt}, {short} short now
    {:else}
      Out of reach on this build, {short} short now
    {/if}
    {#if fit.unread}<span class="unread">A requirement wasn't read.</span>{/if}
  </p>
{/if}

<style>
  .fit {
    margin: 10px 0 0;
    padding: 5px 8px;
    border-left: 2px solid var(--border-strong);
    background: rgba(255, 255, 255, 0.03);
    color: var(--muted);
    font-size: var(--text-sm);
  }

  .fit.good {
    border-left-color: var(--good);
    color: var(--good);
  }

  .fit.bad {
    border-left-color: var(--warn);
    color: var(--warn);
  }

  .unread {
    margin-left: 4px;
    color: var(--faint);
  }
</style>
