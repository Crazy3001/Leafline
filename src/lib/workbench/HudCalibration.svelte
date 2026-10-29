<script lang="ts">
  import { GraduationCap, LoaderCircle } from "@lucide/svelte";

  import Card from "$lib/components/Card.svelte";
  import type { Calibration, HudReading, HudValues } from "$lib/types";

  let {
    hud,
    calibrate,
  }: {
    hud: HudReading;
    calibrate: (values: HudValues) => Promise<Calibration>;
  } = $props();

  // Start from whatever was read, so a partly calibrated HUD only needs the gaps filled.
  // svelte-ignore state_referenced_locally
  let fields = $state({
    level: hud.level?.toString() ?? "",
    name: hud.name ?? "",
    hp: hud.hp ? `${hud.hp.current}/${hud.hp.max}` : "",
    mp: hud.mp ? `${hud.mp.current}/${hud.mp.max}` : "",
    exp: hud.exp ? hud.exp.points.toLocaleString("en-US") : "",
    percent: hud.exp ? hud.exp.percent.toFixed(2) : "",
  });
  let busy = $state(false);
  let result = $state<Calibration | null>(null);
  let error = $state<string | null>(null);

  const whole = (s: string) => (/^\d+$/.test(s.trim()) ? Number(s.trim()) : null);
  const pair = (s: string): [number, number] | null => {
    const match = s.replace(/\s/g, "").match(/^(\d+)\/(\d+)$/);
    return match ? [Number(match[1]), Number(match[2])] : null;
  };

  const values = $derived.by((): HudValues | null => {
    const level = whole(fields.level);
    const hp = pair(fields.hp);
    const mp = pair(fields.mp);
    const expPoints = whole(fields.exp.replace(/[,.\s]/g, ""));
    const expPercent = fields.percent.trim().replace(/%$/, "");
    if (level === null || !hp || !mp || expPoints === null || !/^\d{1,3}([.,]\d{1,2})?$/.test(expPercent)) return null;
    return { level, hp, mp, expPoints, expPercent, name: fields.name.trim() || null };
  });

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    if (!values) return;
    busy = true;
    error = null;
    try {
      result = await calibrate(values);
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<Card title="Teach the HUD">
  {#snippet aside()}<GraduationCap size={14} />{/snippet}

  <p class="intro">
    The HUD's tiny font is read by matching shapes Leafline learns from your own screen. Type exactly what the HUD in
    this screenshot shows. The shapes stay on this PC.
  </p>

  <form onsubmit={submit}>
    <label class="wide">Character name<input bind:value={fields.name} placeholder="As shown under your job" spellcheck="false" /></label>
    <label>Level<input bind:value={fields.level} inputmode="numeric" placeholder="30" /></label>
    <label>HP<input bind:value={fields.hp} inputmode="numeric" placeholder="1149/1149" /></label>
    <label>MP<input bind:value={fields.mp} inputmode="numeric" placeholder="953/953" /></label>
    <label>EXP<input bind:value={fields.exp} inputmode="numeric" placeholder="24,331" /></label>
    <label>EXP %<input bind:value={fields.percent} inputmode="decimal" placeholder="25.42" /></label>
    <button class="primary" type="submit" disabled={!values || busy}>
      {#if busy}<span class="spin"><LoaderCircle size={14} /></span>{/if}
      Learn from this screenshot
    </button>
  </form>

  {#if error}
    <p class="message bad">{error}</p>
  {:else if result}
    <p class="message" class:bad={result.failed.length > 0}>
      {#if result.learned.length}Learned {result.learned.join(", ")}.{/if}
      {#if result.failed.length}
        Could not match {result.failed.join(", ")} to the screen; check for typos or something covering the HUD.
      {/if}
    </p>
  {/if}
</Card>

<style>
  .intro {
    margin: 0 0 10px;
    color: var(--muted);
    font-size: 12px;
    line-height: 1.45;
  }

  form {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
  }

  label {
    display: grid;
    gap: 3px;
    color: var(--faint);
    font-size: 11px;
  }

  .wide,
  button {
    grid-column: 1 / -1;
  }

  input {
    height: 30px;
    padding: 0 8px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--text);
    font: inherit;
    font-size: 13px;
    font-variant-numeric: tabular-nums;
  }

  input:focus {
    outline: none;
    border-color: var(--accent);
  }

  button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 7px;
    height: 32px;
    margin-top: 2px;
    border: 0;
    border-radius: var(--radius-sm);
    background: var(--accent);
    color: #1d1405;
    font-weight: 600;
    cursor: pointer;
  }

  button:hover:not(:disabled) {
    background: var(--accent-strong);
  }

  button:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .spin {
    display: inline-flex;
    animation: spin 0.9s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .message {
    margin: 10px 0 0;
    color: var(--good);
    font-size: 12px;
  }

  .message.bad {
    color: var(--warn);
  }
</style>
