<script lang="ts">
  // The item check: one item, read from its tooltip or taken from the data. Its stats, whether
  // it has been scrolled, where it comes from, and the player's own notes and prices. Leafline
  // has no market prices: the prices here are only the ones the player logged.
  import { untrack } from "svelte";

  import BuildFit from "./BuildFit.svelte";
  import Card from "./Card.svelte";
  import DropLogger from "./DropLogger.svelte";
  import ItemDetails from "./ItemDetails.svelte";
  import ItemNotes from "./ItemNotes.svelte";
  import { backend } from "$lib/api";
  import { formatSigned, titleCase } from "$lib/format";
  import { mapData } from "$lib/mapdata.svelte";
  import { tooltipOf } from "$lib/items.svelte";
  import type { ItemRecord, ItemTooltip, MapReading, Requirements } from "$lib/types";

  let {
    item: read,
    record = null,
    map = null,
    title = "Item",
    plain = false,
  }: {
    /** A copy read from the game, or null to show `record` from the data. */
    item: ItemTooltip | null;
    record?: ItemRecord | null;
    map?: MapReading | null;
    title?: string;
    plain?: boolean;
  } = $props();

  const wearable = $derived(read ? !read.plain : record?.kind === "equip");

  const requirementKeys: [keyof Requirements, string][] = [
    ["level", "LEV"],
    ["str", "STR"],
    ["dex", "DEX"],
    ["int", "INT"],
    ["luk", "LUK"],
    ["fam", "FAM"],
  ];

  /** The data's record for a copy read from the game, to count the slots it started with. */
  let matched = $state<ItemRecord | null>(null);
  const ready = $derived(mapData.status?.ready ?? false);
  const nameKey = $derived(read && !read.plain ? JSON.stringify([read.name, read.requirements.level, ready]) : null);

  // Follows the key alone: every live reading brings new objects with the same content.
  $effect(() => {
    const key = nameKey;
    matched = null;
    if (key == null || !ready) return;
    untrack(() => (read?.name ? backend.findItem(read.name, read.requirements.level) : null))?.then((result) => {
      if (key === nameKey) matched = result;
    });
  });

  /**
   * The copy as read, with what the reader missed taken from the data. Requirements, type and
   * jobs are the same on every copy; stats and slots left are not (stats roll on a drop, failed
   * scrolls use slots), so those only ever come from the tooltip.
   */
  const fromData = $derived(read && !read.plain && matched ? tooltipOf(matched) : null);
  const filled = $derived(
    new Set(fromData ? requirementKeys.map(([key]) => key).filter((key) => read!.requirements[key] == null) : []),
  );
  const item = $derived.by((): ItemTooltip => {
    if (!read) return tooltipOf(record!);
    if (!fromData) return read;
    const requirements = { ...read.requirements };
    for (const key of filled) requirements[key] = fromData.requirements[key];
    return {
      ...read,
      itemType: read.itemType ?? fromData.itemType,
      requirements,
      jobs: read.jobs.length ? read.jobs : fromData.jobs,
    };
  });
  const upgraded = $derived(item.upgraded);

  /**
   * Clean or scrolled. Successful scrolls show as the name's (+N) and its amber colour; failed
   * ones only as fewer slots left than the item has, which needs the data's slot count.
   */
  const scrolling = $derived.by(() => {
    if (!read || read.plain || !item.name) return null;
    const total = matched?.upgradeSlots ?? null;
    const left = item.remainingEnhancements;
    const used = total != null && left != null && left <= total ? total - left : null;
    const passed = item.upgrades ?? (upgraded ? null : 0);
    const failed = used != null && passed != null && used >= passed ? used - passed : null;
    const counted = total != null ? `Counted from the data's ${total} upgrade slots, which Classic could change.` : "";
    const plural = (n: number) => (n === 1 ? "" : "s");
    let verdict: { label: string; tone: "clean" | "scrolled"; title: string };
    if (item.upgrades) verdict = { label: `Scrolled +${item.upgrades}`, tone: "scrolled", title: "The (+N) after its name" };
    else if (upgraded) verdict = { label: "Scrolled", tone: "scrolled", title: "Its name is amber, but the (+N) wasn't read" };
    else if (failed)
      verdict = {
        label: "Failed scrolls",
        tone: "scrolled",
        title: `No upgrades, but ${failed} slot${plural(failed)} used. ${counted}`,
      };
    else if (used === 0) verdict = { label: "Clean", tone: "clean", title: `No upgrades, every slot left. ${counted}` };
    else
      verdict = {
        label: "No upgrades",
        tone: "clean",
        title: "No (+N) after its name. Failed scrolls only show as missing slots, and this item's slot count isn't known.",
      };
    const line =
      total != null && left != null && left <= total
        ? `${left} of ${total} slots left` + (failed != null && used ? ` · ${passed} passed, ${failed} failed` : "")
        : null;
    return { verdict, line, counted };
  });
  // The slots line under it says slots left in full when the data's count is known.
  const summary = $derived(
    [
      item.itemType,
      item.requirements.level ? `Lv ${item.requirements.level}` : null,
      item.remainingEnhancements != null && !scrolling?.line ? `${item.remainingEnhancements} slots left` : null,
    ].filter(Boolean),
  );
</script>

<Card {title} {plain}>
  {#snippet aside()}
    <!-- Upgrades are read from the name's (+N) suffix, so say nothing when the name is unreadable. -->
    {#if scrolling}
      <span class="verdict" class:upgraded={scrolling.verdict.tone === "scrolled"} title={scrolling.verdict.title}>
        {scrolling.verdict.label}
      </span>
    {/if}
  {/snippet}

  <h3 class="name" class:upgraded>{item.name || "Unreadable name"}</h3>
  {#if summary.length}
    <p class="summary">{summary.join(" · ")}</p>
  {/if}
  {#if scrolling?.line}
    <p class="slots tabular" title={scrolling.counted}>{scrolling.line}</p>
  {/if}

  {#if item.stats.length}
    <table class="stats">
      <tbody>
        {#each item.stats as stat}
          <tr>
            <th>{stat.label}</th>
            <td class="value tabular">{formatSigned(stat.value)}</td>
            <td class="detail tabular">
              {#if stat.base != null && stat.bonus != null}
                <span>{stat.base} base</span>
                <span class="bonus" class:negative={stat.bonus < 0}>{formatSigned(stat.bonus)}</span>
              {/if}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}

  {#each item.properties as property}
    <p class="property"><span>{property.label}</span>{property.value}</p>
  {/each}

  {#if wearable}
    <div class="chips">
      {#each requirementKeys as [key, label] (key)}
        {@const value = item.requirements[key]}
        <span
          class="chip req"
          class:zero={!value}
          class:unread={value == null}
          title={filled.has(key) && value != null ? "Not read from the tooltip: this is the data's value" : undefined}
        >
          {label}<b class="tabular">{value ?? "?"}</b>
        </span>
      {/each}
    </div>
  {/if}

  {#if item.jobs.length}
    <div class="chips jobs">
      {#each item.jobs as flag (flag.job)}
        <span class="chip job" class:eligible={flag.eligible}>{titleCase(flag.job)}</span>
      {/each}
    </div>
  {/if}

  {#if wearable}<BuildFit {item} />{/if}

  <ItemDetails item={read} record={record ?? matched} {map} />

  <!-- Only for a copy read from the game: that's the one the player just got. -->
  {#if read}<DropLogger item={item.name} itemId={matched?.id ?? null} {map} />{/if}

  <!-- The price log (PriceLog.svelte) is hidden for v1: with no price source it was all typing.
       Its records stay in the file, ready for when a price source gives permission. -->
  <ItemNotes name={item.name} />
</Card>

<style>
  .name {
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--text-lg);
    font-weight: 600;
    line-height: 1.25;
  }

  .name.upgraded {
    color: var(--accent-strong);
  }

  .summary {
    margin: 2px 0 10px;
    color: var(--muted);
    font-size: var(--text-sm);
  }

  .slots {
    margin: -8px 0 10px;
    color: var(--faint);
    font-size: var(--text-sm);
  }

  .verdict {
    padding: 2px 8px;
    border-radius: var(--radius-sm);
    font-size: var(--text-xs);
    font-weight: 600;
    color: var(--good);
    background: var(--good-soft);
  }

  .verdict.upgraded {
    color: var(--warn);
    background: var(--warn-soft);
  }

  .stats {
    width: 100%;
    border-collapse: collapse;
    margin-bottom: 8px;
  }

  .stats tr + tr {
    border-top: 1px solid var(--border);
  }

  .stats th {
    width: 100%;
    padding: 5px 0;
    text-align: left;
    font-weight: 500;
    color: var(--muted);
  }

  .stats .value {
    padding: 5px 8px;
    text-align: right;
    font-weight: 600;
    white-space: nowrap;
  }

  .stats .detail {
    width: 1%;
    text-align: right;
    white-space: nowrap;
    color: var(--faint);
    font-size: var(--text-sm);
  }

  .bonus {
    margin-left: 4px;
    color: var(--good);
  }

  .bonus.negative {
    color: var(--bad);
  }

  .property {
    display: flex;
    justify-content: space-between;
    margin: 0 0 6px;
  }

  .property span {
    color: var(--muted);
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-top: 8px;
  }

  .chip {
    padding: 2px 7px;
    border-radius: var(--radius-sm);
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid var(--border);
    font-size: var(--text-xs);
    color: var(--muted);
  }

  .req b {
    margin-left: 5px;
    color: var(--text);
    font-weight: 600;
  }

  .req.zero b {
    color: var(--faint);
    font-weight: 500;
  }

  .req.unread {
    border-style: dashed;
  }

  .job {
    color: var(--faint);
  }

  .job.eligible {
    color: var(--accent-strong);
    border-color: rgba(243, 177, 63, 0.35);
    background: var(--accent-soft);
  }
</style>
