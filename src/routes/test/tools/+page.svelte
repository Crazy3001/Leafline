<script lang="ts">
  // The program's Capture test window (website plan, stage 2), a developer check opened from
  // Settings in a dev build: the test pattern, saving the game's frames beside the website's, the
  // comparison, and the steps for running the test.
  import { onMount } from "svelte";
  import { isTauri } from "@tauri-apps/api/core";

  import AgreementView from "$lib/capture/AgreementView.svelte";
  import MeasureView from "$lib/capture/MeasureView.svelte";
  import TestShell from "$lib/capture/TestShell.svelte";
  import type { Agreement, Measure, RecorderStatus } from "$lib/capture/types";

  const program = !__LEAFLINE_WEB__ && isTauri() ? import("$lib/capture/program") : null;

  let folder = $state<string | null>(null);
  let perSecond = $state(2);
  let recorder = $state<RecorderStatus | null>(null);
  let check = $state<Measure | null>(null);
  let checking = $state(false);
  let report = $state<Agreement | null>(null);
  let comparing = $state<{ done: number; total: number } | null>(null);
  let problem = $state<string | null>(null);

  async function run<T>(job: (p: typeof import("$lib/capture/program")) => Promise<T>): Promise<T | undefined> {
    if (!program) return undefined;
    problem = null;
    try {
      return await job(await program);
    } catch (err) {
      problem = String(err);
      return undefined;
    }
  }

  onMount(() => {
    if (!program) return;
    const timer = setInterval(async () => {
      recorder = (await run((p) => p.recordingStatus())) ?? recorder;
    }, 1000);
    const stop = program.then((p) => p.onCompareProgress((done, total) => (comparing = { done, total })));
    return () => {
      clearInterval(timer);
      void stop.then((unlisten) => unlisten());
    };
  });

  async function pick() {
    folder = (await run((p) => p.pickFolder())) ?? folder;
  }

  async function checkPattern() {
    checking = true;
    check = (await run((p) => p.checkPattern())) ?? null;
    checking = false;
  }

  async function record(on: boolean) {
    const status = await run((p) => (on && folder ? p.startRecording(folder, perSecond) : p.stopRecording()));
    if (status) recorder = status;
  }

  async function compare() {
    if (!folder) return;
    report = null;
    comparing = { done: 0, total: 0 };
    report = (await run((p) => p.compareFrames(folder!))) ?? null;
    comparing = null;
  }
</script>

<TestShell title="Leafline — Capture test">
  <h1>Capture test</h1>
  <p class="lead">
    Whether the website could read the game from a window shared to Chrome or Edge (stage 2 of the website plan). A
    developer check: the program's half is here, the website's is its <code>/test/capture</code> page. Frames are saved
    only to the folder you pick, and nothing is sent anywhere.
  </p>
  {#if !program}
    <p class="status bad">This window is the program's; open it from the program's Settings.</p>
  {/if}
  {#if problem}<p class="status bad">{problem}</p>{/if}

  <section>
    <h2>A. Test pattern</h2>
    <p class="lead">
      The pattern window shows a test pattern pixel for pixel. Share it on the website's test page to see what the
      browser does to it. The check here reads it with the program's own capture first, which should be exact.
    </p>
    <div class="controls">
      <button class="button primary" onclick={() => run((p) => p.openPattern())}>Show the pattern</button>
      <button class="button" onclick={checkPattern} disabled={checking}>
        {checking ? "Checking…" : "Check it with the program's capture"}
      </button>
    </div>
    {#if check}<MeasureView measure={check} />{/if}
  </section>

  <section>
    <h2>B. Frames for the comparison</h2>
    <p class="lead">
      Saves the game window's frames as <code>app-&lt;time&gt;.png</code>, on the same clock ticks as the website's test
      page, into a folder you pick. Pick a folder outside OneDrive: these are pictures of your game.
    </p>
    <div class="controls">
      <button class="button" onclick={pick}>{folder ? "Change folder…" : "Pick a folder…"}</button>
      <select bind:value={perSecond} disabled={recorder?.recording}>
        <option value={1}>1 a second</option>
        <option value={2}>2 a second</option>
        <option value={4}>4 a second (Pickup Log)</option>
      </select>
      {#if recorder?.recording}
        <button class="button" onclick={() => record(false)}>Stop saving</button>
      {:else}
        <button class="button primary" onclick={() => record(true)} disabled={!folder}>Start saving</button>
      {/if}
      <button class="button" onclick={compare} disabled={!folder || !!comparing || recorder?.recording}>
        {comparing ? "Comparing…" : "Compare the frames"}
      </button>
    </div>
    {#if folder}<p class="status muted">{folder}</p>{/if}
    {#if recorder && (recorder.recording || recorder.saved || recorder.problem)}
      <p class="status">
        {recorder.recording ? `Saving ${recorder.perSecond} a second from “${recorder.window}”: ` : "Saved: "}
        {recorder.saved} frames{recorder.missed ? `, ${recorder.missed} missed` : ""}
        {#if recorder.problem}<span class="bad"> · {recorder.problem}</span>{/if}
      </p>
    {/if}
    {#if comparing}
      <p class="status">
        Reading the frames with the program's readers{comparing.total
          ? `: ${comparing.done} of ${comparing.total}`
          : "…"} (about a second a frame).
      </p>
    {/if}
    {#if report}
      <AgreementView {report} />
      <p class="status muted">The report is saved in the folder as <code>leafline-capture-report.json</code>.</p>
    {/if}
  </section>

  <section>
    <h2>How to run the test</h2>
    <p class="lead">On the alt account only, with Windows HDR off and the game at its usual size and UI Size.</p>
    <h3>Before you start</h3>
    <ol>
      <li>Make an empty folder outside OneDrive, such as <code>C:\LeaflineCaptureTest</code>.</li>
      <li>
        Open the website's test page in Chrome or Edge: <code>http://localhost:1433/Leafline/test/capture</code> after
        <code>npm.cmd run build:wasm</code> and <code>npm.cmd run dev:web</code> in a second terminal, or the live
        site's <code>/test/capture</code> once it is published.
      </li>
    </ol>
    <h3>A. Pattern (5 minutes, no game needed)</h3>
    <ol>
      <li>Here: Show the pattern, then Check it with the program's capture. Every number should be 0.</li>
      <li>
        On the test page: Share a window… and pick “Leafline capture test pattern”. Leave it uncovered for a few
        seconds, then Copy results and send them to Claude.
      </li>
      <li>If you have time, do the same in the other browser (Chrome and Edge).</li>
    </ol>
    <h3>B. Real game (about 15 minutes)</h3>
    <ol>
      <li>Log in on the alt. Live reading can stay on.</li>
      <li>Here: pick the folder, 2 a second, Start saving.</li>
      <li>
        Test page: Share a window… and pick the MapleStory window. Pick the same folder (allow Chrome to edit it), 2 a
        second, Start saving.
      </li>
      <li>
        Play these scenes for a minute or two each: standing in town; fighting on a hunting map; walking through two or
        three more maps; the inventory open with mesos showing; the Quest Helper showing while a kill quest counts; the
        cursor held still over four or five items for a few seconds each; the minimap open, then folded.
      </li>
      <li>
        For the Pickup Log: stop saving on both, switch both to 4 a second, start both again and pick up a pile of drops
        a few times.
      </li>
      <li>
        Stop saving on both, then Compare the frames here. It takes a few minutes. Send Claude the table, or the
        report file from the folder. The frames stay in the folder; delete them when we are done.
      </li>
    </ol>
    <h3>C. Endurance (15 minutes)</h3>
    <ol>
      <li>
        Test page: keep sharing the game window (saving stopped). Optionally open the browser's task manager
        (Shift+Esc) to see the tab's CPU.
      </li>
      <li>Endurance › Start. Click into the game and play normally for 15 minutes.</li>
      <li>Notice whether the game stutters or slows down at any point, and roughly when.</li>
      <li>Come back to the test page, Stop, then Copy the log and send it to Claude with your notes.</li>
    </ol>
    <h3>Pass bar</h3>
    <ol>
      <li>Level, EXP, mesos and map name agree on 99% or more of the pairs.</li>
      <li>After 10+ minutes behind the game, frames still arrive at each reader's rate (Pickup Log about 4 a second).</li>
      <li>No slowdown in the game.</li>
      <li>Quest Helper, item tooltips, Pickup Log and minimap each pass or fail on their own.</li>
    </ol>
  </section>
</TestShell>

<style>
  h3 {
    margin: 14px 0 2px;
    font-size: var(--text-sm, 13px);
  }
</style>
