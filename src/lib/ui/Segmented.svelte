<script lang="ts" generics="T extends string | number">
  // A row of choices where exactly one is picked: the one control for modes and sorts.
  let {
    options,
    value,
    label,
    size = "md",
    stretch = false,
    onchange,
  }: {
    options: { id: T; label: string; title?: string }[];
    value: T;
    /** What the choice is about, for screen readers. */
    label: string;
    size?: "md" | "sm";
    /** Fill the width, each choice the same share. */
    stretch?: boolean;
    onchange: (value: T) => void;
  } = $props();
</script>

<div class="seg {size}" class:stretch role="group" aria-label={label}>
  {#each options as option (option.id)}
    <button aria-pressed={option.id === value} title={option.title} onclick={() => onchange(option.id)}>
      {option.label}
    </button>
  {/each}
</div>

<style>
  .seg {
    display: inline-flex;
    flex: none;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }

  .stretch {
    display: flex;
    flex: 1;
  }

  .stretch button {
    flex: 1;
  }

  button {
    height: calc(var(--control-h) - 2px);
    padding: 0 10px;
    border: 0;
    background: transparent;
    color: var(--muted);
    font-size: var(--text-sm);
    white-space: nowrap;
    cursor: pointer;
  }

  .sm button {
    height: calc(var(--control-h-sm) - 2px);
    padding: 0 8px;
    font-size: var(--text-xs);
  }

  button + button {
    border-left: 1px solid var(--border-strong);
  }

  button:hover {
    color: var(--text);
  }

  button:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }

  button[aria-pressed="true"] {
    background: var(--surface-3);
    color: var(--text);
    font-weight: 600;
  }
</style>
