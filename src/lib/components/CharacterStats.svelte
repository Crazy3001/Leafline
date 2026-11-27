<script lang="ts">
  // Character → Stats: every number the game's Character Stat window and its detail pane showed at
  // the last read, and the buffs the player says are on. With buffs on, each number they change
  // shows as it is with them, the number read beside it. The training estimates use the same.
  import { can } from "$lib/platform";
  import { Info, RefreshCw } from "@lucide/svelte";
  import { onMount, untrack } from "svelte";

  import Ref from "./Ref.svelte";
  import { backend } from "$lib/api";
  import { damage, followDamage, setBuff } from "$lib/damage.svelte";
  import { formatAgo, formatNumber } from "$lib/format";
  import { keyHint } from "$lib/settings.svelte";
  import { tip } from "$lib/tip.svelte";
  import type { BuffRow, SkillEffects, StatSheet } from "$lib/types";
  import Switch from "$lib/ui/Switch.svelte";

  let {
    character,
    showRead = true,
  }: {
    character: string | null;
    /** The build's header has its own Read button. */
    showRead?: boolean;
  } = $props();

  $effect(() => {
    const name = character;
    untrack(() => followDamage(name));
  });

  const stats = $derived(damage.name && character && damage.name.toLowerCase() === character.toLowerCase() ? damage.stats : null);
  /** The numbers without any buff; with buffs on, each one they change shows its new number beside. */
  const sheet = $derived(stats?.base ?? stats?.sheet ?? null);
  const buffed = $derived(stats?.buffed ?? null);
  const buffs = $derived(stats?.buffs ?? []);

  // "Read 4m ago" moves on by itself.
  let clock = $state(Date.now() / 1000);
  onMount(() => {
    const timer = setInterval(() => (clock = Date.now() / 1000), 30_000);
    return () => clearInterval(timer);
  });
  const readAgo = $derived(stats?.readAt != null ? formatAgo(clock - stats.readAt) : null);

  let reading = $state(false);
  let problem = $state<string | null>(null);
  onMount(() => {
    const stop = backend.onCharacterRead((read) => {
      reading = false;
      problem = read.problem && !read.stats ? read.problem : null;
    });
    return () => void stop.then((unlisten) => unlisten());
  });

  function read() {
    reading = true;
    problem = null;
    backend.readCharacter().catch((e) => {
      reading = false;
      problem = String(e);
    });
  }

  type Key = keyof StatSheet;
  type Cell = { label: string; key: Key; suffix?: string };

  /** What decides a fight, beside the attack range. */
  const FIGHT: Cell[] = [
    { label: "Magic", key: "magic" },
    { label: "Accuracy", key: "accuracy" },
    { label: "Evasion", key: "evasion" },
    { label: "Crit. rate", key: "critRate", suffix: "%" },
    { label: "Crit. damage", key: "critDamage", suffix: "%" },
  ];
  /** The rest of the window, folded away until asked for. */
  const MORE: Cell[] = [
    { label: "Max HP", key: "maxHp" },
    { label: "Max MP", key: "maxMp" },
    { label: "Fame", key: "fame" },
    { label: "Weapon Def.", key: "weaponDef" },
    { label: "Magic Def.", key: "magicDef" },
    { label: "Speed", key: "speed", suffix: "%" },
    { label: "Jump", key: "jump", suffix: "%" },
  ];
  const BUFF_HELP =
    "Turn on the ones you have running. Each counts at its level. When the Character Stat window shows a buff in red, its switch follows.";

  const BASE: { label: string; key: "str" | "dex" | "int" | "luk"; gear: "gearStr" | "gearDex" | "gearInt" | "gearLuk" }[] = [
    { label: "STR", key: "str", gear: "gearStr" },
    { label: "DEX", key: "dex", gear: "gearDex" },
    { label: "INT", key: "int", gear: "gearInt" },
    { label: "LUK", key: "luk", gear: "gearLuk" },
  ];

  const value = (s: StatSheet | null, key: Key) => (s?.[key] ?? null) as number | null;
  /** The number with the buffs on now, when they change it. */
  const changed = (key: Key) => {
    const now = value(buffed, key);
    return now != null && now !== value(sheet, key) ? now : null;
  };
  const attackChanged = $derived(
    buffed != null && (buffed.attackMin !== sheet?.attackMin || buffed.attackMax !== sheet?.attackMax),
  );

  const EFFECT_NAMES: [keyof SkillEffects, string, string][] = [
    ["attack", "Attack", ""],
    ["magic", "Magic", ""],
    ["weaponDef", "Weapon Def.", ""],
    ["weaponDefPercent", "Weapon Def.", "%"],
    ["magicDef", "Magic Def.", ""],
    ["accuracy", "Accuracy", ""],
    ["evasion", "Evasion", ""],
    ["speed", "Speed", ""],
    ["jump", "Jump", ""],
  ];

  /** What a buff changes at its level, in the window's words: `Accuracy +15, Evasion +15`. */
  function effectText(b: BuffRow): string | null {
    const parts = EFFECT_NAMES.filter(([key]) => b.effects[key] !== 0).map(
      ([key, name, unit]) => `${name} ${b.effects[key] > 0 ? "+" : ""}${b.effects[key]}${unit}`,
    );
    return parts.length ? parts.join(", ") : null;
  }

  const anyOn = $derived(buffs.some((b) => b.on));
</script>

{#snippet pair(cell: Cell)}
  {@const read = value(sheet, cell.key)}
  {@const now = changed(cell.key)}
  <div>
    <dt>{cell.label}</dt>
    <dd class="tabular">
      {#if read == null}
        —
      {:else if now != null}
        <span class:up={now > read} class:down={now < read}>{formatNumber(now)}{cell.suffix ?? ""}</span>
        <span class="small">{formatNumber(read)}{cell.suffix ?? ""}</span>
      {:else}
        {formatNumber(read)}{cell.suffix ?? ""}
      {/if}
    </dd>
  </div>
{/snippet}

<section class="sheet" aria-label="Character stats">
  <div class="top">
    <span class="when">
      {#if readAgo}Read {readAgo}{:else}Not read yet{/if}
    </span>
    {#if showRead && can.liveReading}<button
      class="link"
      disabled={reading}
      use:tip={`Reads your Character Stat window, with Detail open, and your Skill window${keyHint("ReadCharacter")}`}
      onclick={read}><RefreshCw size={12} />{reading ? "Reading…" : "Read"}</button
    >{/if}
  </div>
  {#if problem}<p class="problem">{problem}</p>{/if}

  {#if !sheet}
    <p class="quiet-line">
      {#if can.liveReading}
        Open your Character Stat window in game and press Detail. Leafline reads it while it's open.
      {:else}
        Share the game window, then open your Character Stat window and press Detail.
      {/if}
    </p>
  {:else}
    {#if anyOn && buffed}
      <p class="quiet-line">With your buffs on. Each number they change has the one without them beside it.</p>
    {/if}

    <!-- The four stats you put AP into, with what your gear adds, and the AP still to spend. -->
    <dl class="main">
      {#each BASE as stat (stat.key)}
        {@const own = sheet[stat.key]}
        {@const gear = sheet[stat.gear]}
        <div>
          <dt>{stat.label}</dt>
          <dd class="num">{own == null ? "—" : own + (gear ?? 0)}</dd>
          {#if own != null && gear}<span class="small gear" use:tip={`${own} you put in, +${gear} from gear`}>+{gear} gear</span>{/if}
        </div>
      {/each}
      <div class="ap" class:spend={(sheet.abilityPoints ?? 0) > 0}>
        <dt>AP left</dt>
        <dd class="num">{sheet.abilityPoints ?? "—"}</dd>
      </div>
    </dl>

    <dl class="pairs">
      <div>
        <dt>Attack</dt>
        <dd class="tabular">
          {#if sheet.attackMin == null || sheet.attackMax == null}
            —
          {:else if attackChanged && buffed}
            <span class="up">{formatNumber(buffed.attackMin)} – {formatNumber(buffed.attackMax)}</span>
            <span class="small">{formatNumber(sheet.attackMin)} – {formatNumber(sheet.attackMax)}</span>
          {:else}
            {formatNumber(sheet.attackMin)} – {formatNumber(sheet.attackMax)}
          {/if}
        </dd>
      </div>
      {#each FIGHT as cell (cell.key)}{@render pair(cell)}{/each}
    </dl>

    <details class="more">
      <summary>More stats</summary>
      <dl class="pairs">
        {#each MORE as cell (cell.key)}{@render pair(cell)}{/each}
      </dl>
    </details>
  {/if}

  <h4 class="sub">Buffs{#if buffs.length}<span class="help" use:tip={BUFF_HELP}><Info size={12} aria-label="How buffs count" /></span>{/if}</h4>
  {#if !buffs.length}
    <p class="quiet-line">
      {#if sheet}
        No buffs known yet. Open your Skill window, or pick a job in your build.
      {:else}
        Your buffs show here once your Skill window is read or your build has a job.
      {/if}
    </p>
  {:else}
    <ul class="buffs">
      {#each buffs as b (b.id)}
        {@const effect = effectText(b)}
        <li class:off={!b.on}>
          <Switch checked={b.on} label="{b.name} on" disabled={b.level == null} onchange={(on) => setBuff(b.id, on)} />
          <span class="buff-name"><Ref kind="skill" id={b.id} name={b.name} /></span>
          <span class="buff-level tabular">
            {#if b.level == null}
              <span class="faint">Lv ?</span>
            {:else}
              Lv {b.level}{#if !b.levelRead}<span class="faint"> planned</span>{/if}
            {/if}
          </span>
          <span class="buff-effect">
            {#if b.level == null}
              <span class="faint">Open your Skill window to read its level</span>
            {:else if effect}
              {effect}{#if b.effects.seconds}<span class="faint">{" · "}{Math.round(b.effects.seconds / 60) || 1} min</span>{/if}
            {:else}
              <span class="faint">Changes no stat the window shows</span>
            {/if}
          </span>
        </li>
      {/each}
    </ul>
    {#if stats?.attackUnscaled}
      <p class="quiet-line">
        Your attack range stays as read: Leafline needs your weapon's Attack to count a buff's. Put your weapon on in Gear.
      </p>
    {/if}
  {/if}
</section>

<style>
  .sheet {
    display: grid;
    gap: var(--space-0-5);
  }

  .top {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-bottom: var(--space-1);
  }

  .when {
    color: var(--muted);
    font-size: var(--text-md);
  }

  .link {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    margin-left: auto;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--muted);
    font-size: var(--text-md);
    cursor: pointer;
  }

  .link:hover:not(:disabled),
  .link:focus-visible {
    color: var(--text);
    outline: none;
  }

  .link:disabled {
    cursor: default;
    opacity: 0.6;
  }

  .problem {
    margin: 0 0 var(--space-1);
    color: var(--warn);
    font-size: var(--text-md);
  }

  .quiet-line {
    margin: 0 0 var(--space-1);
    color: var(--faint);
    font-size: var(--text-md);
  }

  /* The four stats you raise, big enough to read at a glance, and the AP to spend. */
  .main {
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    margin: var(--space-1-5) 0 var(--space-2);
  }

  .main div {
    display: flex;
    flex-direction: column;
    min-width: 0;
    padding: 0 var(--space-2);
    border-left: 1px solid var(--border);
  }

  .main div:first-child {
    padding-left: 0;
    border-left: 0;
  }

  .main dd {
    font-size: var(--text-xl);
    line-height: 1.2;
  }

  .main .gear {
    cursor: help;
  }

  /* AP waiting to be spent: the one thing here to act on. */
  .ap.spend dd,
  .ap.spend dt {
    color: var(--accent-text);
  }

  /* Label and number on one line, split by rules: no box per number. */
  .pairs {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    column-gap: var(--space-5);
    margin: 0;
  }

  .pairs div {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--space-2);
    min-width: 0;
    padding: var(--space-1-5) 0;
    border-top: 1px solid var(--border);
  }

  .pairs dt {
    font-size: var(--text-md);
    color: var(--muted);
  }

  .more {
    margin-top: var(--space-1);
  }

  .more summary {
    padding: var(--space-1-5) 0;
    color: var(--faint);
    font-size: var(--text-md);
    cursor: pointer;
  }

  .more summary:hover {
    color: var(--text);
  }

  .help {
    display: inline-flex;
    margin-left: var(--space-1-5);
    color: var(--faint);
    vertical-align: -1px;
    cursor: help;
  }

  dt {
    color: var(--faint);
    font-size: var(--text-xs);
  }

  dd {
    margin: 0;
    font-weight: var(--weight-strong);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .small {
    color: var(--faint);
    font-size: var(--text-xs);
    font-weight: var(--weight-body);
  }

  .up {
    color: var(--good);
  }

  .down {
    color: var(--bad);
  }

  .faint {
    color: var(--faint);
  }

  .sub {
    margin: var(--space-3) 0 var(--space-1-5);
    font-family: var(--font-display);
    font-size: var(--text-md);
    font-weight: var(--weight-body);
    color: var(--muted);
  }

  .buffs {
    display: grid;
    gap: 0;
    margin: var(--space-1) 0 0;
    padding: 0;
    list-style: none;
  }

  .buffs li {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: center;
    column-gap: var(--space-2);
    row-gap: 1px;
    padding: var(--space-1-5) 0;
    border-top: 1px solid var(--border);
    font-size: var(--text-md);
  }

  .buffs li.off .buff-effect {
    color: var(--muted);
  }

  .buff-name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .buff-level {
    color: var(--muted);
  }

  .buff-effect {
    grid-column: 2 / -1;
    font-size: var(--text-xs);
  }
</style>
