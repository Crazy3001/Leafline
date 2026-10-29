<script lang="ts">
  // A search box over the cached map names.
  import { Search } from "@lucide/svelte";

  import { backend } from "$lib/api";
  import type { MapRef } from "$lib/types";

  let {
    placeholder = "Search maps…",
    label,
    onpick,
  }: { placeholder?: string; label: string; onpick: (map: MapRef) => void } = $props();

  let text = $state("");
  let results = $state<MapRef[]>([]);
  let active = $state(0);
  let open = $state(false);
  let asked = 0;

  async function search() {
    const ask = ++asked;
    const found = text.trim() ? await backend.searchMaps(text) : [];
    if (ask !== asked) return;
    results = found;
    active = 0;
    open = true;
  }

  function pick(map: MapRef) {
    onpick(map);
    text = "";
    results = [];
    open = false;
  }

  function onkeydown(event: KeyboardEvent) {
    if (!results.length) return;
    if (event.key === "ArrowDown") active = (active + 1) % results.length;
    else if (event.key === "ArrowUp") active = (active - 1 + results.length) % results.length;
    else if (event.key === "Enter") pick(results[active]);
    else if (event.key === "Escape") open = false;
    else return;
    event.preventDefault();
  }
</script>

<div class="picker">
  <label class="field">
    <Search size={13} />
    <input
      type="search"
      aria-label={label}
      {placeholder}
      bind:value={text}
      oninput={search}
      {onkeydown}
      onfocus={() => (open = true)}
      onblur={() => setTimeout(() => (open = false), 120)}
    />
  </label>
  {#if open && text.trim()}
    <ul class="results" role="listbox" aria-label={label}>
      {#each results as map, i (map.id)}
        <li role="option" aria-selected={i === active}>
          <button onmousedown={(e) => e.preventDefault()} onclick={() => pick(map)} class:active={i === active}>
            <span class="name">{map.name}</span>
            {#if map.street}<span class="street">{map.street}</span>{/if}
          </button>
        </li>
      {:else}
        <li class="empty">No map by that name.</li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .picker {
    position: relative;
  }

  .field {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 8px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--faint);
  }

  .field:focus-within {
    border-color: color-mix(in srgb, var(--map) 60%, transparent);
  }

  input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: 0;
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: 12px;
  }

  input::-webkit-search-cancel-button {
    filter: invert(0.6);
  }

  .results {
    position: absolute;
    z-index: 5;
    top: calc(100% + 4px);
    left: 0;
    right: 0;
    max-height: 220px;
    margin: 0;
    padding: 4px;
    overflow-y: auto;
    list-style: none;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface-3);
    box-shadow: var(--shadow);
  }

  .results button {
    display: grid;
    width: 100%;
    padding: 5px 8px;
    border: 0;
    border-radius: 4px;
    background: transparent;
    text-align: left;
    cursor: pointer;
  }

  .results button.active,
  .results button:hover {
    background: rgba(255, 255, 255, 0.06);
  }

  .name {
    font-size: 12px;
  }

  .street {
    font-size: 10.5px;
    color: var(--faint);
  }

  .empty {
    padding: 6px 8px;
    font-size: 12px;
    color: var(--faint);
  }
</style>
