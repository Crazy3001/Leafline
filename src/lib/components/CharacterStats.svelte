<script lang="ts">
  // Character → Stats: every number the game's Character Stat window and its detail pane showed at
  // the last read, and the buffs the player says are on. With buffs on, each number they change
  // shows as it is with them, the number read beside it. The training estimates use the same.
  import { RefreshCw } from "@lucide/svelte";
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
  const sheet = $derived(stats?.sheet ?? null);
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

  const GROUPS: Cell[][] = [
    [
      { label: "Max HP", key: "maxHp" },
      { label: "Max MP", key: "maxMp" },
      { label: "Fame", key: "fame" },
      { label: "AP left", key: "abilityPoints" },
    ],
    [
      { label: "Weapon Def.", key: "weaponDef" },
      { label: "Magic", key: "magic" },
      { label: "Magic Def.", key: "magicDef" },
      { label: "Accuracy", key: "accuracy" },
      { label: "Evasion", key: "evasion" },
      { label: "Crit. rate", key: "critRate", suffix: "%" },
      { label: "Crit. damage", key: "critDamage", suffix: "%" },
      { label: "Speed", key: "speed", suffix: "%" },
      { label: "Jump", key: "jump", suffix: "%" },
    ],
  ];

  const BASE: { label: string; key: "str" | "dex" | "int" | "luk"; gear: "gearStr" | "gearDex" | "gearInt" | "gearLuk" }[] = [
    { label: "STR", key: "str", gear: "gearStr" },
    { label: "DEX", key: "dex", gear: "gearDex" },
    { label: "INT", key: "int", gear: "gearInt" },
    { label: "LUK", key: "luk", gear: "gearLuk" },
  ];

  const value = (s: StatSheet | null, key: Key) => (s?.[key] ?? null) as number | null;
  /** The number with buffs on, when they change it. */
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

  const onAtRead = $derived(buffs.filter((b) => b.onAtRead).map((b) => b.name));
  const anyOn = $derived(buffs.some((b) => b.on));
</script>

<section class="sheet" aria-label="Character stats">
  <div class="top">
    <span class="when">
      {#if readAgo}Read {readAgo}{:else}Not read yet{/if}
    </span>
    {#if showRead}<button
      class="link"
      disabled={reading}
      use:tip={`Reads your Character Stat window, with Detail open, and your Skill window${keyHint("ReadCharacter")}`}
      onclick={read}><RefreshCw size={12} />{reading ? "Reading…" : "Read"}</button
    >{/if}
  </div>
  {#if problem}<p class="problem">{problem}</p>{/if}

  {#if !sheet}
    <p class="quiet-line">
      Open your Character Stat window in the game and press Detail, then Read. Leafline also reads it by itself while it is open.
    </p>
  {:else}
    {#if anyOn && buffed}
      <p class="quiet-line">With your buffs on. The number read is beside each one they change.</p>
    {/if}
    {#if onAtRead.length}
      <p class="quiet-line">Read with {onAtRead.join(", ")} on, so {onAtRead.length === 1 ? "it is" : "they are"} already in the numbers read.</p>
    {/if}

    <dl class="grid four">
      {#each GROUPS[0] as cell (cell.key)}
        {@const read = value(sheet, cell.key)}
        <div>
          <dt>{cell.label}</dt>
          <dd class="tabular">{read == null ? "—" : formatNumber(read)}</dd>
        </div>
      {/each}
    </dl>

    <dl class="grid four">
      {#each BASE as stat (stat.key)}
        {@const own = sheet[stat.key]}
        {@const gear = sheet[stat.gear]}
        <div>
          <dt>{stat.label}</dt>
          <dd class="tabular">
            {#if own == null}—{:else if gear}{own + gear} <span class="small">({own}+{gear})</span>{:else}{own}{/if}
          </dd>
        </div>
      {/each}
    </dl>

    <dl class="grid">
      <div class="wide">
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
      {#each GROUPS[1] as cell (cell.key)}
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
      {/each}
    </dl>

  {/if}

  <h4 class="sub">Buffs</h4>
  {#if !buffs.length}
    <p class="quiet-line">
      {#if sheet}
        No buffs known yet. Open your Skill window, or choose a job in your build, and your buffs show here.
      {:else}
        Your buffs show here once your Skill window has been read or your build has a job.
      {/if}
    </p>
  {:else}
    <p class="quiet-line">Turn on the ones you have running. Each counts at its level.</p>
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
    gap: 2px;
  }

  .top {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 4px;
  }

  .when {
    color: var(--muted);
    font-size: var(--text-sm);
  }

  .link {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    margin-left: auto;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--muted);
    font-size: var(--text-sm);
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
    margin: 0 0 4px;
    color: var(--warn);
    font-size: var(--text-sm);
  }

  .quiet-line {
    margin: 0 0 4px;
    color: var(--faint);
    font-size: var(--text-sm);
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(96px, 1fr));
    gap: 6px;
    margin: 6px 0 4px;
  }

  .grid.four {
    grid-template-columns: repeat(4, minmax(0, 1fr));
  }

  .grid div {
    min-width: 0;
    padding: 5px 8px;
    border-radius: var(--radius-sm);
    background: rgba(255, 255, 255, 0.03);
  }

  .grid .wide {
    grid-column: span 2;
  }

  dt {
    color: var(--faint);
    font-size: var(--text-xs);
  }

  dd {
    margin: 0;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .small {
    color: var(--faint);
    font-size: var(--text-xs);
    font-weight: 400;
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
    margin: 12px 0 6px;
    font-family: var(--font-display);
    font-size: var(--text-sm);
    font-weight: 400;
    color: var(--muted);
  }

  .buffs {
    display: grid;
    gap: 4px;
    margin: 4px 0 0;
    padding: 0;
    list-style: none;
  }

  .buffs li {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: center;
    column-gap: 8px;
    row-gap: 1px;
    padding: 6px 8px;
    border-radius: var(--radius-sm);
    background: rgba(255, 255, 255, 0.03);
    font-size: var(--text-sm);
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
