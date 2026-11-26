<script lang="ts">
  // Next up: a short list of what's worth doing now, the best first, each with one button. Built
  // only from what Leafline already reads and knows: quests whose steps are all done (to hand in),
  // AP the Character Stat window shows unspent, and the next quest objective with the map that has
  // most of it. Suggestions to act on, never alerts.
  import { Footprints, Gift, Plus, Sparkles } from "@lucide/svelte";
  import { untrack } from "svelte";

  import { backend } from "$lib/api";
  import GoButton from "$lib/components/GoButton.svelte";
  import Ref from "$lib/components/Ref.svelte";
  import { damage, followDamage } from "$lib/damage.svelte";
  import { mapPlace, npcPlace } from "$lib/destination.svelte";
  import { groundDistance, questTitle } from "$lib/format";
  import { mapData, watchMapData } from "$lib/mapdata.svelte";
  import { readyToHandIn, questLog, watchQuestLog } from "$lib/questlog.svelte";
  import { huntOrder, orderGrounds } from "$lib/quests.svelte";
  import { live } from "$lib/readings.svelte";
  import type { Destination, Guide, MapRef, QuestSheet } from "$lib/types";
  import TitledCard from "$lib/ui/TitledCard.svelte";

  let { onme }: { onme: () => void } = $props();

  watchMapData();
  watchQuestLog();

  const ready = $derived(mapData.status?.ready ?? false);
  const analysis = $derived(live.analysis);
  const map = $derived(analysis?.map ?? null);
  const name = $derived(analysis?.hud?.name ?? live.session?.name ?? null);

  $effect(() => {
    const who = name;
    untrack(() => followDamage(who));
  });

  // ---- Where each tracked quest's objectives are, and who takes it in.
  const tracked = $derived((analysis?.quest?.quests ?? []).filter((q) => q.title));
  let guide = $state<Guide | null>(null);
  const guideKey = $derived(JSON.stringify([tracked.map((q) => [q.title, q.objectives.map((o) => o.target)]), map?.name, map?.street, map?.twin]));
  $effect(() => {
    const key = guideKey;
    void mapData.status?.dataUpdatedAt;
    if (!ready || !tracked.length) {
      guide = null;
      return;
    }
    const asked = tracked.map((q) => ({ title: q.title, objectives: q.objectives.map((o) => o.target) }));
    untrack(() =>
      backend
        .questGuide(asked, map?.name ?? null, map?.street ?? null, map?.twin ?? null, [])
        .then((g) => {
          if (key === guideKey) guide = g;
        })
        .catch(() => (guide = null)),
    );
  });

  // ---- Quest log entries ready to hand in, with who takes them.
  const logReady = $derived((questLog.log?.quests ?? []).filter(readyToHandIn).slice(0, 2));
  let sheets = $state<Record<number, QuestSheet>>({});
  const readyKey = $derived(logReady.map((q) => q.questId ?? "").join(","));
  $effect(() => {
    const asked = readyKey;
    if (!ready) return;
    untrack(() => {
      const ids = logReady.map((q) => q.questId).filter((id): id is number => id != null && !sheets[id]);
      Promise.all(ids.map((id) => backend.questSheet(id, null, null, null)))
        .then((found) => {
          if (asked !== readyKey) return;
          const next = { ...sheets };
          found.forEach((w) => {
            if (w) next[w.sheet.row.id] = w.sheet;
          });
          sheets = next;
        })
        .catch(() => {});
    });
  });

  type Step = {
    key: string;
    kind: "handin" | "ap" | "hunt";
    title: string;
    npc?: string | null;
    map?: MapRef | null;
    detail?: string;
    place?: Destination | null;
  };

  const steps = $derived.by(() => {
    const out: Step[] = [];
    const seen = new Set<string>();

    // Quests the Quest Helper shows with every step done.
    tracked.forEach((q, i) => {
      const done = q.objectives.length > 0 && q.objectives.every((o) => !o.unread && o.current >= o.required);
      if (!done) return;
      const info = guide?.quests[i]?.quest ?? null;
      const npc = info?.turnIn ?? info?.giver ?? null;
      const maps = info?.turnIn ? info.turnInMaps : (info?.giverMaps ?? []);
      seen.add(q.title.toLowerCase());
      out.push({ key: `h:${q.title}`, kind: "handin", title: `Hand in ${questTitle(info?.quest.name ?? q.title).name}`, npc, map: maps[0] ?? null, place: npc ? npcPlace(npc, maps) : null });
    });
    // And the quest log's, as of the last look at the Quest window.
    for (const q of logReady) {
      if (seen.has(q.title.toLowerCase())) continue;
      const info = q.questId != null ? sheets[q.questId]?.info : null;
      const npc = info?.turnIn ?? info?.giver ?? null;
      const maps = info?.turnIn ? info.turnInMaps : (info?.giverMaps ?? []);
      out.push({ key: `l:${q.key}`, kind: "handin", title: `Hand in ${questTitle(q.title).name}`, npc, map: maps[0] ?? null, place: npc ? npcPlace(npc, maps) : null });
    }

    // AP the Character Stat window last showed unspent.
    const stats = damage.name && name && damage.name.toLowerCase() === name.toLowerCase() ? damage.stats : null;
    const ap = stats?.sheet?.abilityPoints ?? stats?.base?.abilityPoints ?? null;
    if (ap && ap > 0) out.push({ key: "ap", kind: "ap", title: `Spend ${ap} AP`, detail: "Your build on Me says where" });

    // The next objective still to do, with the map that has the most of it.
    outer: for (const [i, q] of tracked.entries()) {
      for (const o of q.objectives) {
        if (o.unread || o.current >= o.required) continue;
        const g = guide?.quests[i]?.objectives.find((x) => x.target === o.target);
        const hunt = g?.hunts.find((h) => h.monster.known && h.maps.length);
        if (!g || !hunt) continue;
        const best = orderGrounds(hunt.maps, huntOrder.value)[0];
        const left = o.required - o.current;
        const verb = g.kind === "item" ? "Collect" : "Defeat";
        out.push({
          key: `o:${q.title}:${o.target}`,
          kind: "hunt",
          title: `${verb} ${left} more ${o.target}`,
          map: best.map,
          detail: best.hops === 0 ? "On this map" : groundDistance(best, guide?.from != null),
          place: best.hops === 0 ? null : mapPlace(best.map),
        });
        break outer;
      }
    }
    return out.slice(0, 4);
  });

  const ICONS = { handin: Gift, ap: Plus, hunt: Footprints };
</script>

<TitledCard title="Next up" icon={Sparkles} tone="leaf">
  {#snippet aside()}best first{/snippet}
  {#if steps.length}
    <ol class="steps">
      {#each steps as s, i (s.key)}
        {@const Icon = ICONS[s.kind]}
        <li class="step {s.kind}" class:top={i === 0}>
          <span class="ic" aria-hidden="true"><Icon size={18} /></span>
          <span class="t">{s.title}</span>
          <span class="d">
            {#if s.kind === "handin"}
              {#if s.npc}To <Ref kind="npc" name={s.npc} />{#if s.map}{" "}· <Ref kind="map" id={s.map.id} name={s.map.name} />{/if}{:else}Ready to hand in{/if}
            {:else if s.kind === "hunt" && s.map}
              Most on <Ref kind="map" id={s.map.id} name={s.map.name} />{#if s.detail}{" "}· {s.detail}{/if}
            {:else}
              {s.detail}
            {/if}
          </span>
          <span class="act">
            {#if s.kind === "ap"}
              <button class="btn sm" onclick={onme}>Build</button>
            {:else if s.place}
              <GoButton place={s.place} />
            {/if}
          </span>
        </li>
      {/each}
    </ol>
  {:else}
    <p class="empty">Nothing waiting. Quests ready to hand in, AP to spend and where to hunt show here as they come up.</p>
  {/if}
</TitledCard>

<style>
  .steps {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .step {
    display: grid;
    grid-template-columns: 38px minmax(0, 1fr) auto;
    column-gap: var(--space-3);
    row-gap: var(--space-0-5);
    align-items: center;
    padding: var(--space-3);
    border-radius: var(--radius);
    background: var(--surface-2);
  }

  .step.top {
    background: color-mix(in srgb, var(--exp) 15%, var(--surface-2));
  }

  .ic {
    display: grid;
    grid-row: span 2;
    place-items: center;
    width: 38px;
    height: 38px;
    border-radius: var(--radius);
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .handin .ic {
    background: color-mix(in srgb, var(--exp) 22%, transparent);
    color: var(--exp-text);
  }

  .hunt .ic {
    background: color-mix(in srgb, var(--ref-mob) 18%, transparent);
    color: var(--ref-mob);
  }

  .t {
    overflow: hidden;
    font-weight: var(--weight-strong);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .d {
    grid-column: 2;
    min-width: 0;
    overflow: hidden;
    color: var(--muted);
    font-size: var(--text-xs);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .act {
    grid-column: 3;
    grid-row: 1 / 3;
  }

  .empty {
    margin: 0;
    color: var(--muted);
  }
</style>
