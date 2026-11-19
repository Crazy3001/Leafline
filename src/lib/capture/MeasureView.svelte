<script lang="ts">
  // Part A's results: the pattern as a capture delivered it, against the pattern drawn again.
  import { rgb, type Measure } from "./types";

  let { measure, format = null }: { measure: Measure; format?: string | null } = $props();

  const found = $derived(measure.found);
  const channels = ["Red", "Green", "Blue"];
  const partNames: Record<string, string> = {
    code: "Code strip",
    greyRamp: "Grey ramp",
    colourRamps: "Colour ramps",
    swatches: "Swatches",
    lines: "1-px lines",
    glyphs: "Font strips",
  };
  const readers: Record<string, string> = {
    questReader: "Quest Helper reader",
    pickupReader: "Pickup Log reader",
    standIn: "stand-in (game reads by OCR)",
  };
  /** The worst lines, by how far their own colour moved. */
  const worst = $derived(found ? [...found.lines].sort((a, b) => b.off - a.off).slice(0, 6) : []);
  const sameSize = $derived(
    found ? measure.frameWidth === found.window.width && measure.frameHeight === found.window.height : false,
  );
  const level = (off: number) => (off <= 2 ? "good" : off <= 12 ? "warn" : "bad");
</script>

{#if !found}
  <p class="note bad">{measure.problem ?? "The pattern wasn't found."}</p>
  <p class="note">Frame {measure.frameWidth} x {measure.frameHeight}{format ? `, ${format}` : ""}.</p>
{:else}
  <dl class="facts">
    {#if format}
      <div><dt>Pixel format</dt><dd>{format}</dd></div>
    {/if}
    <div>
      <dt>Size</dt>
      <dd>
        Frame {measure.frameWidth} x {measure.frameHeight}, window {found.window.width} x {found.window.height} at
        {found.window.scaleMilli / 10}%:
        {#if sameSize}
          <span class="good">pixel for pixel</span>
        {:else}
          the frame is {measure.frameWidth - found.window.width} x {measure.frameHeight - found.window.height} bigger
          (title bar or border); pattern at {found.x}, {found.y}, pixel for pixel
        {/if}
      </dd>
    </div>
    <div>
      <dt>Error per channel</dt>
      <dd>
        {#each found.channels as c, i}
          <span class="channel">{channels[i]} mean {c.mean.toFixed(2)}, largest {c.largest}</span>
        {/each}
      </dd>
    </div>
    <div>
      <dt>Grey ramp</dt>
      <dd>
        black {found.ramp.black}, white {found.ramp.white}, largest step error
        <span class={level(found.ramp.largest)}>{found.ramp.largest}</span>{found.ramp.squeezed
          ? " — squeezed to video levels"
          : ""}
      </dd>
    </div>
    <div>
      <dt>Colour bleed</dt>
      <dd>
        beside 1-px lines: typically <span class={found.bleed.typical > 1 ? "bad" : found.bleed.typical ? "warn" : "good"}
          >{found.bleed.typical} px</span
        >, at most {found.bleed.largest} px
      </dd>
    </div>
  </dl>

  <h4>By part</h4>
  <table>
    <thead><tr><th>Part</th><th>Red</th><th>Green</th><th>Blue</th></tr></thead>
    <tbody>
      {#each found.parts as p}
        <tr>
          <td>{partNames[p.part] ?? p.part}</td>
          {#each p.channels as c}<td class="num">{c.mean.toFixed(2)} / {c.largest}</td>{/each}
        </tr>
      {/each}
    </tbody>
  </table>
  <p class="hint">Mean / largest difference, in levels of 255.</p>

  <h4>Font strips read back</h4>
  <table>
    <thead><tr><th>Strip</th><th>Read by</th><th>Read</th><th>Flipped</th></tr></thead>
    <tbody>
      {#each found.glyphs as g}
        <tr>
          <td>{g.name}</td>
          <td class="muted">{readers[g.readBy]}</td>
          <td class={g.read === g.expected ? "good" : "bad"}>{g.read ?? "nothing"}</td>
          <td class="num">{g.flipped} / {g.fontPixels}</td>
        </tr>
      {/each}
    </tbody>
  </table>

  <h4>Swatches</h4>
  <table>
    <thead><tr><th>Colour</th><th></th><th>Drawn</th><th>Seen</th><th>Off</th></tr></thead>
    <tbody>
      {#each found.swatches as s}
        <tr>
          <td>{s.name}</td>
          <td><span class="chip" style:background={rgb(s.expected)}></span><span class="chip" style:background={rgb(s.seen)}></span></td>
          <td class="num">{s.expected.join(", ")}</td>
          <td class="num">{s.seen.join(", ")}</td>
          <td class="num {level(s.off)}">{s.off}</td>
        </tr>
      {/each}
    </tbody>
  </table>

  <h4>1-px lines, worst first</h4>
  <table>
    <thead><tr><th>Line</th><th>On</th><th>Seen</th><th>Off</th><th>Bleed</th></tr></thead>
    <tbody>
      {#each worst as l}
        <tr>
          <td><span class="chip" style:background={rgb(l.colour)}></span>{l.colour.join(", ")} {l.vertical ? "upright" : "across"}</td>
          <td class="num">{l.ground.join(", ")}</td>
          <td class="num">{l.seen.join(", ")}</td>
          <td class="num {level(l.off)}">{l.off}</td>
          <td class="num">{l.bleed} px</td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}

<style>
  .facts {
    display: grid;
    gap: 6px;
    margin: 0 0 12px;
  }

  .facts div {
    display: grid;
    grid-template-columns: 140px 1fr;
    gap: 12px;
  }

  dt {
    color: var(--muted);
  }

  dd {
    margin: 0;
  }

  .channel {
    margin-right: 14px;
    white-space: nowrap;
  }

  h4 {
    margin: 16px 0 6px;
    font-family: var(--font-display);
    font-size: var(--text-sm);
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--text-sm);
  }

  th {
    text-align: left;
    font-weight: 600;
    color: var(--muted);
  }

  th,
  td {
    padding: 4px 8px 4px 0;
    border-bottom: var(--line-w) solid var(--line);
  }

  .num {
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .chip {
    display: inline-block;
    width: 14px;
    height: 14px;
    margin-right: 4px;
    border: 1px solid var(--border-strong);
    border-radius: 3px;
    vertical-align: -2px;
  }

  .muted,
  .hint {
    color: var(--muted);
  }

  .hint {
    margin: 4px 0 0;
    font-size: var(--text-xs);
  }

  .note {
    margin: 4px 0;
  }

  .good {
    color: var(--good);
  }

  .warn {
    color: var(--warn);
  }

  .bad {
    color: var(--bad);
  }
</style>
