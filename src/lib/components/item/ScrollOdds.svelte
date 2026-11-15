<script lang="ts">
  // The odds of scrolling gear: how likely the target is, how likely the item breaks, what it ends
  // with and what one good item costs. Starts from the item or scroll it is shown for; every number
  // can be changed, since Classic's rules may differ from the community's.
  import { costPerSuccess, destroyChanceFor, scrollOdds } from "$lib/scrolling";
  import { formatMesos } from "$lib/format";

  let {
    slots = 7,
    rate = null,
    stat = 1,
    statLabel = null,
  }: {
    /** Slots to start from: the item's own, or what the read copy has left. */
    slots?: number;
    /** The scroll's success rate in percent, when the calculator is for one scroll. */
    rate?: number | null;
    /** What one pass adds, when the scroll says. */
    stat?: number;
    statLabel?: string | null;
  } = $props();

  const RATES = [10, 30, 60, 70, 100];

  // Seeded from the props once; the player's changes stand until the page changes item.
  let slotCount = $state(7);
  let percent = $state(60);
  let perPass = $state(1);
  let destroyPct = $state(0);
  let target = $state(5);
  let stop = $state(false);
  let scrollPrice = $state<number | null>(null);
  let itemPrice = $state<number | null>(null);

  $effect(() => {
    slotCount = Math.max(1, slots);
    percent = rate ?? 60;
    perPass = stat || 1;
    destroyPct = Math.round(destroyChanceFor((rate ?? 60) / 100) * 100);
    target = Math.max(1, Math.min(slots, Math.round(slots * 0.7)));
  });

  function pickRate(p: number) {
    percent = p;
    destroyPct = Math.round(destroyChanceFor(p / 100) * 100);
  }

  const odds = $derived(
    scrollOdds({ slots: slotCount, rate: percent / 100, destroy: destroyPct / 100, stat: perPass, target, stopAtTarget: stop }),
  );
  const cost = $derived(
    scrollPrice != null && itemPrice != null ? costPerSuccess(odds, scrollPrice, itemPrice) : null,
  );
  const tallest = $derived(Math.max(...odds.passes, 0.0001));
  const pct = (x: number) => `${(x * 100).toFixed(x < 0.1 ? 1 : 0)}%`;
  const num = (v: string) => (v.trim() === "" ? null : Math.max(0, Number(v)));
</script>

<div class="calc">
  <div class="inputs">
    <label>
      <span>Slots</span>
      <input class="tabular" type="number" min="1" max="20" value={slotCount} oninput={(e) => (slotCount = Math.max(1, Math.min(20, Number(e.currentTarget.value) || 1)))} />
    </label>
    {#if rate == null}
      <div class="rates" role="group" aria-label="Scroll success rate">
        {#each RATES as r (r)}
          <button aria-pressed={percent === r} onclick={() => pickRate(r)}>{r}%</button>
        {/each}
      </div>
    {:else}
      <label>
        <span>Passes</span>
        <input class="tabular" type="number" min="0" max="100" value={percent} oninput={(e) => (percent = Math.max(0, Math.min(100, Number(e.currentTarget.value) || 0)))} />
      </label>
    {/if}
    <label>
      <span>{statLabel ? `${statLabel} a pass` : "Stat a pass"}</span>
      <input class="tabular" type="number" min="0" value={perPass} oninput={(e) => (perPass = Math.max(0, Number(e.currentTarget.value) || 0))} />
    </label>
    <label>
      <span>Breaks on a fail</span>
      <input class="tabular" type="number" min="0" max="100" value={destroyPct} oninput={(e) => (destroyPct = Math.max(0, Math.min(100, Number(e.currentTarget.value) || 0)))} />
    </label>
    <label>
      <span>Passes wanted</span>
      <input class="tabular" type="number" min="1" max={slotCount} value={target} oninput={(e) => (target = Math.max(1, Math.min(slotCount, Number(e.currentTarget.value) || 1)))} />
    </label>
    <label>
      <span>Scroll price</span>
      <input class="tabular" type="number" min="0" placeholder="mesos" value={scrollPrice ?? ""} oninput={(e) => (scrollPrice = num(e.currentTarget.value))} />
    </label>
    <label>
      <span>Item price</span>
      <input class="tabular" type="number" min="0" placeholder="mesos" value={itemPrice ?? ""} oninput={(e) => (itemPrice = num(e.currentTarget.value))} />
    </label>
    <label class="check">
      <input type="checkbox" bind:checked={stop} />
      <span>Stop once there</span>
    </label>
  </div>

  <dl class="facts">
    <div><dt>Reach {target}</dt><dd class="tabular good">{pct(odds.reach)}</dd></div>
    <div><dt>Item breaks</dt><dd class="tabular" class:bad={odds.destroyed > 0}>{pct(odds.destroyed)}</dd></div>
    <div><dt>Ends with, if it lasts</dt><dd class="tabular">+{odds.expectedStat.toFixed(1)}</dd></div>
    <div><dt>Scrolls used</dt><dd class="tabular">{odds.scrollsUsed.toFixed(1)}</dd></div>
    {#if cost != null}
      <div class="wide">
        <dt>One item that gets there costs</dt>
        <dd class="tabular mesos">{Number.isFinite(cost) ? `${formatMesos(cost)} mesos` : "it can't get there"}</dd>
      </div>
    {/if}
  </dl>

  <ul class="passes" aria-label="Chance of each number of passes">
    {#each odds.passes as p, k (k)}
      <li class:hit={k >= target}>
        <span class="k tabular">{k}</span>
        <span class="bar"><span style:width="{(p / tallest) * 100}%"></span></span>
        <span class="p tabular">{pct(p)}</span>
      </li>
    {/each}
  </ul>
  <p class="note">
    Every attempt uses a slot, pass or fail. A failed 30% or 70% scroll breaking the item half the time is the
    community's figure; check it in game and change it above if Classic differs.
  </p>
</div>

<style>
  .inputs {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(100px, 1fr));
    gap: 6px 8px;
    align-items: end;
  }

  label {
    display: grid;
    gap: 2px;
    color: var(--faint);
    font-size: var(--text-xs);
  }

  label.check {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--muted);
  }

  input[type="number"] {
    width: 100%;
    height: 26px;
    padding: 0 6px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--text);
  }

  .rates {
    display: inline-flex;
    grid-column: span 2;
    gap: 2px;
    padding: 2px;
    border-radius: var(--radius-sm);
    background: rgba(255, 255, 255, 0.04);
  }

  .rates button {
    flex: 1;
    padding: 3px 0;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--muted);
    font-size: var(--text-xs);
    cursor: pointer;
  }

  .rates button[aria-pressed="true"] {
    background: var(--accent-soft);
    color: var(--text);
  }

  .facts {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(96px, 1fr));
    gap: 6px;
    margin: 10px 0 0;
  }

  .facts div {
    padding: 4px 7px;
    border-radius: var(--radius-sm);
    background: rgba(255, 255, 255, 0.03);
  }

  .facts .wide {
    grid-column: 1 / -1;
  }

  dt {
    color: var(--faint);
    font-size: var(--text-xs);
  }

  dd {
    margin: 0;
    font-weight: 600;
  }

  dd.good {
    color: var(--good);
  }

  dd.bad {
    color: var(--bad);
  }

  dd.mesos {
    color: var(--mesos);
  }

  .passes {
    display: grid;
    gap: 2px;
    margin: 10px 0 0;
    padding: 0;
    list-style: none;
  }

  .passes li {
    display: grid;
    grid-template-columns: 18px 1fr 44px;
    align-items: center;
    gap: 8px;
    font-size: var(--text-xs);
  }

  .k {
    color: var(--muted);
    text-align: right;
  }

  .bar {
    height: 6px;
    overflow: hidden;
    border-radius: var(--radius-pill);
    background: rgba(255, 255, 255, 0.05);
  }

  .bar span {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: var(--muted);
  }

  .hit .bar span {
    background: var(--good);
  }

  .p {
    color: var(--faint);
    text-align: right;
  }

  .note {
    margin: 8px 0 0;
    color: var(--faint);
    font-size: var(--text-xs);
  }
</style>
