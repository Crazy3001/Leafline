<script lang="ts">
  // An on/off setting: a rounded track, amber when on.
  let {
    checked,
    label,
    disabled = false,
    onchange,
  }: { checked: boolean; label: string; disabled?: boolean; onchange: (checked: boolean) => void } = $props();
</script>

<button
  class="switch"
  class:on={checked}
  role="switch"
  aria-checked={checked}
  aria-label={label}
  {disabled}
  onclick={() => onchange(!checked)}
>
  <span class="knob"></span>
</button>

<style>
  .switch {
    position: relative;
    flex: none;
    width: 36px;
    height: 20px;
    padding: 0;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-pill);
    background: var(--surface-3);
    box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.35);
    cursor: pointer;
    transition: background 0.12s ease;
  }

  .switch:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .switch:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .knob {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: var(--muted);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.4);
    transition: left 0.16s var(--ease);
  }

  .on {
    border-color: var(--accent);
    background: var(--accent);
  }

  .on .knob {
    left: 18px;
    background: #fff;
  }

  @media (prefers-reduced-motion: reduce) {
    .switch,
    .knob {
      transition: none;
    }
  }
</style>
