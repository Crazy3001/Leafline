<script lang="ts">
  // Part B's results: how far the program's readers agree on the program's frames and the
  // website's, pair by pair.
  import { percent, type Agreement } from "./types";

  let { report }: { report: Agreement } = $props();

  const fitted = $derived(
    report.websiteResized
      ? "resized to the program's size (the browser delivered them smaller)"
      : report.websiteOffset && (report.websiteOffset[0] || report.websiteOffset[1])
        ? `cut at ${report.websiteOffset[0]}, ${report.websiteOffset[1]} (the browser kept a title bar or border)`
        : "the same size as the program's",
  );
</script>

<p class={report.passes ? "verdict good" : "verdict bad"}>
  {report.passes
    ? "Pass bar met: level, EXP, mesos and map name agree on 99% or more of the pairs."
    : "Pass bar not met: level, EXP, mesos and map name each need 99% or more."}
</p>
<p class="note">
  {report.programFrames} program frames, {report.websiteFrames} website frames, {report.pairs} pairs within 0.1 s. Website
  frames were {fitted}.
</p>

<table>
  <thead>
    <tr><th>Reader</th><th>Agree</th><th>Same</th><th>Differ</th><th>Only program</th><th>Only website</th><th>Neither</th></tr>
  </thead>
  <tbody>
    {#each report.readers as r}
      <tr>
        <td>{r.reader}{r.gate ? " *" : ""}</td>
        <td class="num {r.passes == null ? '' : r.passes ? 'good' : 'bad'}">{percent(r.share)}</td>
        <td class="num">{r.agree}</td>
        <td class="num">{r.differ}</td>
        <td class="num">{r.onlyProgram}</td>
        <td class="num">{r.onlyWebsite}</td>
        <td class="num">{r.neither}</td>
      </tr>
    {/each}
    <tr>
      <td>Pickup Log</td>
      <td class="num {report.pickup.passes == null ? '' : report.pickup.passes ? 'good' : 'bad'}">{percent(report.pickup.share)}</td>
      <td class="num" colspan="5">
        {report.pickup.matched} lines alike of {report.pickup.programLines} (program) and {report.pickup.websiteLines} (website)
      </td>
    </tr>
  </tbody>
</table>
<p class="hint">
  * Part of the pass bar. Agree is over pairs where either side read something; every reader needs 99% to pass on its own.
</p>

{#each report.readers.filter((r) => r.examples.length) as r}
  <details>
    <summary>{r.reader}: {r.examples.length} pairs read differently</summary>
    <table>
      <thead><tr><th>Program frame</th><th>Program read</th><th>Website read</th></tr></thead>
      <tbody>
        {#each r.examples as e}
          <tr>
            <td class="num">{e.programFrame}<br /><span class="muted">{e.websiteFrame}</span></td>
            <td>{e.program ?? "nothing"}</td>
            <td>{e.website ?? "nothing"}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  </details>
{/each}

<style>
  .verdict {
    margin: 0 0 var(--space-1-5);
    font-weight: var(--weight-strong);
  }

  .note {
    margin: 0 0 var(--space-2);
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--text-md);
  }

  th {
    text-align: left;
    font-weight: var(--weight-strong);
    color: var(--muted);
  }

  th,
  td {
    padding: var(--space-1) var(--space-2) var(--space-1) 0;
    border-bottom: var(--line-w) solid var(--line);
    vertical-align: top;
  }

  .num {
    font-variant-numeric: tabular-nums;
  }

  .hint,
  .muted {
    color: var(--muted);
  }

  .hint {
    font-size: var(--text-xs);
  }

  details {
    margin-top: var(--space-2);
  }

  summary {
    cursor: pointer;
  }

  .good {
    color: var(--good);
  }

  .bad {
    color: var(--bad);
  }
</style>
