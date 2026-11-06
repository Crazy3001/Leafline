<script lang="ts">
  // The finer groups under a kind, such as equipment slots or areas, to narrow a list to. Choosing
  // the one that's on goes back to all of them.
  let {
    label,
    caption = null,
    options,
    value,
    onselect,
  }: {
    label: string;
    /** A word shown before the chips, for a row that would otherwise look like the one above. */
    caption?: string | null;
    options: string[];
    value: string | null;
    onselect: (option: string | null) => void;
  } = $props();
</script>

{#if options.length}
  <div class="chips" role="radiogroup" aria-label={label}>
    {#if caption}<span class="caption" aria-hidden="true">{caption}</span>{/if}
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

  .caption {
    align-self: center;
    margin-right: 3px;
    color: var(--faint);
    font-size: var(--text-xs);
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
