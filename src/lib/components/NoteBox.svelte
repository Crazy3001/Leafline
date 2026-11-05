<script lang="ts">
  // A note the player keeps on one thing, such as an item or a map: shown wherever that thing
  // appears, written in place, kept on this PC. What it is a note on is up to the caller.
  import { PenLine, Plus } from "@lucide/svelte";
  import { untrack } from "svelte";

  import { formatDay } from "$lib/format";

  type Note = { note: string; updatedAt: number };

  let {
    key,
    load,
    save: write,
    placeholder,
    title = "Your note",
    changed = 0,
    flush = false,
  }: {
    /** What the note is on; the note is loaded again when it changes. Null shows nothing. */
    key: string | number | null;
    load: () => Promise<Note | null>;
    save: (text: string) => Promise<Note | null>;
    placeholder: string;
    title?: string;
    /** Bumped when the note may have changed in another window. */
    changed?: number;
    /** No rule above it, for a place that already has one. */
    flush?: boolean;
  } = $props();

  let note = $state<Note | null>(null);
  let loaded = $state(false);
  let editing = $state(false);
  let text = $state("");
  let saving = $state(false);
  let error = $state<string | null>(null);
  let box = $state<HTMLTextAreaElement>();

  // Follows the key alone: a live reading of the same thing must not close the note being typed.
  $effect(() => {
    const asked = key;
    void changed;
    if (untrack(() => editing)) return;
    note = null;
    loaded = false;
    error = null;
    if (asked == null) return;
    untrack(() =>
      load()
        .then((found) => {
          if (asked === key) note = found;
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
      note = await write(text);
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

{#if key != null}
  <div class="notes" class:flush>
    <div class="head">
      <span class="eyebrow">{title}</span>
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
          {placeholder}
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

  .flush {
    margin-top: 0;
    padding-top: 0;
    border-top: 0;
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
    background: var(--surface-2);
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
    height: var(--control-h-sm);
    padding: 0 12px;
    border: 1px solid var(--accent);
    border-radius: var(--radius-sm);
    background: var(--accent);
    color: var(--bg);
    font-size: var(--text-sm);
    font-weight: 600;
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
