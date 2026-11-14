<script lang="ts">
  // The one tooltip a window shows, under the element the mouse rests on (above it when there is
  // no room below), always inside the window. See `$lib/tip.svelte`.
  import { hideTip, tipState } from "$lib/tip.svelte";

  const MARGIN = 6;
  const GAP = 6;

  let box = $state<HTMLElement | null>(null);
  let place = $state<{ left: number; top: number } | null>(null);

  $effect(() => {
    const anchor = tipState.anchor;
    void tipState.body;
    place = null;
    if (!anchor || !box) return;
    const frame = requestAnimationFrame(() => {
      if (!box || !anchor.isConnected) return hideTip();
      const a = anchor.getBoundingClientRect();
      const { width, height } = box.getBoundingClientRect();
      const vw = window.innerWidth;
      const vh = window.innerHeight;
      const left = Math.min(Math.max(a.left + a.width / 2 - width / 2, MARGIN), vw - width - MARGIN);
      const below = a.bottom + GAP;
      const top = below + height <= vh - MARGIN || a.top - GAP - height < MARGIN ? below : a.top - GAP - height;
      place = { left: Math.max(left, MARGIN), top: Math.max(Math.min(top, vh - height - MARGIN), MARGIN) };
    });
    return () => cancelAnimationFrame(frame);
  });

  $effect(() => {
    if (!tipState.anchor) return;
    const away = () => hideTip();
    window.addEventListener("scroll", away, true);
    window.addEventListener("wheel", away, { passive: true });
    window.addEventListener("keydown", away);
    window.addEventListener("resize", away);
    window.addEventListener("blur", away);
    return () => {
      window.removeEventListener("scroll", away, true);
      window.removeEventListener("wheel", away);
      window.removeEventListener("keydown", away);
      window.removeEventListener("resize", away);
      window.removeEventListener("blur", away);
    };
  });
</script>

{#if tipState.anchor && tipState.body}
  {@const body = tipState.body}
  <div
    class="tip"
    class:words={typeof body === "string"}
    role="tooltip"
    bind:this={box}
    style:left="{place?.left ?? 0}px"
    style:top="{place?.top ?? 0}px"
    style:visibility={place ? "visible" : "hidden"}
  >
    {#if typeof body === "string"}{body}{:else}{@render body()}{/if}
  </div>
{/if}

<style>
  .tip {
    position: fixed;
    z-index: 2000;
    max-width: min(280px, calc(100vw - 12px));
    padding: 6px 9px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface-raised);
    box-shadow: var(--shadow-lg);
    color: var(--text);
    font-size: var(--text-xs);
    line-height: 1.45;
    pointer-events: none;
    animation: appear 90ms ease-out;
  }

  .tip.words {
    white-space: pre-line;
  }

  @keyframes appear {
    from {
      opacity: 0;
    }
  }
</style>
