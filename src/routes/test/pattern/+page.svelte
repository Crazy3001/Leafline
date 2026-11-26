<script lang="ts">
  // The capture test pattern (website plan, stage 2), pixel for pixel: shown by the program in a
  // window of its own, or on the website in a window the test page opens. The test page shares
  // this window and compares what the browser delivers with the same pattern drawn again.
  import { onMount } from "svelte";
  import { isTauri } from "@tauri-apps/api/core";

  import { PATTERN, type WindowInfo } from "$lib/capture/types";

  let canvas: HTMLCanvasElement;
  let info = $state<WindowInfo | null>(null);
  let problem = $state<string | null>(null);

  const small = $derived(info != null && (info.width < PATTERN.width || info.height < PATTERN.height));

  async function pixels(window: WindowInfo): Promise<Uint8ClampedArray> {
    // The literal constant, so the program's build leaves the website's WebAssembly out.
    if (__LEAFLINE_WEB__) return (await import("$lib/capture/pattern-web")).patternPixels(window);
    if (!isTauri()) throw new Error("The pattern is drawn by the program or the website, not the dev preview.");
    return (await import("$lib/capture/program")).patternPixels(window);
  }

  async function draw() {
    const scale = window.devicePixelRatio || 1;
    const now: WindowInfo = {
      width: Math.round(window.innerWidth * scale),
      height: Math.round(window.innerHeight * scale),
      scaleMilli: Math.round(scale * 1000),
    };
    if (info && now.width === info.width && now.height === info.height && now.scaleMilli === info.scaleMilli) return;
    info = now;
    try {
      const data = await pixels(now);
      // One canvas pixel to one screen pixel: its CSS size is the pattern's over the scale.
      canvas.width = PATTERN.width;
      canvas.height = PATTERN.height;
      canvas.style.width = `${PATTERN.width / scale}px`;
      canvas.style.height = `${PATTERN.height / scale}px`;
      canvas.getContext("2d")!.putImageData(new ImageData(new Uint8ClampedArray(data), PATTERN.width, PATTERN.height), 0, 0);
      problem = null;
    } catch (err) {
      problem = String(err);
    }
  }

  onMount(() => {
    void draw();
    let timer: ReturnType<typeof setTimeout>;
    const later = () => {
      clearTimeout(timer);
      timer = setTimeout(draw, 150);
    };
    window.addEventListener("resize", later);
    return () => window.removeEventListener("resize", later);
  });
</script>

<svelte:head><title>Leafline capture test pattern</title></svelte:head>

<main>
  <canvas bind:this={canvas}></canvas>
  <!-- Moves below the pattern so the window keeps changing: a still window gets few frames, and
       the game never stands still. -->
  <div class="alive"><span></span></div>
  {#if problem}
    <p class="note bad">{problem}</p>
  {:else if small}
    <p class="note bad">
      This window is {info?.width} x {info?.height} screen pixels; the pattern needs {PATTERN.width} x {PATTERN.height}.
      Make the window bigger.
    </p>
  {:else if info}
    <p class="note">
      Leafline capture test pattern · window {info.width} x {info.height} at {info.scaleMilli / 10}% · keep it uncovered
      while it is shared
    </p>
  {/if}
</main>

<style>
  :global(html),
  :global(body) {
    background: rgb(90, 90, 90);
    overflow: hidden;
  }

  main {
    margin: 0;
    padding: 0;
  }

  canvas {
    display: block;
    image-rendering: pixelated;
  }

  .alive {
    position: relative;
    height: 4px;
    overflow: hidden;
  }

  .alive span {
    position: absolute;
    width: 60px;
    height: 4px;
    background: #f4a93b;
    animation: sweep 2s linear infinite;
  }

  @keyframes sweep {
    from {
      left: -60px;
    }
    to {
      left: 100%;
    }
  }

  .note {
    margin: var(--space-1) var(--space-2);
    color: #eee;
    font: 12px/1.4 var(--font-sans);
  }

  .bad {
    color: #ffb3ad;
  }
</style>
