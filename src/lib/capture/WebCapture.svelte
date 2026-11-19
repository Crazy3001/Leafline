<script lang="ts">
  // The website's capture test page (website plan, stage 2): a shared window's frames measured
  // against the test pattern (A), saved beside the program's for comparison (B), and read for a
  // quarter of an hour behind the game (C). Website only.
  import { base } from "$app/paths";
  import { onMount } from "svelte";

  import MeasureView from "./MeasureView.svelte";
  import {
    canRead,
    capture,
    clearOwnFrames,
    downloadOwnFrames,
    enduranceCsv,
    pickFolder,
    useOwnStorage,
    sendJobs,
    setEndurance,
    setPerSecond,
    setSaving,
    sharedTrack,
    share,
    stopSharing,
    support,
  } from "./session.svelte";
  import { PATTERN } from "./types";

  let clock = $state(Date.now());
  let copied = $state<string | null>(null);

  onMount(() => {
    const timer = setInterval(() => (clock = Date.now()), 1000);
    // For checking the page by script: `?debug` lets a test hand over a folder without the picker.
    if (new URLSearchParams(location.search).has("debug")) {
      (window as unknown as Record<string, unknown>).leaflineCaptureTest = { capture, setSaving, setEndurance, sharedTrack };
    }
    return () => {
      clearInterval(timer);
      stopSharing();
    };
  });

  const stats = $derived(capture.stats);
  const quiet = $derived(capture.sharing && capture.statsAt > 0 ? Math.round((clock - capture.statsAt) / 1000) : 0);
  const minutes = $derived(capture.endurance ? Math.floor((clock - capture.endurance.startedAt) / 60000) : 0);
  /** Part C's bar after 10 minutes: Pickup Log looks at nearly four a second, frames at least as often. */
  const late = $derived(capture.rows.filter((r) => r.minute > 10));
  const keepsUp = $derived(late.length ? late.every((r) => r.reads / 60 >= 3.6 && r.frames / 60 >= 4) : null);

  function openPattern() {
    const scale = window.devicePixelRatio || 1;
    const width = Math.ceil(PATTERN.width / scale) + 16;
    const height = Math.ceil(PATTERN.height / scale) + 40;
    window.open(`${base}/test/pattern`, "leafline-pattern", `popup,width=${width},height=${height}`);
  }

  function toggleMeasure() {
    capture.measuring = !capture.measuring;
    sendJobs();
  }

  function toggleReaders() {
    capture.reading = !capture.reading;
    sendJobs();
  }

  async function copy(text: string, what: string) {
    try {
      await navigator.clipboard.writeText(text);
      copied = `${what} copied.`;
    } catch (err) {
      copied = `Couldn't copy: ${err}`;
    }
  }

  function download(text: string, name: string) {
    const url = URL.createObjectURL(new Blob([text], { type: "text/csv" }));
    const a = Object.assign(document.createElement("a"), { href: url, download: name });
    a.click();
    URL.revokeObjectURL(url);
  }

  const format = $derived(stats?.format ? `${stats.format}${stats.colorSpace ? ` (${stats.colorSpace})` : ""}` : null);
</script>

<h1>Capture test</h1>
<p class="lead">
  A check of whether the website can read the game from a window shared to Chrome or Edge. It is a go/no-go test for
  Leafline's developers, not a player feature. Frames stay in this tab: nothing is uploaded, and frames are written only
  to a folder you pick.
</p>
{#if !canRead}
  <p class="status bad">
    {support.share
      ? "This browser can share a window but can't hand its frames to the page."
      : "This browser can't share a window."} Reading the game needs Chrome, Edge or Firefox on a computer.
  </p>
{/if}

<section>
  <h2>Shared window</h2>
  <div class="controls">
    {#if capture.sharing}
      <button class="button" onclick={stopSharing}>Stop sharing</button>
    {:else}
      <button class="button primary" onclick={share} disabled={!canRead}>Share a window…</button>
    {/if}
    <span class="muted">{capture.sharing ? capture.label || "Shared" : "Nothing shared"}</span>
  </div>
  {#if capture.sharing}
    <p class="status">
      {#if stats}
        {stats.fps.toFixed(0)} frames a second · {stats.frames} so far · {format ?? "format not given"} · coded {stats.codedWidth}
        x {stats.codedHeight}, shown {stats.displayWidth} x {stats.displayHeight} · {capture.readIn === "video" ? "taken from a video of the stream" : `read in the ${capture.readIn}`}{capture.readInWhy
          ? ` (the browser wouldn't hand the stream to the worker: ${capture.readInWhy})`
          : ""}
        {#if quiet > 3}<span class="warn"> · no frame for {quiet} s</span>{/if}
      {:else}
        Waiting for the first frame…
      {/if}
    </p>
  {/if}
  {#if capture.error}<p class="status bad">{capture.error}</p>{/if}
</section>

<section>
  <h2>A. Test pattern</h2>
  <p class="lead">
    Shows exactly what the browser does to a window's pixels: its pixel format, its size against the window's, colour
    errors and how far colour bleeds beside 1-px lines.
  </p>
  <ol>
    <li>
      Open the pattern: in the program, Settings › Feedback › Capture test › Show the pattern (a dev build), or here:
      <button class="button" onclick={openPattern}>Open a pattern window</button>
    </li>
    <li>Share that window above. Keep it uncovered and its zoom at 100%.</li>
  </ol>
  <div class="controls">
    <button class="button" onclick={toggleMeasure}>{capture.measuring ? "Stop measuring" : "Measure"}</button>
    {#if capture.measure}
      <button class="button" onclick={() => copy(JSON.stringify({ format: stats?.format, colorSpace: stats?.colorSpace, readIn: capture.readIn, fps: stats?.fps, measure: capture.measure }, null, 1), "Results")}>
        Copy results
      </button>
    {/if}
    {#if copied}<span class="muted">{copied}</span>{/if}
  </div>
  {#if capture.measure && capture.measuring}
    <MeasureView measure={capture.measure} {format} />
  {/if}
</section>

<section>
  <h2>B. Frames for the comparison</h2>
  <p class="lead">
    With the game shared here and the program saving its own frames into the same folder, saves a frame on the same
    clock ticks as the program, named <code>web-&lt;time&gt;.png</code>. The program's Capture test window then compares
    the two sets. Pick a folder outside OneDrive: these are pictures of your game.
  </p>
  {#if !support.folder && support.ownStorage}
    <p class="lead">
      This browser can't write to a folder you pick, so frames are kept in its own storage for this site, then
      downloaded into your Downloads folder (about 6 MB each). Move them into the program's folder to compare, and delete
      them here afterwards.
    </p>
  {/if}
  <div class="controls">
    {#if support.folder}
      <button class="button" onclick={pickFolder}>
        {capture.folder ? `Folder: ${capture.folder.name}` : "Pick a folder…"}
      </button>
    {:else if support.ownStorage}
      <button class="button" onclick={useOwnStorage} disabled={capture.folderIsOwn}>
        {capture.folderIsOwn ? "Saving in this browser" : "Save in this browser"}
      </button>
    {/if}
    <label>
      <select value={capture.perSecond} onchange={(e) => setPerSecond(Number(e.currentTarget.value))}>
        <option value={1}>1 a second</option>
        <option value={2}>2 a second</option>
        <option value={4}>4 a second (Pickup Log)</option>
      </select>
    </label>
    {#if capture.savingOn}
      <button class="button" onclick={() => setSaving(false)}>Stop saving</button>
    {:else}
      <button class="button primary" onclick={() => setSaving(true)} disabled={!capture.folder || !capture.sharing}>
        Start saving
      </button>
    {/if}
  </div>
  {#if capture.folderIsOwn && !capture.savingOn && stats?.saved}
    <div class="controls">
      <button class="button" onclick={downloadOwnFrames} disabled={!!capture.downloading && capture.downloading.done < capture.downloading.total}>
        Download the frames
      </button>
      <button class="button" onclick={clearOwnFrames}>Delete them here</button>
      {#if capture.downloading}
        <span class="muted">{capture.downloading.done} of {capture.downloading.total} downloaded</span>
      {/if}
    </div>
  {/if}
  {#if !support.folder && !support.ownStorage}
    <p class="status bad">This browser can't save frames. Use Chrome, Edge or Firefox.</p>
  {/if}
  {#if stats && (capture.savingOn || stats.saved)}
    <p class="status">
      {stats.saved} saved{stats.dropped ? `, ${stats.dropped} skipped (saving fell behind)` : ""}{stats.saving
        ? `, ${stats.saving} being written`
        : ""}
      {#if stats.saveProblem}<span class="bad"> · {stats.saveProblem}</span>{/if}
    </p>
  {/if}
</section>

<section>
  <h2>C. Endurance</h2>
  <p class="lead">
    Reads every frame the way the website would (the Pickup Log four times a second, the rest once a second, without OCR)
    and logs each minute. Start it, put the game in front and play for 15 minutes; note any stutter in the game.
  </p>
  <div class="controls">
    {#if capture.endurance}
      <button class="button" onclick={() => setEndurance(false)}>Stop</button>
      <span>{minutes} min</span>
    {:else}
      <button class="button primary" onclick={() => setEndurance(true)} disabled={!capture.sharing}>Start</button>
    {/if}
    <button class="button" onclick={toggleReaders} disabled={!!capture.endurance}>
      {capture.reading || capture.endurance ? "Readers on" : "Try the readers"}
    </button>
    {#if capture.rows.length}
      <button class="button" onclick={() => download(enduranceCsv(), "leafline-endurance.csv")}>Save the log</button>
      <button class="button" onclick={() => copy(enduranceCsv(), "Log")}>Copy the log</button>
    {/if}
  </div>
  {#if capture.read}
    <p class="status">
      Last read {capture.read.ms.toFixed(1)} ms ({capture.read.full ? "everything" : "Pickup Log only"}) · HUD
      {capture.read.hud ? `found, level ${capture.read.level ?? "?"}` : "not found"} · Pickup Log {capture.read.pickupLog
        ? "found"
        : "not found"}
      {#if stats?.readMean != null}· typically {stats.readMean.toFixed(1)} ms{/if}
    </p>
  {/if}
  {#if keepsUp != null}
    <p class={keepsUp ? "status good" : "status bad"}>
      After 10 minutes: {keepsUp
        ? "frames and Pickup Log looks kept up every minute."
        : "some minutes fell short of 4 frames and 3.6 Pickup Log looks a second."}
    </p>
  {/if}
  {#if capture.rows.length}
    <table>
      <thead>
        <tr>
          <th>Min</th><th>Hidden</th><th>Frames/s</th><th>Looks/s</th><th>Read ms</th><th>p95</th><th>Copy ms</th><th>Busy</th
          ><th>CPU</th>
        </tr>
      </thead>
      <tbody>
        {#each capture.rows as r}
          <tr>
            <td>{r.minute}</td>
            <td>{Math.round(r.hidden * 100)}%</td>
            <td class={r.frames / 60 >= 4 ? "" : "bad"}>{(r.frames / 60).toFixed(1)}</td>
            <td class={r.reads / 60 >= 3.6 ? "" : "bad"}>{(r.reads / 60).toFixed(2)}</td>
            <td>{r.readMean.toFixed(1)}</td>
            <td>{r.readP95.toFixed(1)}</td>
            <td>{r.copyMean.toFixed(1)}</td>
            <td>{Math.round(r.busy * 100)}%</td>
            <td>{r.pressure}</td>
          </tr>
        {/each}
      </tbody>
    </table>
    <p class="muted">
      Hidden: share of the minute's frames that came while the browser said this page couldn't be seen. CPU: the browser's
      pressure state where it reports one. Busy: time this page's worker spent copying and reading.
    </p>
  {/if}
</section>

<style>
  table {
    width: 100%;
    margin-top: 8px;
    border-collapse: collapse;
    font-variant-numeric: tabular-nums;
  }

  th {
    text-align: left;
    color: var(--muted);
    font-weight: 600;
  }

  th,
  td {
    padding: 3px 8px 3px 0;
    border-bottom: var(--line-w) solid var(--line);
  }
</style>
