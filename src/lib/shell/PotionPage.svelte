<script lang="ts">
  // Tools → Potion costs: the running session's potions open, with what they cost and mesos per
  // hour once they're taken out. The same card sits folded in the Train tab.
  import PotionCosts from "$lib/components/PotionCosts.svelte";
  import { characters } from "$lib/characters.svelte";
  import { lootPerHour } from "$lib/loot.svelte";
  import { live } from "$lib/readings.svelte";

  const session = $derived(live.session && live.session.activeSeconds > 0 ? live.session : null);
  const picked = $derived(session?.loot?.watched ? (session.loot.mesosPerHour ?? null) : null);
</script>

<section class="card page">
  {#if session}
    <PotionCosts
      startedAt={session.startedAt}
      activeSeconds={session.activeSeconds}
      mesosPerHour={session.mesosPerHour ?? picked}
      character={session.name ?? characters.active}
      quickSlots={session.quickSlots ?? null}
      lootPerHour={session.loot?.watched ? lootPerHour(session.loot.items, session.loot.seconds) : null}
      collapsible={false}
    />
  {:else}
    <p class="quiet">
      Potion costs are counted for a session: how many of each went and what that took off your mesos per hour. They show
      here, and folded in the Train tab, once a session starts with your first EXP.
    </p>
  {/if}
</section>

<style>
  .page {
    margin: 0 10px 10px;
    padding: 12px 14px 14px;
  }

  .quiet {
    margin: 0;
    color: var(--muted);
    font-size: var(--text-sm);
  }
</style>
