<script lang="ts">
  // Equipment slots to narrow a list to. Choosing the one that's on goes back to all of them.
  let {
    slots,
    value,
    onselect,
  }: { slots: string[]; value: string | null; onselect: (slot: string | null) => void } = $props();
</script>

{#if slots.length}
  <div class="slots" role="radiogroup" aria-label="Slot">
    <button role="radio" aria-checked={value == null} onclick={() => onselect(null)}>All</button>
    {#each slots as slot (slot)}
      <button role="radio" aria-checked={value === slot} onclick={() => onselect(value === slot ? null : slot)}>{slot}</button>
    {/each}
  </div>
{/if}

<style>
  .slots {
    display: flex;
    flex-wrap: wrap;
    gap: 3px;
    margin-bottom: 8px;
  }

  button {
    height: 20px;
    padding: 0 6px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--muted);
    font-size: 11px;
    cursor: pointer;
  }

  button:hover,
  button:focus-visible {
    color: var(--text);
    outline: none;
  }

  [aria-checked="true"] {
    border-color: color-mix(in srgb, var(--accent) 45%, transparent);
    background: var(--accent-soft);
    color: var(--accent-strong);
  }
</style>
