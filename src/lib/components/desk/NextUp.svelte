<script lang="ts">
  // Next up: a short list of what's worth doing now, the best first, each with one button (see
  // nextup.svelte.ts for what goes in it). Suggestions to act on, never alerts.
  import { Footprints, Gift, Plus, Sparkles } from "@lucide/svelte";

  import GoButton from "$lib/components/GoButton.svelte";
  import Ref from "$lib/components/Ref.svelte";
  import { nextSteps, watchNextUp } from "$lib/nextup.svelte";
  import TitledCard from "$lib/ui/TitledCard.svelte";

  let { onme }: { onme: () => void } = $props();

  watchNextUp();
  const steps = $derived(nextSteps());

  const ICONS = { handin: Gift, ap: Plus, hunt: Footprints };
</script>

<TitledCard title="Next up" icon={Sparkles} tone="leaf">
  {#snippet aside()}best first{/snippet}
  {#if steps.length}
    <ol class="steps">
      {#each steps as s, i (s.key)}
        {@const Icon = ICONS[s.kind]}
        <li class="step {s.kind}" class:top={i === 0}>
          <span class="ic" aria-hidden="true"><Icon size={18} /></span>
          <span class="t">{s.title}</span>
          <span class="d">
            {#if s.kind === "handin"}
              {#if s.npc}To <Ref kind="npc" name={s.npc} />{#if s.map}{" "}· <Ref kind="map" id={s.map.id} name={s.map.name} />{/if}{:else}Ready to hand in{/if}
            {:else if s.kind === "hunt" && s.map}
              Most on <Ref kind="map" id={s.map.id} name={s.map.name} />{#if s.detail}{" "}· {s.detail}{/if}
            {:else}
              {s.detail}
            {/if}
          </span>
          <span class="act">
            {#if s.kind === "ap"}
              <button class="btn sm" onclick={onme}>Build</button>
            {:else if s.place}
              <GoButton place={s.place} />
            {/if}
          </span>
        </li>
      {/each}
    </ol>
  {:else}
    <p class="empty">Nothing waiting. Quests ready to hand in, AP to spend and where to hunt show here as they come up.</p>
  {/if}
</TitledCard>

<style>
  .steps {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .step {
    display: grid;
    grid-template-columns: 38px minmax(0, 1fr) auto;
    column-gap: var(--space-3);
    row-gap: var(--space-0-5);
    align-items: center;
    padding: var(--space-3);
    border-radius: var(--radius);
    background: var(--surface-2);
  }

  .step.top {
    background: color-mix(in srgb, var(--exp) 15%, var(--surface-2));
  }

  .ic {
    display: grid;
    grid-row: span 2;
    place-items: center;
    width: 38px;
    height: 38px;
    border-radius: var(--radius);
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .handin .ic {
    background: color-mix(in srgb, var(--exp) 22%, transparent);
    color: var(--exp-text);
  }

  .hunt .ic {
    background: color-mix(in srgb, var(--ref-mob) 18%, transparent);
    color: var(--ref-mob);
  }

  .t {
    overflow: hidden;
    font-weight: var(--weight-strong);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .d {
    grid-column: 2;
    min-width: 0;
    overflow: hidden;
    color: var(--muted);
    font-size: var(--text-xs);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .act {
    grid-column: 3;
    grid-row: 1 / 3;
  }

  .empty {
    margin: 0;
    color: var(--muted);
  }
</style>
