<script lang="ts">
  // The player's own note on an item: what they want to remember about it, kept on this PC and
  // shown whenever the item is checked or looked up again. One note per item name.
  import { PenLine, Plus } from "@lucide/svelte";
  import { untrack } from "svelte";

  import { backend } from "$lib/api";
  import { formatDay } from "$lib/format";
  import type { ItemNote } from "$lib/types";

  let { name }: { name: string } = $props();

  let note = $state<ItemNote | null>(null);
  let loaded = $state(false);
  let editing = $state(false);
  let text = $state("");
  let saving = $state(false);
  let error = $state<string | null>(null);
  let box = $state<HTMLTextAreaElement>();

  // Follows the name alone: a live reading of the same item must not close the note being typed.
  $effect(() => {
    const asked = name;
    note = null;
    loaded = false;
    editing = false;
    error = null;
    if (!asked) return;
    untrack(() =>
      backend
        .itemNote(asked)
        .then((found) => {
          if (asked === name) note = found;
        })
        .catch((e) => (error = String(e)))
        .finally(() => (loaded = true)),
    );
  });

  $effect(() => {
    if (editing) box?.focus();
  });

  function edit() {
    text = note?.note ?? "";
    error = null;
    editing = true;
  }

  async function save(event?: SubmitEvent) {
    event?.preventDefault();
    saving = true;
    error = null;
    try {
      note = await backend.setItemNote(name, text);
      editing = false;
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }

  function onKey(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.stopPropagation();
      editing = false;
    } else if (event.key === "Enter" && (event.ctrlKey || event.metaKey)) {
      event.preventDefault();
      save();
    }
  }
</script>

{#if name}
  <div class="notes">
    <div class="head">
      <span class="eyebrow">Your note</span>
      {#if loaded && !editing}
        <button class="add" onclick={edit}>
          {#if note}<PenLine size={12} /> Edit{:else}<Plus size={12} /> Add a note{/if}
        </button>
      {/if}
    </div>

    {#if editing}
      <form onsubmit={save}>
        <textarea
          bind:this={box}
          bind:value={text}
          onkeydown={onKey}
          rows="3"
          maxlength="500"
          placeholder="What to remember about it: keep, sell, scroll it, who wants one"
        ></textarea>
        <div class="actions">
          <span class="keys">Ctrl+Enter saves · Esc cancels{note ? " · empty deletes" : ""}</span>
          <button type="button" class="link" onclick={() => (editing = false)}>Cancel</button>
          <button type="submit" class="save" disabled={saving}>Save</button>
        </div>
      </form>
    {:else if note}
      <p class="text">{note.note}</p>
      <p class="when tabular">{formatDay(note.updatedAt)}</p>
    {/if}
    {#if error}<p class="error">{error}</p>{/if}
  </div>
{/if}

<style>
  .notes {
    margin-top: 12px;
    padding-top: 10px;
    border-top: 1px solid var(--border);
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 4px;
  }

  .add {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 8px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--muted);
    font-size: var(--text-sm);
    cursor: pointer;
  }

  .add:hover,
  .add:focus-visible {
    border-color: var(--accent);
    color: var(--text);
    outline: none;
  }

  .text {
    margin: 0;
    font-size: var(--text-md);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .when {
    margin: 2px 0 0;
    color: var(--faint);
    font-size: var(--text-xs);
  }

  textarea {
    display: block;
    width: 100%;
    padding: 6px 8px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: rgba(0, 0, 0, 0.25);
    color: var(--text);
    font: inherit;
    font-size: var(--text-md);
    resize: vertical;
  }

  textarea:focus {
    border-color: var(--accent);
    outline: none;
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: 6px;
  }

  .keys {
    flex: 1;
    color: var(--faint);
    font-size: var(--text-xs);
  }

  .link {
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--muted);
    font-size: var(--text-sm);
    cursor: pointer;
  }

  .link:hover {
    color: var(--text);
  }

  .save {
    padding: 3px 12px;
    border: 1px solid color-mix(in srgb, var(--accent) 45%, transparent);
    border-radius: var(--radius-sm);
    background: var(--accent-soft);
    color: var(--accent-strong);
    font-size: var(--text-sm);
    cursor: pointer;
  }

  .save:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .error {
    margin: 6px 0 0;
    color: var(--bad);
    font-size: var(--text-sm);
  }
</style>
