<script lang="ts">
  // One hotkey: click it, then press the keys. Esc cancels; Backspace clears it. Used by Settings
  // and the first-launch setup. A key that is already a hotkey never reaches the page (Windows
  // gives it to Leafline), so while this waits, hotkeys are kept quiet and one that arrives is
  // taken as the key pressed.
  import { backend } from "$lib/api";
  import { display, keyName, quietHotkeys } from "$lib/settings.svelte";

  let {
    keys,
    disabled = false,
    taken = false,
    label,
    onchange,
  }: { keys: string; disabled?: boolean; taken?: boolean; label: string; onchange: (keys: string) => void } = $props();

  let listening = $state(false);

  $effect(() => {
    if (!listening) return;
    const loud = quietHotkeys();
    const stop = backend.onHotkey((action) => {
      const pressed = display.hotkeys.keys[action];
      if (!listening || !pressed) return;
      listening = false;
      onchange(pressed);
    });
    return () => {
      loud();
      stop.then((unlisten) => unlisten());
    };
  });

  /** A pressed key as Leafline writes it: `Ctrl+Shift+K`, `F5`, `Pause`, `Numpad5`. Null for a modifier alone. */
  function keysOf(event: KeyboardEvent): string | null {
    const code = event.code;
    const key = code.startsWith("Key")
      ? code.slice(3)
      : code.startsWith("Digit")
        ? code.slice(5)
        : /^(F\d{1,2}|Pause|ScrollLock|Numpad(\d|Add|Subtract|Multiply|Divide|Decimal|Enter)|Space|Home|End|Insert|Delete|PageUp|PageDown|Arrow(Up|Down|Left|Right))$/.test(code)
          ? code
          : null;
    if (!key) return null;
    const held = [event.ctrlKey && "Ctrl", event.altKey && "Alt", event.shiftKey && "Shift", event.metaKey && "Super"];
    return [...held.filter(Boolean), key].join("+");
  }

  function onkeydown(event: KeyboardEvent) {
    if (!listening) return;
    event.preventDefault();
    if (event.key === "Escape") {
      listening = false;
    } else if (event.key === "Backspace") {
      listening = false;
      onchange("");
    } else {
      const pressed = keysOf(event);
      if (!pressed) return;
      listening = false;
      onchange(pressed);
    }
  }
</script>

<button
  class="hotkey"
  class:listening
  class:taken={taken && !listening}
  {disabled}
  title={taken ? "Another program on this PC uses this key, so Leafline can't. Pick another." : undefined}
  aria-label="{label}: {listening ? 'press keys' : keys ? keyName(keys) : 'no hotkey'}{taken ? ', in use by another program' : ''}"
  onclick={() => (listening = !listening)}
  onblur={() => (listening = false)}
  {onkeydown}
>
  {#if listening}
    Press keys…
  {:else if keys}
    {#each keyName(keys).split("+") as key, i}{#if i}<span class="plus">+</span>{/if}<kbd>{key}</kbd>{/each}
  {:else}
    None
  {/if}
</button>

<style>
  .hotkey {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    min-width: 54px;
    justify-content: center;
    padding: 2px 6px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--muted);
    font: inherit;
    font-size: var(--text-sm);
    cursor: pointer;
  }

  .hotkey:hover:enabled {
    border-color: var(--border-strong);
  }

  .hotkey:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .hotkey.listening {
    border-color: var(--accent);
    color: var(--accent-strong);
  }

  .hotkey.taken {
    border-color: var(--warn);
  }

  .hotkey.taken kbd {
    color: var(--warn);
    text-decoration: line-through;
  }

  .hotkey.taken {
    border-color: var(--warn);
  }

  .hotkey.taken kbd {
    color: var(--warn);
    text-decoration: line-through;
  }

  .hotkey:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  kbd {
    padding: 0 4px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    color: var(--text);
    font-family: var(--font-sans);
    font-size: var(--text-xs);
  }

  .plus {
    color: var(--faint);
  }
</style>
