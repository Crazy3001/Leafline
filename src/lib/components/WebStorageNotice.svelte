<script lang="ts">
  // The website keeps your records in this browser, and only one tab can hold them at a time.
  // This says when another tab has them, or this browser can't keep them, and lets you take them.
  import { storage, takeOver } from "$lib/platform/storage.svelte";

  let taking = $state(false);

  async function take() {
    taking = true;
    try {
      await takeOver();
    } finally {
      taking = false;
    }
  }
</script>

{#if storage.state === "in-use" || storage.state === "moved" || storage.state === "failed"}
  <div class="notice" role="status">
    {#if storage.state === "failed"}
      <span>
        Leafline couldn't open its storage in this browser{storage.problem ? `: ${storage.problem}` : ""}. It works in
        current Chrome, Edge, Firefox and Safari, outside private windows.
      </span>
    {:else}
      <span>
        {storage.state === "moved" ? "Leafline moved to another tab." : "Leafline is open in another tab."} One tab at a
        time can use your records.
      </span>
      <button class="button" onclick={take} disabled={taking}>{taking ? "Waiting…" : "Use it here"}</button>
    {/if}
  </div>
{/if}

<style>
  .notice {
    position: fixed;
    inset: 0 0 auto 0;
    z-index: 1000;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: center;
    gap: 10px;
    padding: 8px 16px;
    border-bottom: var(--line-w) solid var(--accent);
    background: var(--surface);
    color: var(--text);
    font-size: var(--text-sm);
  }

  .button {
    height: var(--control-h);
    padding: 0 12px;
    border: var(--line-w) solid var(--accent);
    border-radius: var(--radius);
    background: var(--accent);
    color: var(--accent-ink);
    font: inherit;
    cursor: pointer;
  }
</style>
