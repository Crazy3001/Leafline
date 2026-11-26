<script lang="ts" module>
  /** A list's words, handed to a search box that stands above it. */
  export interface FoldedField {
    readonly value: string;
    readonly placeholder: string;
    input: (text: string) => void;
  }
  /** Set by the search sheet: its one box types into the list, whose own field folds away. */
  export interface SearchFold {
    attach: (field: FoldedField) => () => void;
  }
  export const SEARCH_FOLD = Symbol("search fold");
</script>

<script lang="ts">
  // The search box each half of the Lookup tab opens with, and its filter and clear buttons.
  // Inside the search sheet the sheet's own box does the typing, so only the buttons stay.
  import { tip } from "$lib/tip.svelte";
  import { getContext } from "svelte";
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

  const fold = getContext<SearchFold | undefined>(SEARCH_FOLD);
  $effect(() => {
    if (!fold) return;
    return fold.attach({
      get value() {
        return value;
      },
      get placeholder() {
        return placeholder;
      },
      input: (text) => oninput(text),
    });
  });
</script>

{#if !fold || filterable || clearable}
<div class="search" class:folded={!!fold}>
  {#if !fold}
    <label class="field">
      <Search size={13} />
      <input type="search" {placeholder} aria-label={label} {value} oninput={(e) => oninput(e.currentTarget.value)} />
    </label>
  {/if}
  {#if filterable}
    <button
      class="tool"
      class:active={filters || narrowed > 0}
      use:tip={"Filters"}
      aria-label="Filters"
      aria-expanded={filters}
      onclick={() => (filters = !filters)}
    >
      <SlidersHorizontal size={13} />{#if fold}<span>Filters</span>{/if}{#if narrowed}<span class="tabular">{narrowed}</span>{/if}
    </button>
  {/if}
  {#if clearable}
    <button class="tool" use:tip={fold ? null : "Clear the search"} aria-label="Clear the search" onclick={onclear}
      ><X size={13} />{#if fold}<span>Clear</span>{/if}</button
    >
  {/if}
</div>
{/if}

<style>
  .search {
    display: flex;
    gap: var(--space-1);
    margin-bottom: var(--space-1-5);
  }

  /* Only the buttons, at the right: the sheet's box above does the typing. */
  .folded {
    justify-content: flex-end;
  }

  .folded .tool {
    gap: var(--space-1);
    padding: 0 var(--space-2);
    border-radius: var(--radius-pill);
  }

  .field {
    display: flex;
    flex: 1;
    align-items: center;
    gap: var(--space-1-5);
    min-width: 0;
    height: 28px;
    padding: 0 var(--space-2);
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
    font-size: var(--text-md);
  }

  .field input::-webkit-search-cancel-button {
    display: none;
  }

  .tool {
    display: inline-flex;
    flex: none;
    align-items: center;
    justify-content: center;
    gap: var(--space-0-5);
    min-width: 28px;
    height: 28px;
    padding: 0 var(--space-1-5);
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
    color: var(--accent-text);
  }
</style>
