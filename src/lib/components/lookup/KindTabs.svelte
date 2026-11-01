<script lang="ts" generics="T">
  // The row of kinds under a search box.
  let {
    label,
    options,
    value,
    onselect,
  }: { label: string; options: { id: T; label: string }[]; value: T; onselect: (id: T) => void } = $props();
</script>

<div class="kinds" role="radiogroup" aria-label={label} style:--count={options.length}>
  {#each options as option (option.label)}
    <button role="radio" aria-checked={value === option.id} onclick={() => onselect(option.id)}>{option.label}</button>
  {/each}
</div>

<style>
  .kinds {
    display: grid;
    grid-template-columns: repeat(var(--count), auto);
    gap: 0;
    margin-bottom: 8px;
    border-bottom: 1px solid var(--border);
  }

  .kinds button {
    padding: 3px 2px 5px;
    border: 0;
    border-bottom: 2px solid transparent;
    background: transparent;
    color: var(--muted);
    font-size: 11.5px;
    cursor: pointer;
  }

  .kinds button:hover,
  .kinds button:focus-visible {
    color: var(--text);
    outline: none;
  }

  .kinds [aria-checked="true"] {
    border-bottom-color: var(--accent);
    color: var(--accent-strong);
  }
</style>
