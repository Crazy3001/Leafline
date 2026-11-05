<script lang="ts">
  // The search box each half of the Lookup tab opens with, and its filter and clear buttons.
  import { Search, SlidersHorizontal, X } from "@lucide/svelte";

  let {
    value,
    placeholder,
    label,
    narrowed = 0,
    filters = $bindable(false),
    filterable = true,
    clearable = false,
    oninput,
    onclear,
  }: {
    value: string;
    placeholder: string;
    label: string;
    /** How many filters are set. */
    narrowed?: number;
    /** Whether the filters are showing. */
    filters?: boolean;
    filterable?: boolean;
    clearable?: boolean;
    oninput: (text: string) => void;
    onclear: () => void;
  } = $props();
</script>

<div class="search">
  <label class="field">
    <Search size={13} />
    <input type="search" {placeholder} aria-label={label} {value} oninput={(e) => oninput(e.currentTarget.value)} />
  </label>
  {#if filterable}
    <button
      class="tool"
      class:active={filters || narrowed > 0}
      title="Filters"
      aria-label="Filters"
      aria-expanded={filters}
      onclick={() => (filters = !filters)}
    >
      <SlidersHorizontal size={13} />{#if narrowed}<span class="tabular">{narrowed}</span>{/if}
    </button>
  {/if}
  {#if clearable}
    <button class="tool" title="Clear the search" aria-label="Clear the search" onclick={onclear}><X size={13} /></button>
  {/if}
</div>

<style>
  .search {
    display: flex;
    gap: 4px;
    margin-bottom: 6px;
  }

  .field {
    display: flex;
    flex: 1;
    align-items: center;
    gap: 6px;
    min-width: 0;
    height: 28px;
    padding: 0 8px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--faint);
  }

  .field:focus-within {
    border-color: color-mix(in srgb, var(--accent) 60%, transparent);
  }

  .field input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: 0;
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
  }

  .field input::-webkit-search-cancel-button {
    display: none;
  }

  .tool {
    display: inline-flex;
    flex: none;
    align-items: center;
    justify-content: center;
    gap: 3px;
    min-width: 28px;
    height: 28px;
    padding: 0 6px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--muted);
    font-size: var(--text-xs);
    cursor: pointer;
  }

  .tool:hover,
  .tool:focus-visible {
    color: var(--text);
    outline: none;
  }

  .tool.active {
    border-color: color-mix(in srgb, var(--accent) 45%, transparent);
    color: var(--accent);
  }
</style>
