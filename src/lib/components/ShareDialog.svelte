<script lang="ts">
  // Share a saved session: a picture and a few lines of text the player copies or saves and
  // posts themselves. Leafline sends nothing. The character's name is left out unless ticked.
  import { Check, Copy, Download, X } from "@lucide/svelte";

  import { backend } from "$lib/api";
  import { copyPicture, drawShareCard, pngBytes, shareText, type ShareOptions } from "$lib/share";
  import type { LoggedDrop, SavedSession, SessionTimeline } from "$lib/types";
  import Button from "$lib/ui/Button.svelte";

  let {
    session,
    timeline,
    drops,
    onclose,
  }: { session: SavedSession; timeline: SessionTimeline | null; drops: LoggedDrop[]; onclose: () => void } = $props();

  let canvas = $state<HTMLCanvasElement>();
  let options = $state<ShareOptions>({ name: false, maps: true, drops: true });
  let done = $state<"picture" | "text" | "saved" | null>(null);
  let problem = $state<string | null>(null);
  let doneTimer: ReturnType<typeof setTimeout> | undefined;

  $effect(() => {
    if (canvas) drawShareCard(canvas, session, timeline, drops, { ...options });
  });

  function flash(what: typeof done) {
    done = what;
    problem = null;
    clearTimeout(doneTimer);
    doneTimer = setTimeout(() => (done = null), 1800);
  }

  async function run(what: NonNullable<typeof done>, action: () => Promise<unknown>) {
    try {
      if ((await action()) !== false) flash(what);
    } catch (e) {
      problem =
        what === "saved"
          ? `Couldn't save it: ${e}`
          : `Couldn't copy it${what === "picture" ? ". Save picture works instead" : ""}. (${e})`;
    }
  }

  const copyImage = () => run("picture", () => copyPicture(canvas!));
  const copyText = () => run("text", () => navigator.clipboard.writeText(shareText(session, drops, options)));
  const save = () =>
    run("saved", async () => {
      const day = new Date((session.startedAt ?? session.savedAt) * 1000).toISOString().slice(0, 10);
      return backend.saveImage(await pngBytes(canvas!), `leafline-session-${day}.png`);
    });

  function onKey(event: KeyboardEvent) {
    if (event.key === "Escape") onclose();
  }
</script>

<svelte:window onkeydown={onKey} />

<div class="backdrop" role="presentation" onclick={(e) => e.target === e.currentTarget && onclose()}>
  <div class="dialog" role="dialog" aria-modal="true" aria-labelledby="share-title">
    <header>
      <h2 id="share-title">Share this session</h2>
      <button class="close" aria-label="Close" onclick={onclose}><X size={16} /></button>
    </header>

    <canvas bind:this={canvas} class="preview" aria-label="The picture to share"></canvas>

    <div class="options">
      <label><input type="checkbox" bind:checked={options.name} disabled={!session.character} /> Character name</label>
      <label><input type="checkbox" bind:checked={options.maps} disabled={!session.maps.length} /> Maps</label>
      <label><input type="checkbox" bind:checked={options.drops} disabled={!drops.length} /> Drops ({drops.length})</label>
    </div>

    <footer>
      <p class="quiet">Leafline doesn't post anything. Copy or save it, then share it yourself.</p>
      {#if problem}<p class="problem">{problem}</p>{/if}
      <div class="actions">
        <Button onclick={copyText}>{#if done === "text"}<Check size={13} />Copied{:else}<Copy size={13} />Copy text{/if}</Button>
        <Button onclick={save}>{#if done === "saved"}<Check size={13} />Saved{:else}<Download size={13} />Save picture{/if}</Button>
        <Button variant="primary" onclick={copyImage}>{#if done === "picture"}<Check size={13} />Copied{:else}<Copy size={13} />Copy picture{/if}</Button>
      </div>
    </footer>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: grid;
    place-items: center;
    padding: 24px;
    background: rgba(0, 0, 0, 0.6);
  }

  .dialog {
    display: grid;
    gap: 12px;
    width: min(760px, 100%);
    max-height: 100%;
    overflow: auto;
    padding: 16px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg);
    background: var(--surface);
    box-shadow: var(--shadow);
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  h2 {
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--text-lg);
    font-weight: 600;
  }

  .close {
    display: grid;
    place-items: center;
    width: var(--control-h);
    height: var(--control-h);
    padding: 0;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--muted);
    cursor: pointer;
  }

  .close:hover {
    background: var(--surface-3);
    color: var(--text);
  }

  .preview {
    width: 100%;
    height: auto;
    aspect-ratio: 1200 / 630;
    border: 1px solid var(--border);
  }

  .options {
    display: flex;
    flex-wrap: wrap;
    gap: 16px;
    font-size: var(--text-sm);
    color: var(--muted);
  }

  .options label {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
  }

  .options input {
    accent-color: var(--accent);
  }

  footer {
    display: grid;
    gap: 8px;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .quiet {
    margin: 0;
    color: var(--faint);
    font-size: var(--text-xs);
  }

  .problem {
    margin: 0;
    color: var(--bad);
    font-size: var(--text-sm);
  }
</style>
