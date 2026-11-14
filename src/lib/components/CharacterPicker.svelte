<script lang="ts">
  // Which character the Train tab's numbers are for. Follows the name on the HUD; with the game
  // closed, or to look at another character, pick one. Shown once there are two to choose from.
  import { characters } from "$lib/characters.svelte";
  import { live } from "$lib/readings.svelte";

  const onScreen = $derived(live.session?.name ?? null);
  const elsewhere = $derived(characters.all || (characters.active != null && onScreen != null && characters.active !== onScreen));
  const ALL = "";

  function pick(value: string) {
    if (value === ALL) characters.chooseAll();
    else characters.choose(value);
  }
</script>

{#if characters.known.length > 1}
  <div class="picker">
    <label for="character">Character</label>
    <select id="character" value={characters.all ? ALL : (characters.active ?? ALL)} onchange={(e) => pick(e.currentTarget.value)}>
      <option value={ALL}>All characters</option>
      {#each characters.known as name (name)}
        <option value={name}>{name}</option>
      {/each}
    </select>
    {#if characters.all}
      <span class="hint">Goal and potions stay with {characters.active}.</span>
      <button class="back" onclick={() => characters.choose(null)}>Back to {onScreen ?? characters.active}</button>
    {:else if elsewhere}
      <button class="back" onclick={() => characters.choose(null)}>Back to {onScreen}</button>
    {:else}
      <span class="hint">Rates and best maps are kept per character.</span>
    {/if}
  </div>
{/if}

<style>
  .picker {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    border-bottom: 1px solid var(--border-strong);
    font-size: var(--text-sm, 12px);
  }
  label {
    color: var(--muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-size: var(--text-xs);
  }
  .hint {
    color: var(--muted);
  }
  .back {
    all: unset;
    cursor: pointer;
    text-decoration: underline;
  }
</style>
