<script lang="ts">
  // A map's monsters in a row: each one's picture, name and share of the spawns, the most first.
  import Pic from "$lib/ui/Pic.svelte";

  let { mobs }: { mobs: { id: number; name: string; share: number }[] } = $props();
</script>

<span class="chips">
  {#each mobs as mob (mob.id)}
    <span class="chip">
      <Pic kind="mob" id={mob.id} name={mob.name} size={14} />
      <span class="name">{mob.name}</span>
      <span class="share tabular">{Math.round(mob.share * 100)}%</span>
    </span>
  {/each}
</span>

<style>
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 3px;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    max-width: 100%;
    padding: 1px 6px 1px 3px;
    border-radius: var(--radius-pill);
    background: color-mix(in srgb, var(--marker-mob) 12%, transparent);
    color: var(--muted);
    font-size: var(--text-xs);
    line-height: 16px;
  }

  .name {
    overflow: hidden;
    color: var(--text);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .share {
    color: var(--faint);
  }
</style>
