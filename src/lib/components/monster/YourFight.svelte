<script lang="ts">
  // How a fight with this monster goes for the character: the chance a blow lands, what it does
  // after the monster's defence and the level penalty, and how many attacks a kill takes, with how
  // often each count comes up. From the combat model the training estimates use: community
  // formulas, with the defence part fitted to the player's own play once there is enough of it.
  import { untrack } from "svelte";

  import { backend } from "$lib/api";
  import { assumptions, watchAssumptions } from "$lib/assumptions.svelte";
  import { characters } from "$lib/characters.svelte";
  import { damage, effective, followDamage } from "$lib/damage.svelte";
  import { formatNumber } from "$lib/format";
  import { measuredRates, watchHistory } from "$lib/history.svelte";
  import { live } from "$lib/readings.svelte";
  import type { MonsterFight, SessionSummary } from "$lib/types";

  let {
    id,
    level = null,
    session = null,
  }: {
    id: number;
    level?: number | null;
    session?: SessionSummary | null;
  } = $props();

  watchHistory();
  watchAssumptions();

  const name = $derived(session?.name ?? live.analysis?.hud?.name ?? characters.active);
  $effect(() => {
    const who = name;
    untrack(() => followDamage(who));
  });
  const used = $derived(effective(assumptions.value, damage.value, damage.stats));
  const lv = $derived(level ?? session?.level ?? null);

  let fight = $state<MonsterFight | null>(null);
  let asked = $state(false);
  const key = $derived(JSON.stringify([id, name, lv, used.assumptions, assumptions.value.skillName, damage.value, damage.stats?.buffed]));

  // Follows the key alone: every live reading brings new objects with the same content.
  $effect(() => {
    const k = key;
    untrack(() =>
      backend.monsterFight(id, name, lv, measuredRates(session), used.assumptions, assumptions.value.skillName || null),
    )
      .then((result) => {
        if (k === key) fight = result;
      })
      .catch(() => (fight = null))
      .finally(() => (asked = true));
  });

  const pct = (x: number) => `${Math.round(x * 100)}%`;
  const hits = (n: number) => `${n} ${n === 1 ? "attack" : "attacks"}`;
  const widest = $derived(Math.max(...(fight?.fight.spread.map(([, s]) => s) ?? [0]), 0.01));
</script>

{#if fight}
  {@const f = fight.fight}
  <dl class="facts">
    <div>
      <dt>Attacks to kill</dt>
      <dd class="tabular">{f.spread.length ? f.attacks.toFixed(1) : "too many"}</dd>
    </div>
    <div>
      <dt>Blows land</dt>
      <dd class="tabular" class:warn={f.hit < 0.9}>{pct(f.hit)}</dd>
    </div>
    <div>
      <dt>A blow does</dt>
      <dd class="tabular">{formatNumber(f.blowLow)}–{formatNumber(f.blowHigh)}</dd>
    </div>
    <div>
      <dt>Attacking a kill</dt>
      <dd class="tabular">{fight.attackSecondsPerKill.toFixed(1)} s</dd>
    </div>
  </dl>

  {#if f.spread.length > 1}
    <ul class="spread" aria-label="How many attacks kills take">
      {#each f.spread as [n, share] (n)}
        <li>
          <span class="n tabular">{hits(n)}</span>
          <span class="bar"><span style:width="{(share / widest) * 100}%"></span></span>
          <span class="share tabular">{pct(share)}</span>
        </li>
      {/each}
    </ul>
  {/if}

  <p class="why">
    {fight.fighter.magic ? "Spells" : "Blows"} of {formatNumber(Math.round(fight.fighter.low))}–{formatNumber(
      Math.round(fight.fighter.high),
    )}{fight.fighter.lines > 1 ? `, ${fight.fighter.lines} lines an attack` : ""}{fight.fighter.critRate
      ? `, ${pct(fight.fighter.critRate)} critical`
      : ""}.
    {#if f.levelFactor < 0.999}It is above your level, which leaves {pct(f.levelFactor)} of each blow.{/if}
    {#if f.defenceDivisor > 1.001}Its defence divides each blow by {f.defenceDivisor.toFixed(2)}{fight.fitted
        ? ", as fitted to your own fights"
        : ", a starting guess until you have played a few maps"}.{/if}
    The formulas are the community's (mapleclassic.wiki, pre-launch); change your attack or skill in Train →
    Assumptions.
  </p>
{:else if asked}
  <p class="why">
    Read your Character Stat window with Detail open, or type your attack range in Train → Where to train →
    Assumptions, to see how a fight with it goes.
  </p>
{/if}

<style>
  .facts {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(96px, 1fr));
    gap: var(--space-1-5);
    margin: 0;
  }

  .facts div {
    padding: var(--space-1) var(--space-2);
    border-radius: var(--radius-sm);
    background: var(--hover);
  }

  dt {
    color: var(--faint);
    font-size: var(--text-xs);
  }

  dd {
    margin: 0;
    font-weight: var(--weight-strong);
  }

  dd.warn {
    color: var(--warn);
  }

  .spread {
    display: grid;
    gap: var(--space-0-5);
    margin: var(--space-2) 0 0;
    padding: 0;
    list-style: none;
  }

  .spread li {
    display: grid;
    grid-template-columns: 74px 1fr 36px;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-xs);
  }

  .n {
    color: var(--muted);
  }

  .bar {
    height: 6px;
    overflow: hidden;
    border-radius: var(--radius-pill);
    background: var(--hover);
  }

  .bar span {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: var(--marker-mob);
  }

  .share {
    color: var(--faint);
    text-align: right;
  }

  .why {
    margin: var(--space-2) 0 0;
    color: var(--faint);
    font-size: var(--text-xs);
  }
</style>
