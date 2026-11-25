<script lang="ts">
  // What a tab shows before anything has been read from the game: three steps to get going.
  import { backend } from "$lib/api";
  import Logo from "$lib/components/Logo.svelte";
  import { hotkey } from "$lib/settings.svelte";
</script>

<div class="welcome card">
  <Logo size={28} />
  <h1>Welcome to Leafline</h1>
  <p class="hint">It reads what your game screen shows and puts useful facts beside it.</p>
  <ol>
    <li>
      <span class="n">1</span>
      <span>Turn on <button class="inline" onclick={() => backend.showSettings()}>live reading</button> to read the game as you play.</span>
    </li>
    <li>
      <span class="n">2</span>
      <span>Download map data in the Map tab for routes, training spots and quest help.</span>
    </li>
    <li>
      <span class="n">3</span>
      <!-- No hotkey is bound until the player picks one, so name only the ones they have. -->
      {#if hotkey("ToggleOverlay") || hotkey("ToggleClickThrough")}
        <span>
          {#if hotkey("ToggleOverlay")}<kbd>{hotkey("ToggleOverlay")}</kbd> shows or hides this window.{/if}
          {#if hotkey("ToggleClickThrough")}<kbd>{hotkey("ToggleClickThrough")}</kbd> lets clicks pass through it.{/if}
        </span>
      {:else}
        <span>Pick hotkeys in Settings to show, hide or lock this window without leaving the game.</span>
      {/if}
    </li>
  </ol>
  <button class="inline small" onclick={() => backend.showSettings()}>Settings and what Leafline does</button>
</div>

<style>
  .welcome {
    display: grid;
    justify-items: center;
    gap: 6px;
    margin: var(--space-3) 10px;
    padding: 24px 18px 18px;
    text-align: center;
    color: var(--muted);
  }

  h1 {
    margin: 6px 0 0;
    color: var(--text);
    font-family: var(--font-display);
    font-size: var(--text-lg);
    font-weight: var(--weight-strong);
  }

  .hint {
    max-width: 260px;
    margin: 0;
    color: var(--faint);
    font-size: var(--text-md);
  }

  ol {
    display: grid;
    gap: 10px;
    margin: 12px 0 6px;
    padding: 0;
    list-style: none;
    text-align: left;
    font-size: var(--text-md);
  }

  li {
    display: flex;
    gap: 9px;
  }

  .n {
    display: grid;
    flex: none;
    place-items: center;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: var(--accent-soft);
    color: var(--accent-text);
    font-family: var(--font-display);
    font-size: var(--text-xs);
    font-weight: var(--weight-strong);
  }

  .inline {
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--accent-text);
    font: inherit;
    font-weight: var(--weight-strong);
    cursor: pointer;
  }

  .inline:hover,
  .inline:focus-visible {
    text-decoration: underline;
    outline: none;
  }

  .inline.small {
    color: var(--muted);
    font-size: var(--text-md);
    font-weight: inherit;
  }

  kbd {
    padding: 0 4px;
    border: 1px solid var(--border-strong);
    border-bottom-width: 2px;
    border-radius: var(--radius-sm);
    color: var(--muted);
    font-family: var(--font-sans);
    font-size: var(--text-xs);
  }
</style>
