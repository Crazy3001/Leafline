<script lang="ts">
  // One monster from the search: its numbers, what it is weak to, where it lives, what players
  // saw it drop, and the quests that want it. The numbers are the data's base values.
  import { tip } from "$lib/tip.svelte";
  import { ExternalLink, LoaderCircle, Skull } from "@lucide/svelte";
  import { untrack } from "svelte";

  import Card from "./Card.svelte";
  import MonsterDropList from "./monster/MonsterDropList.svelte";
  import PlaceList from "./map/PlaceList.svelte";
  import Section from "./map/Section.svelte";
  import { backend } from "$lib/api";
  import { formatCompact, formatDuration, formatNumber, monsterLabel } from "$lib/format";
  import { measuredRates, watchHistory } from "$lib/history.svelte";
  import { OSMS_CREDIT, OSMS_URL, WIKI_URL } from "$lib/mapdata.svelte";
  import Ref from "./Ref.svelte";
  import PicFrame from "$lib/ui/PicFrame.svelte";
  import { travelKey } from "$lib/settings.svelte";
  import type { MapReading, MonsterSheet, MonsterWhere, QuestReading, SessionSummary } from "$lib/types";

  let {
    id,
    map = null,
    level = null,
    quest = null,
    session = null,
    plain = false,
    compact = false,
  }: {
    id: number;
    map?: MapReading | null;
    level?: number | null;
    quest?: QuestReading | null;
    session?: SessionSummary | null;
    plain?: boolean;
    /** A link card's short form: the numbers, where to find it and what it drops. */
    compact?: boolean;
  } = $props();

  watchHistory();

  /** Where each element sits on the card, and what its colour means for the player. */
  const EFFECTS = [
    { effect: "Weak", label: "Weak to", tone: "good" },
    { effect: "Strong", label: "Strong against", tone: "warn" },
    { effect: "Immune", label: "Immune to", tone: "bad" },
  ];
  const CASTS: Record<string, string> = { self_buff: "Buffs itself", debuff: "Can debuff you" };

  let found = $state<MonsterWhere | null>(null);
  let error = $state<string | null>(null);

  const key = $derived(JSON.stringify([id, map?.name, map?.street, map?.twin, travelKey()]));

  // Follows the key alone: every live reading brings new objects with the same content.
  $effect(() => {
    const asked = key;
    error = null;
    untrack(() => backend.monsterSheet(id, map?.name ?? null, map?.street ?? null, map?.twin ?? null, measuredRates(session)))
      .then((result) => {
        if (asked === key) found = result;
      })
      .catch((e) => (error = String(e)));
  });

  const sheet = $derived(found?.sheet.id === id ? found.sheet : null);

  /** The three headline numbers in the HUD's colours; HP uses the lighter red that reads as text. */
  const TINT: Record<string, string> = { HP: "var(--bad)", MP: "var(--mp)", EXP: "var(--exp)" };

  const facts = $derived(
    sheet
      ? [
          { label: "HP", value: formatNumber(sheet.hp) },
          { label: "MP", value: formatNumber(sheet.details.mp) },
          { label: "EXP", value: formatNumber(sheet.exp) },
          {
            label: "EXP / 1k HP",
            value: sheet.hp ? ((sheet.exp * 1000) / sheet.hp).toFixed(0) : "—",
            title: "Base EXP per 1,000 monster HP: how much EXP each hit is worth",
          },
        ]
      : [],
  );

  const stats = $derived(sheet ? statRows(sheet) : []);
  const traits = $derived(sheet ? traitsOf(sheet) : []);

  /** How it compares with the character, from what the HUD and session tracker read. */
  const yours = $derived.by(() => {
    if (!sheet) return null;
    const gap = level == null ? null : sheet.level - level;
    const left = session?.expNeeded != null && session.exp ? session.expNeeded - session.exp.points : null;
    const kills = left != null && left > 0 && sheet.exp > 0 && !sheet.boss ? Math.ceil(left / sheet.exp) : null;
    return {
      gap: gap == null ? null : gap === 0 ? "Your level" : `${Math.abs(gap)} ${Math.abs(gap) === 1 ? "level" : "levels"} ${gap > 0 ? "above" : "below"} you`,
      kills,
    };
  });

  function statRows(monster: MonsterSheet) {
    const d = monster.details;
    const rows: { label: string; value: number | null; title?: string }[] = [
      { label: "Attack", value: d.attack },
      { label: "Magic", value: d.magic },
      { label: "Weapon Def.", value: d.weaponDef },
      { label: "Magic Def.", value: d.magicDef },
      { label: "Accuracy", value: d.accuracy },
      { label: "Evasion", value: d.evasion },
      { label: "Speed", value: d.speed, title: "Added to its movement speed. Below zero is slower." },
      { label: "Knockback", value: d.knockback, title: "The damage one hit must deal to knock it back" },
      { label: "HP recovery", value: d.hpRecovery },
      { label: "MP recovery", value: d.mpRecovery },
    ];
    return rows.filter((r) => r.value != null);
  }

  function traitsOf(monster: MonsterSheet) {
    const d = monster.details;
    const attacks = d.magicAttacks || d.attackElements.length ? [d.attackElements.join(", "), d.magicAttacks ? "magic" : "attacks"].filter(Boolean).join(" ") : null;
    return [
      monster.undead ? "Undead" : null,
      d.invincible ? "Can't be hurt" : null,
      d.aggressive ? "Attacks on sight" : null,
      attacks ? `Uses ${attacks}` : null,
      ...d.casts.map((kind) => CASTS[kind]),
    ].filter(Boolean);
  }

  /** How far the Quest Helper counted, for a quest it is tracking. */
  function tracked(name: string, monster: string) {
    const entry = quest?.quests.find((q) => q.title === name);
    const objective = entry?.objectives.find((o) => o.target === monster);
    if (!objective) return null;
    return `${objective.unread ? "–" : objective.current} / ${objective.required}`;
  }

  const wikiPage = (name: string) => `${WIKI_URL}/index.php?search=${encodeURIComponent(name)}&go=Go`;
</script>

<Card title="Monster" {plain}>
  {#snippet aside()}
    {#if sheet?.boss}<span class="boss"><Skull size={11} />Boss</span>{/if}
  {/snippet}

  {#if sheet}
    <div class="title-row">
      <PicFrame kind="mob" id={sheet.id} size={compact ? 52 : 60} />
      <div class="title-text">
        <h3 class="name">{monsterLabel(sheet.name)}</h3>
        <p class="meta tabular">{[`Lv ${sheet.level}`, yours?.gap].filter(Boolean).join(" · ")}</p>
        {#if sheet.expRank}
          <p class="rank tabular" use:tip={"Its HP for each point of EXP, against every field monster that spawns: first takes the least HP"}>
            <b>#{sheet.expRank.place}</b> of {sheet.expRank.of} for HP per EXP
          </p>
        {/if}
      </div>
    </div>

    <dl class="facts">
      {#each facts as fact (fact.label)}
        <div use:tip={fact.title}><dt>{fact.label}</dt><dd class="tabular" style:color={TINT[fact.label]}>{fact.value}</dd></div>
      {/each}
    </dl>

    {#if yours?.kills}
      <p class="kills tabular">
        About {formatNumber(yours.kills)} kills to your next level, at base EXP
      </p>
    {/if}

    {#if sheet.details.elements.length && !compact}
      <div class="elements">
        {#each EFFECTS as { effect, label, tone } (effect)}
          {@const named = sheet.details.elements.filter((e) => e.effect === effect).map((e) => e.element)}
          {#if named.length}
            <div><span class="soft">{label}</span> <span class={tone}>{named.join(", ")}</span></div>
          {/if}
        {/each}
      </div>
    {/if}

    {#if !compact}
    <dl class="stats">
      {#each stats as stat (stat.label)}
        <div use:tip={stat.title}><dt>{stat.label}</dt><dd class="tabular">{formatNumber(stat.value)}</dd></div>
      {/each}
    </dl>

    {#if traits.length}<p class="traits">{traits.join(" · ")}</p>{/if}
    {/if}

    {#if sheet.splits.length}
      <p class="traits">
        Turns into
        {#each sheet.splits as split, i (split.id)}
          {i ? ", " : ""}<Ref kind="mob" id={split.id} name={split.name}>{split.count} {monsterLabel(split.name)}</Ref>
        {/each}
        when defeated
      </p>
    {/if}

    {#if sheet.respawn}
      <p class="traits">
        Listed respawn <span class="tabular">{formatDuration(sheet.respawn)}</span>
      </p>
    {/if}

    <Section title="Where to find it" count={sheet.maps.length}>
      {#if sheet.maps.length}
        <PlaceList places={sheet.maps} from={found?.from ?? null} />
      {:else}
        <p class="soft note">Not on any map in the data. It may be summoned, or not in the game yet.</p>
      {/if}
    </Section>

    <Section title="Drops">
      <MonsterDropList drops={sheet.drops} name={sheet.name} />
    </Section>

    {#if sheet.quests.length && !compact}
      <Section title="Quests hunting it" count={sheet.quests.length}>
        <ul class="quests">
          {#each sheet.quests as need (need.quest.id)}
            {@const counted = tracked(need.quest.name, sheet.name)}
            <li>
              <Ref kind="quest" id={need.quest.id} name={need.quest.name} class="strong" />
              <span class="tabular" class:yours-now={counted}>{counted ?? `×${need.count}`}</span>
              <span class="who"
                >{#if need.quest.levelMin}Lv {need.quest.levelMin}{/if}{#if need.giver}{need.quest.levelMin ? " · " : ""}<Ref kind="npc" name={need.giver} />{/if}</span
              >
            </li>
          {/each}
        </ul>
      </Section>
    {/if}

    {#if sheet.versions.length && !compact}
      <Section title="Other versions" count={sheet.versions.length} open={false}>
        <ul class="versions">
          {#each sheet.versions as version (version.id)}
            <li>
              <Ref kind="mob" id={version.id} name={sheet.name} icon={false} class="strong tabular"
                >Lv {version.level} · {formatCompact(version.hp)} HP · {formatNumber(version.exp)} EXP</Ref
              >
              <span class="who"
                >{#each version.maps.slice(0, 2) as m, k (m.id)}{k ? ", " : ""}{#if m.known}<Ref kind="map" id={m.id} name={m.name} />{:else}{m.name}{/if}{:else}Not on any map{/each}</span
              >
            </li>
          {/each}
        </ul>
      </Section>
    {/if}

    {#if !compact}
    <div class="links">
      <button class="out" onclick={() => backend.openLink(wikiPage(sheet.name))}>
        mapleclassic.wiki<ExternalLink size={11} />
      </button>
    </div>
    {/if}
    <button class="credit" onclick={() => backend.openLink(OSMS_URL)}>{OSMS_CREDIT}</button>
  {:else if error}
    <p class="soft note">{error}</p>
  {:else}
    <p class="soft note"><span class="spin"><LoaderCircle size={13} /></span> Looking it up…</p>
  {/if}
</Card>

<style>
  .title-row {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 10px;
  }

  .title-text {
    min-width: 0;
  }

  .title-text .meta {
    margin-bottom: 0;
  }

  .rank {
    margin: 2px 0 0;
    color: var(--muted);
    font-size: var(--text-xs);
  }

  .rank b {
    color: var(--exp);
    font-weight: 600;
  }

  .name {
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--text-lg);
    font-weight: 600;
    line-height: 1.25;
  }

  .meta {
    margin: 2px 0 8px;
    color: var(--muted);
    font-size: var(--text-sm);
  }

  .soft {
    color: var(--faint);
  }

  .boss {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    color: var(--bad);
    font-weight: 600;
  }

  .facts {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 6px;
    margin: 0;
  }

  .facts div {
    min-width: 0;
    padding: 4px 7px;
    border-radius: var(--radius-sm);
    background: rgba(255, 255, 255, 0.03);
  }

  dt {
    color: var(--faint);
    font-size: var(--text-xs);
  }

  dd {
    margin: 0;
    font-weight: 600;
  }

  .kills {
    margin: 6px 0 0;
    color: var(--muted);
    font-size: var(--text-sm);
  }

  .elements {
    display: grid;
    gap: 1px;
    margin-top: 8px;
    font-size: var(--text-sm);
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

  .stats {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0 14px;
    margin: 8px 0 0;
  }

  .stats div {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    padding: 2px 0;
    border-top: 1px solid var(--border);
    font-size: var(--text-sm);
  }

  .stats dt {
    color: var(--muted);
    font-size: var(--text-sm);
  }

  .stats dd {
    font-weight: 500;
  }

  .traits {
    margin: 6px 0 0;
    color: var(--muted);
    font-size: var(--text-sm);
  }

  .link {
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--text);
    font-size: inherit;
    text-align: left;
    cursor: pointer;
  }

  .link:hover,
  .link:focus-visible {
    color: var(--accent-strong);
    text-decoration: underline;
    outline: none;
  }

  .link.strong {
    font-size: var(--text-sm);
    font-weight: 600;
  }

  .quests,
  .versions {
    margin: 0;
    padding: 0;
    list-style: none;
    font-size: var(--text-sm);
  }

  .quests li,
  .versions li {
    padding: 3px 0;
  }

  .quests li + li,
  .versions li + li {
    border-top: 1px solid var(--border);
  }

  .quests li {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    column-gap: 8px;
  }

  .quests .tabular {
    color: var(--muted);
  }

  .quests .yours-now {
    color: var(--accent-strong);
    font-weight: 600;
  }

  .who {
    grid-column: 1 / -1;
    display: block;
    color: var(--faint);
    font-size: var(--text-xs);
  }

  .links {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-top: 12px;
  }

  .out {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 24px;
    padding: 0 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--muted);
    font-size: var(--text-xs);
    cursor: pointer;
  }

  .out:hover {
    border-color: var(--border-strong);
    color: var(--text);
  }

  .credit {
    display: block;
    margin-top: 8px;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--faint);
    font-size: var(--text-xs);
    text-align: left;
    cursor: pointer;
  }

  .credit:hover,
  .credit:focus-visible {
    color: var(--muted);
    outline: none;
  }

  .note {
    margin: 0;
    font-size: var(--text-sm);
  }

  .spin {
    display: inline-grid;
    animation: spin 0.9s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
