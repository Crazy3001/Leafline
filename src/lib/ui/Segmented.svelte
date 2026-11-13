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
    gap: 2px;
    padding: 2px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg);
  }

  .stretch {
    display: flex;
    flex: 1;
  }

  .stretch button {
    flex: 1;
  }

  button {
    height: calc(var(--control-h) - 6px);
    padding: 0 10px;
    border: 0;
    border-radius: 4px;
    background: transparent;
    color: var(--muted);
    font-size: var(--text-sm);
    white-space: nowrap;
    cursor: pointer;
  }

  .sm button {
    height: calc(var(--control-h-sm) - 6px);
    padding: 0 8px;
    font-size: var(--text-xs);
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
    box-shadow:
      inset 0 1px 0 var(--highlight),
      0 1px 3px rgba(0, 0, 0, 0.35);
    color: var(--text);
    font-weight: 600;
  }
</style>
