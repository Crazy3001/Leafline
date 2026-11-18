<script lang="ts">
  // Your records as one file: to keep as a backup, or to move between the program, the website and
  // another computer. Opening one shows what it holds before anything changes, and the records it
  // replaces are kept aside.
  import { Download, Upload } from "@lucide/svelte";

  import { backend } from "$lib/api";
  import { isWeb } from "$lib/platform";
  import type { RecordsSummary } from "$lib/types";

  let note = $state<string | null>(null);
  let warn = $state(false);
  let picked = $state<RecordsSummary | null>(null);
  let busy = $state(false);

  function describe(s: RecordsSummary): string {
    const parts = [
      [s.sessions, "session"],
      [s.drops, "drop"],
      [s.builds, "build"],
      [s.notes, "note"],
      [s.spots, "saved spot"],
      [s.prices, "price"],
    ] as const;
    const said = parts.filter(([n]) => n > 0).map(([n, what]) => `${n} ${what}${n === 1 ? "" : "s"}`);
    return said.length ? said.join(", ") : "nothing yet";
  }

  async function run(work: () => Promise<void>) {
    busy = true;
    note = null;
    warn = false;
    try {
      await work();
    } catch (err) {
      note = err instanceof Error ? err.message : String(err);
      warn = true;
    } finally {
      busy = false;
    }
  }

  const save = () =>
    run(async () => {
      const saved = await backend.saveRecords();
      if (saved) note = `Saved ${describe(saved)}${isWeb ? " to your downloads" : ""}.`;
    });

  const pick = () =>
    run(async () => {
      picked = await backend.pickRecords();
    });

  const use = () =>
    run(async () => {
      await backend.useRecords();
      picked = null;
      // Every window reloads when the records change (see the root layout).
    });
</script>

<div class="row">
  <div class="label">
    <span class="name">Records file</span>
    <span class="help">
      Everything you recorded, in one file: sessions, drops, builds, notes, spots and prices. Keep it as a backup, or
      open it in the {isWeb ? "Windows program" : "website"} or on another computer. Map data isn't in it; each place
      downloads its own.
    </span>
  </div>
  <div class="control exports">
    <button class="button" onclick={save} disabled={busy}><Download size={13} />Save my records</button>
    <button class="button" onclick={pick} disabled={busy}><Upload size={13} />Open records…</button>
  </div>
</div>
{#if picked}
  <div class="note confirm" role="alertdialog" aria-label="Replace your records">
    <span>
      That file holds {describe(picked)}. Use it in place of the records here? The ones here now are kept aside, not
      deleted.
    </span>
    <button class="button" onclick={use} disabled={busy}>Use these records</button>
    <button class="button" onclick={() => (picked = null)} disabled={busy}>Cancel</button>
  </div>
{/if}
{#if note}<p class="note" class:warn>{note}</p>{/if}

<style>
  .confirm {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }
</style>
