<script lang="ts">
  // The finer groups under a kind, such as equipment slots or areas, to narrow a list to. Choosing
  // the one that's on goes back to all of them.
  let {
    label,
    options,
    value,
    onselect,
  }: { label: string; options: string[]; value: string | null; onselect: (option: string | null) => void } = $props();
</script>

{#if options.length}
  <div class="chips" role="radiogroup" aria-label={label}>
    <button role="radio" aria-checked={value == null} onclick={() => onselect(null)}>All</button>
    {#each options as option (option)}
      <button role="radio" aria-checked={value === option} onclick={() => onselect(value === option ? null : option)}>{option}</button>
    {/each}
  </div>
{/if}

<style>
  .chips {
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
    font-size: var(--text-xs);
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
