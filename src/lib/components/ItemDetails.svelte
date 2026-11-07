<script lang="ts">
  // What the data says about an item, under its card: what it does, where to get it, and the
  // links to the build and to scrolls. An item read from a tooltip is matched by name first.
  import { Check, LoaderCircle, Plus, ScrollText } from "@lucide/svelte";
  import { untrack } from "svelte";

  import ObjectiveHint from "./quest/ObjectiveHint.svelte";
  import Section from "./map/Section.svelte";
  import { backend } from "$lib/api";
  import { buildState, watchBuilds } from "$lib/builds.svelte";
  import { attackSpeedName, formatMesos } from "$lib/format";
  import { blankQuery, itemView } from "$lib/items.svelte";
  import { OSMS_CREDIT, OSMS_URL, WIKI_CREDIT, WIKI_URL, mapData } from "$lib/mapdata.svelte";
  import { showNpcNamed } from "$lib/places.svelte";
  import { live } from "$lib/readings.svelte";
  import type { GearPiece, ItemRecord, ItemTooltip, ItemWhere, MapReading } from "$lib/types";

  let {
    item = null,
    record = null,
    map = null,
  }: { item?: ItemTooltip | null; record?: ItemRecord | null; map?: MapReading | null } = $props();

  watchBuilds();

  let matched = $state<ItemRecord | null>(null);
  let found = $state<ItemWhere | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let goal = $state<boolean | null>(null);
  /** The build's pieces, or null with no build: a copy read from the game is added as one. */
  let pieces = $state<GearPiece[] | null>(null);
  let pieceError = $state<string | null>(null);

  const ready = $derived(mapData.status?.ready ?? false);
  const shown = $derived(record ?? matched);
  const character = $derived(live.analysis?.hud?.name ?? null);
  const nameKey = $derived(record ? null : JSON.stringify([item?.name, item?.requirements.level, ready]));
  const place = $derived(JSON.stringify([shown?.id, map?.name, map?.street, map?.twin]));

  $effect(() => {
    const key = nameKey;
    matched = null;
    if (key == null || !ready) return;
    untrack(() => (item?.name ? backend.findItem(item.name, item.requirements.level) : null))?.then((result) => {
      if (key === nameKey) matched = result;
    });
  });

  // Follows the key alone: every live reading brings new objects with the same content.
  $effect(() => {
    const key = place;
    found = null;
    error = null;
    const id = untrack(() => shown?.id);
    if (id == null) return;
    loading = true;
    backend
      .itemOrigin(id, untrack(() => map?.name ?? null), untrack(() => map?.street ?? null), untrack(() => map?.twin ?? null))
      .then((result) => {
        if (key === place) found = result;
      })
      .catch((e) => (error = String(e)))
      .finally(() => (loading = false));
  });

  $effect(() => {
    void buildState.version;
    const id = shown?.id;
    const who = character;
    if (id == null || shown?.kind !== "equip") {
      goal = null;
      pieces = null;
      return;
    }
    backend.activeBuild(who).then((plan) => {
      if (shown?.id !== id) return;
      goal = plan ? plan.build.plan.gear.includes(id) : null;
      pieces = plan ? (plan.build.start.pieces ?? []) : null;
    });
  });

  /** This very copy, as read: its own stats and scrolls, for My build. */
  const piece = $derived.by((): GearPiece | null => {
    if (!item || item.plain || !item.name || !shown) return null;
    return {
      id: 0,
      item: shown.id,
      name: item.name,
      itemType: item.itemType,
      level: item.requirements.level,
      stats: item.stats.map((s) => ({ label: s.label, value: s.value })),
      upgrades: item.upgrades,
      slotsLeft: item.remainingEnhancements,
      slots: shown.upgradeSlots ?? null,
    };
  });
  const samePiece = (a: GearPiece, b: GearPiece) => JSON.stringify({ ...a, id: 0 }) === JSON.stringify({ ...b, id: 0 });
  const kept = $derived(piece && pieces ? (pieces.find((p) => samePiece(p, piece)) ?? null) : null);

  async function togglePiece() {
    if (!piece) return;
    try {
      if (kept) await backend.removePiece(kept.id, character);
      else await backend.addPiece(piece, character);
      pieceError = null;
    } catch (e) {
      pieceError = String(e);
    }
  }

  const speed = $derived(shown?.attackSpeed);
  const rate = $derived(shown?.scroll?.successRate);
  const tags = $derived([shown?.questItem && "Quest item", shown?.untradable && "Can't be traded"].filter(Boolean));

  function scrollsFor(target: ItemRecord) {
    itemView.query = { ...blankQuery(), kind: "scroll", slot: target.weaponType ?? target.slot };
    itemView.id = null;
  }

  async function toggleGoal(target: ItemRecord) {
    await backend.setGearGoal(target.id, !goal, character);
  }
</script>

{#if shown}
  {#if shown.description}<p class="about">{shown.description}</p>{/if}

  {#if tags.length || speed != null || rate != null || shown.upgradeSlots != null || shown.price != null}
    <dl class="facts">
      {#if shown.upgradeSlots != null}
        <div><dt>Upgrade slots</dt><dd class="tabular">{shown.upgradeSlots}</dd></div>
      {/if}
      {#if speed != null}
        <div title={`The data's number for it is ${speed}. Lower is faster.`}>
          <dt>Attack speed</dt><dd class="tabular">{attackSpeedName(speed) ?? speed}</dd>
        </div>
      {/if}
      {#if rate != null}
        <div><dt>Success rate</dt><dd class="tabular">{rate}%</dd></div>
      {/if}
      {#if shown.price != null}
        <div title="The price the data lists. What it means in Classic isn't confirmed."><dt>Listed price</dt><dd class="tabular">{formatMesos(shown.price)}</dd></div>
      {/if}
    </dl>
    {#if tags.length}<p class="tags">{tags.join(" · ")}</p>{/if}
  {/if}

  {#if shown.kind === "equip"}
    <div class="actions">
      {#if piece && pieces != null}
        <!-- A copy read from the game may be the player's own: put it in its slot as it is, scrolls and all. -->
        <button
          class:on={!!kept}
          title={kept ? "Take it off in Character → Gear" : "Put this copy in its slot in Character → Gear, with the stats it has"}
          onclick={togglePiece}
        >
          {#if kept}<Check size={12} />Wearing this one{:else}<Plus size={12} />I'm wearing this{/if}
        </button>
      {/if}
      {#if goal != null}
        <button class:on={goal} title={goal ? "Take it off the plan's gear goals" : "Show it as Next for its slot in Character → Gear"} onclick={() => toggleGoal(shown)}>
          {#if goal}<Check size={12} />In my plan{:else}<Plus size={12} />Aim for this{/if}
        </button>
      {/if}
      <button onclick={() => scrollsFor(shown)}><ScrollText size={12} />Scrolls for it</button>
    </div>
    {#if pieceError}<p class="piece-error">{pieceError}</p>{/if}
  {/if}

  <Section title="Where to get it">
    {#if found}
      <ObjectiveHint guide={found.origin.drops} from={found.from} />
      {#if !found.origin.drops.lookedUp}
        <p class="note">The drop table isn't downloaded yet.</p>
      {/if}
      {#if found.origin.quests.length}
        <h5>Quest reward</h5>
        <ul class="quests">
          {#each found.origin.quests as quest (quest.quest.id)}
            {@const giverMap = quest.giverMaps[0]}
            <li>
              <button class="quest" onclick={() => backend.showQuest(quest.quest.id)}>{quest.quest.name}</button>
              <span class="who">
                {#if quest.quest.levelMin}Lv {quest.quest.levelMin}{/if}{#if quest.giver}{quest.quest.levelMin ? " · " : ""}<button
                    class="ref ref-npc"
                    onclick={() => showNpcNamed(quest.giver!)}>{quest.giver}</button
                  >{/if}{#if giverMap}{quest.quest.levelMin || quest.giver ? " · " : ""}{#if giverMap.known}<button
                      class="ref ref-map"
                      onclick={() => backend.showPlace("maps", giverMap.id)}>{giverMap.name}</button
                    >{:else}{giverMap.name}{/if}{/if}
              </span>
            </li>
          {/each}
        </ul>
      {/if}
      <p class="credit">
        <button onclick={() => backend.openLink(WIKI_URL)}>{WIKI_CREDIT}</button>
        <button onclick={() => backend.openLink(OSMS_URL)}>{OSMS_CREDIT}</button>
      </p>
    {:else if error}
      <p class="note">{error}</p>
    {:else if loading}
      <p class="note"><span class="spin"><LoaderCircle size={13} /></span> Looking it up…</p>
    {/if}
  </Section>
{/if}

<style>
  .piece-error {
    margin: 4px 0 0;
    color: var(--bad);
    font-size: var(--text-sm);
  }

  .about {
    margin: 8px 0 0;
    color: var(--muted);
    font-size: var(--text-sm);
    white-space: pre-line;
  }

  .facts {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(90px, 1fr));
    gap: 6px;
    margin: 10px 0 0;
  }

  .facts div {
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

  .tags {
    margin: 6px 0 0;
    color: var(--warn);
    font-size: var(--text-sm);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-top: 10px;
  }

  .actions button {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 3px 8px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--muted);
    font-size: var(--text-sm);
    cursor: pointer;
  }

  .actions button:hover,
  .actions button:focus-visible {
    color: var(--text);
    outline: none;
  }

  .actions .on {
    border-color: color-mix(in srgb, var(--accent) 45%, transparent);
    background: var(--accent-soft);
    color: var(--accent-strong);
  }

  h5 {
    margin: 8px 0 3px;
    color: var(--muted);
    font-size: var(--text-sm);
    font-weight: 500;
  }

  .quests {
    margin: 0;
    padding: 0;
    list-style: none;
    font-size: var(--text-sm);
  }

  .quests li {
    display: grid;
    padding: 3px 0;
  }

  .quests li + li {
    border-top: 1px solid var(--border);
  }

  .quest {
    padding: 0;
    border: 0;
    background: transparent;
    font-size: var(--text-sm);
    font-weight: 600;
    text-align: left;
    cursor: pointer;
  }

  .quest:hover,
  .quest:focus-visible {
    color: var(--accent-strong);
    text-decoration: underline;
    outline: none;
  }

  .who {
    color: var(--muted);
    font-size: var(--text-xs);
  }

  .note {
    margin: 4px 0 0;
    color: var(--faint);
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

  .credit {
    display: grid;
    gap: 2px;
    margin: 8px 0 0;
  }

  .credit button {
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--faint);
    font-size: var(--text-xs);
    text-align: left;
    cursor: pointer;
  }

  .credit button:hover,
  .credit button:focus-visible {
    color: var(--muted);
    outline: none;
  }
</style>
