<script lang="ts">
  // The small card a link shows while the mouse rests on it: what the thing is at a glance, the
  // way a game tooltip does. A click opens the full card. Shown through the window's one tooltip.
  import { attackSpeedText, formatMesos, formatNumber, formatSigned, splitMapLabel } from "$lib/format";
  import { hoverData, type HoverData } from "$lib/hover";
  import { itemSlot, lastLine } from "$lib/items.svelte";
  import { mapData } from "$lib/mapdata.svelte";
  import type { PeekTarget } from "$lib/peek.svelte";
  import { loadSkills, skillById, skillNamed } from "$lib/skills.svelte";
  import Pic from "$lib/ui/Pic.svelte";

  let {
    target,
    note = null,
  }: {
    target: PeekTarget;
    /** A line of its own about this link, such as that it's worn. */
    note?: string | null;
  } = $props();

  /** The most names a line lists. */
  const LIST = 3;

  let data = $state<HoverData | null>(null);
  let done = $state(false);

  $effect(() => {
    const t = target;
    data = null;
    done = false;
    if (t.kind === "skill") {
      loadSkills().finally(() => (done = true));
      return;
    }
    let live = true;
    hoverData({ ...t, kind: t.kind }).then((found) => {
      if (!live) return;
      data = found;
      done = true;
    });
    return () => {
      live = false;
    };
  });

  const skill = $derived(target.kind === "skill" ? (skillById(target.id) ?? skillNamed(target.name)) : null);
  const title = $derived(
    target.kind === "map" && target.name ? splitMapLabel(target.name).name : (target.name ?? skill?.name ?? ""),
  );
  const names = (list: string[]) => {
    const unique = [...new Set(list)];
    return unique.slice(0, LIST).join(", ") + (unique.length > LIST ? ` and ${unique.length - LIST} more` : "");
  };
  const requires = (r: { classes: string[]; reqStr: number; reqDex: number; reqInt: number; reqLuk: number }) =>
    [
      r.classes.join(" / "),
      ...(
        [
          ["STR", r.reqStr],
          ["DEX", r.reqDex],
          ["INT", r.reqInt],
          ["LUK", r.reqLuk],
        ] as const
      )
        .filter(([, v]) => v > 0)
        .map(([k, v]) => `${k} ${v}`),
    ]
      .filter(Boolean)
      .join(" · ");
</script>

{#snippet head(kind: "item" | "mob" | "npc" | "skill" | null, id: number | null, name: string, sub: string)}
  <div class="head">
    {#if kind}
      <span class="pic"><Pic {kind} {id} {name} size={32} /></span>
    {/if}
    <span class="titles">
      <b class="name">{name}</b>
      {#if sub}<span class="sub">{sub}</span>{/if}
    </span>
  </div>
{/snippet}

{#snippet row(label: string, value: string)}
  <span class="label">{label}</span><span class="value">{value}</span>
{/snippet}

<div class="hover">
  {#if data?.kind === "item"}
    {@const r = data.record}
    {@render head("item", r.id, r.name, r.kind === "equip" ? `Lv ${r.reqLevel} · ${r.weaponType ?? itemSlot(r)}` : itemSlot(r))}
    {#if r.kind === "equip" && (r.stats.length || r.attackSpeed != null || r.upgradeSlots != null)}
      <div class="rows">
        {#each r.stats as stat (stat.label)}{@render row(stat.label, formatSigned(stat.value))}{/each}
        {#if r.attackSpeed != null}{@render row("Speed", attackSpeedText(r.attackSpeed))}{/if}
        {#if r.upgradeSlots != null}{@render row("Slots", String(r.upgradeSlots))}{/if}
      </div>
    {:else if lastLine(r)}
      <p class="text">{lastLine(r)}</p>
    {/if}
    {#if r.kind === "equip" && requires(r)}<p class="line"><span class="label">Requires</span> {requires(r)}</p>{/if}
    {#if data.origin}
      {@const o = data.origin}
      {#if o.drops.hunts.length}
        <p class="line"><span class="label">Dropped by</span> {names(o.drops.hunts.map((h) => h.monster.name))}</p>
      {/if}
      {#if o.shops.length}
        <p class="line">
          <span class="label">Sold by</span>
          {o.shops[0].npc}{o.shops[0].mapName ? `, ${o.shops[0].mapName}` : ""} · {formatMesos(o.shops[0].price)} mesos
        </p>
      {/if}
      {#if o.quests.length}
        <p class="line"><span class="label">From quests</span> {names(o.quests.map((q) => q.quest.name))}</p>
      {/if}
    {/if}
  {:else if data?.kind === "mob"}
    {@const s = data.sheet}
    {@render head("mob", s.id, s.name, [`Lv ${s.level}`, s.boss && "Boss", s.undead && "Undead", s.details.aggressive && "Attacks on sight"].filter(Boolean).join(" · "))}
    <div class="rows">
      {@render row("HP", formatNumber(s.hp))}
      {@render row("EXP", formatNumber(s.exp))}
    </div>
    {#each ["Weak", "Strong", "Immune"] as const as effect (effect)}
      {@const els = s.details.elements.filter((e) => e.effect === effect).map((e) => e.element)}
      {#if els.length}
        <p class="line">
          <span class="label">{effect === "Weak" ? "Weak to" : effect === "Strong" ? "Strong against" : "Immune to"}</span>
          <span class:good={effect === "Weak"} class:bad={effect !== "Weak"}>{els.join(", ")}</span>
        </p>
      {/if}
    {/each}
    {#if s.maps.length}<p class="line"><span class="label">Found in</span> {names(s.maps.map((m) => m.map.name))}</p>{/if}
    {#if s.drops?.drops.length}<p class="line"><span class="label">Drops</span> {names(s.drops.drops.map((d) => d.item))}</p>{/if}
  {:else if data?.kind === "npc"}
    {@const s = data.sheet}
    {@render head("npc", s.id, s.name, s.maps[0]?.map.name ?? "")}
    {#if s.maps.length > 1}<p class="line"><span class="label">Also in</span> {names(s.maps.slice(1).map((m) => m.map.name))}</p>{/if}
    {#if s.shop}<p class="line"><span class="label">Shop</span> sells {s.shop.items.length} {s.shop.items.length === 1 ? "item" : "items"}</p>{/if}
    {#if s.quests.length}<p class="line"><span class="label">Quests</span> {names(s.quests.map((q) => q.name))}</p>{/if}
  {:else if data?.kind === "map"}
    {@const d = data.details}
    {@render head(null, null, d.map.name, [d.map.street, d.isTown ? "Town" : null].filter(Boolean).join(" · "))}
    {#if d.levelRange}<div class="rows">{@render row("Monsters", `Lv ${d.levelRange[0]}${d.levelRange[1] !== d.levelRange[0] ? `–${d.levelRange[1]}` : ""}`)}</div>{/if}
    {#if d.mobs.length}<p class="line"><span class="label">Monsters</span> {names(d.mobs.map((m) => `${m.name} (Lv ${m.level})`))}</p>{/if}
    {#if d.npcs.length}<p class="line"><span class="label">NPCs</span> {names(d.npcs.map((n) => n.name))}</p>{/if}
    {#if d.shops.length}<p class="line"><span class="label">Shops</span> {names(d.shops.map((shop) => shop.npc))}</p>{/if}
  {:else if data?.kind === "quest"}
    {@const q = data.sheet.row}
    {@render head(null, null, q.name, [q.levelMin != null ? `Lv ${q.levelMin}+` : null, q.region, q.repeat].filter(Boolean).join(" · "))}
    <div class="rows">
      {#if q.giver}{@render row("From", q.giver)}{/if}
      {#if q.exp}{@render row("EXP", formatNumber(q.exp))}{/if}
      {#if q.mesos}{@render row("Mesos", formatNumber(q.mesos))}{/if}
    </div>
    {#if data.sheet.rewards.length}
      <p class="line"><span class="label">Rewards</span> {names(data.sheet.rewards.map((r) => (r.count > 1 ? `${r.name} ×${r.count}` : r.name)))}</p>
    {/if}
  {:else if skill}
    {@render head("skill", skill.id, skill.name, [skill.className, `Max Lv ${skill.maxLevel}`, skill.passive ? "Passive" : null].filter(Boolean).join(" · "))}
    {#if skill.description}<p class="text clamp">{skill.description}</p>{/if}
    {#if skill.levelDescriptions.length}
      <p class="line"><span class="label">At Lv {skill.levelDescriptions.length}</span> {skill.levelDescriptions.at(-1)}</p>
    {/if}
  {:else}
    {@render head(target.kind === "map" || target.kind === "quest" ? null : target.kind, target.id, title, "")}
    {#if done}
      <p class="text faint">
        {mapData.status && !mapData.status.ready ? "The map data isn't downloaded yet." : "Nothing more in the data."}
      </p>
    {:else}
      <p class="text faint">Looking it up…</p>
    {/if}
  {/if}
  {#if note}<p class="note">{note}</p>{/if}
</div>

<style>
  .hover {
    display: grid;
    gap: var(--space-1-5);
    width: 260px;
    max-width: 100%;
    font-size: var(--text-xs);
  }

  .head {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
  }

  .pic {
    display: grid;
    flex: none;
    place-items: center;
    width: 40px;
    height: 40px;
    border-radius: var(--radius-sm);
    background: var(--hover);
  }

  .pic:empty {
    display: none;
  }

  .titles {
    display: grid;
    min-width: 0;
  }

  .name {
    color: var(--text);
    font-size: var(--text-md);
    font-weight: var(--weight-strong);
  }

  .sub {
    color: var(--muted);
  }

  .rows {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 1px var(--space-3);
    padding-top: var(--space-1-5);
    border-top: 1px solid var(--border);
  }

  .rows:empty {
    display: none;
  }

  .label {
    color: var(--faint);
  }

  .value {
    color: var(--text);
    font-variant-numeric: tabular-nums;
  }

  .line,
  .text,
  .note {
    margin: 0;
    color: var(--muted);
  }

  .line:first-of-type {
    padding-top: var(--space-1-5);
    border-top: 1px solid var(--border);
  }

  .clamp {
    display: -webkit-box;
    overflow: hidden;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 4;
    line-clamp: 4;
  }

  .faint {
    color: var(--faint);
  }

  .good {
    color: var(--good);
  }

  .bad {
    color: var(--bad);
  }

  .note {
    color: var(--text);
    white-space: pre-line;
  }
</style>
