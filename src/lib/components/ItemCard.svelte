<script lang="ts">
  // The item check: one item, read from its tooltip or taken from the data. Its stats, whether
  // it has been scrolled, where it comes from, and the player's own notes. Leafline has no
  // market prices.
  import { untrack } from "svelte";

  import BuildFit from "./BuildFit.svelte";
  import Card from "./Card.svelte";
  import DropLogger from "./DropLogger.svelte";
  import ItemDetails from "./ItemDetails.svelte";
  import ItemNotes from "./ItemNotes.svelte";
  import { backend } from "$lib/api";
  import { reportUrl } from "$lib/feedback";
  import { formatSigned, titleCase } from "$lib/format";
  import { mapData } from "$lib/mapdata.svelte";
  import { checkItem, nameMatch, statRows, tooltipOf } from "$lib/items.svelte";
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
  /** The lookup for the current name has finished, so no match means there really is none. */
  let looked = $state(false);
  const ready = $derived(mapData.status?.ready ?? false);
  const nameKey = $derived(read ? JSON.stringify([read.name, read.requirements.level, ready]) : null);

  // Follows the key alone: every live reading brings new objects with the same content.
  $effect(() => {
    const key = nameKey;
    matched = null;
    looked = false;
    if (key == null || !ready) return;
    untrack(() => (read?.name ? backend.findItem(read.name, read.requirements.level) : Promise.resolve(null)))?.then((result) => {
      if (key === nameKey) {
        matched = result;
        looked = true;
      }
    });
  });

  /**
   * How far the read can be trusted. "exact": the name matches an item in the data, so the data's
   * fields are the baseline and the read only updates what can be believed. "close": the data's
   * nearest item is a guess, so only the data is shown. "none": nothing in the data fits. "unchecked":
   * the data isn't downloaded, so the read is shown as it came.
   */
  const trust = $derived.by((): "data" | "exact" | "likely" | "close" | "none" | "unchecked" | "looking" => {
    if (!read) return "data";
    if (!ready) return "unchecked";
    if (!looked) return "looking";
    if (!matched) return "none";
    return nameMatch(read, matched);
  });
  const checked = $derived(read && matched && (trust === "exact" || trust === "likely") ? checkItem(read, matched) : null);

  /** Requirements, type and jobs are the same on every copy, so once the item is known the data's stand. */
  const fromData = $derived(matched && trust !== "unchecked" ? tooltipOf(matched) : null);
  const filled = $derived(new Set(fromData ? requirementKeys.map(([key]) => key) : []));
  const item = $derived.by((): ItemTooltip => {
    if (!read) return tooltipOf(record!);
    if (!fromData) return read;
    return { ...read, itemType: fromData.itemType, requirements: fromData.requirements, jobs: fromData.jobs };
  });
  const upgraded = $derived((trust === "exact" || trust === "likely") && item.upgraded);

  interface Row {
    label: string;
    value: number;
    base: number | null;
    bonus: number | null;
    standard: number | null;
    fromData: boolean;
    note: string | null;
  }
  /** The stats shown. */
  const stats = $derived.by((): Row[] => {
    if (checked) return checked.stats.map((s) => ({ ...s, fromData: s.source === "data" }));
    if (matched) return statRows(tooltipOf(matched), matched).map((s) => ({ ...s, fromData: true, note: null }));
    if (!read) return item.stats.map((s) => ({ ...s, standard: null, fromData: false, note: null }));
    return trust === "unchecked" ? item.stats.map((s) => ({ ...s, standard: null, fromData: false, note: null })) : [];
  });
  const unconfirmed = $derived(checked?.unconfirmed ?? []);
  /** Attack speed never changes with scrolls and knockback is fixed too, so they come from the data and what OCR
   * made of those lines is left out. */
  const properties = $derived.by(() => {
    if (!matched || trust === "unchecked") return trust === "none" ? [] : item.properties;
    return matched.knockback != null ? [{ label: "Knockback Chance", value: `${matched.knockback}%` }] : [];
  });
  /** The data's spelling once the read matched an item: a read can garble letters, or run on
   * into text seen through the tooltip. */
  const name = $derived(matched?.name ?? item.name);

  /**
   * Clean or scrolled. Successful scrolls show as the name's (+N) and its amber colour; failed
   * ones only as fewer slots left than the item has, which needs the data's slot count. Said only
   * when the name matched exactly: a guess at the item can't say how its copy was scrolled.
   */
  const scrolling = $derived.by(() => {
    if (!read || read.plain || !item.name || (trust !== "exact" && trust !== "likely" && trust !== "unchecked")) return null;
    const total = matched?.upgradeSlots ?? null;
    const left = checked ? checked.slotsLeft : item.remainingEnhancements;
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
        : total != null && checked
          ? `${total} upgrade slots · slots left not read`
          : null;
    return { verdict, line, counted };
  });
  // The slots line under it says slots left in full when the data's count is known.
  const summary = $derived(
    [
      item.itemType,
      item.requirements.level ? `Lv ${item.requirements.level}` : null,
      !checked && item.remainingEnhancements != null && !scrolling?.line && trust === "unchecked"
        ? `${item.remainingEnhancements} slots left`
        : null,
    ].filter(Boolean),
  );
  /** Something on the card is a guess or couldn't be confirmed, so the picture may be the problem. */
  const doubtful = $derived(
    !!read && (trust === "close" || trust === "likely" || unconfirmed.length > 0 || !!checked?.slotsNote || stats.some((s) => s.note)),
  );
  let savedTo = $state<string | null>(null);
  let saveFailed = $state(false);
  async function savePicture() {
    saveFailed = false;
    try {
      savedTo = await backend.saveItemPicture(JSON.stringify({ read, matched: matched?.name ?? null, trust, unconfirmed }));
    } catch {
      saveFailed = true;
    }
  }
  const unsureName = $derived(trust === "close" && read ? read.name : null);
  const likelyName = $derived(trust === "likely" && read ? read.name : null);
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

  {#if trust === "none" && read}
    <p class="notice bad">
      Couldn't tell which item this is{read.name ? ` (read as “${read.name}”)` : ""}. Nothing in the item data fits closely
      enough, so no stats are shown rather than wrong ones.
    </p>
    <p class="notice-help">
      The tooltip is easiest to read over a plain background. Move the inventory or equipment window to a quiet part of the
      screen, close any windows behind where the tooltip appears (chat, other panels), then hover the item and press the hotkey
      again. You can also search for it by name. If it keeps failing, press Save frame on the overlay and send the picture in a
      bug report.
    </p>
    <button class="fix" onclick={() => backend.showItem(null, read.name || null)}>
      {read.name ? `Search for “${read.name}”` : "Search items"}
    </button>
  {:else}
  {#if trust === "looking"}
    <p class="notice">Checking the name against the item data…</p>
  {:else if unsureName}
    <p class="notice warn">
      Not sure this is {name}: the name read as “{unsureName}”. It could be a different item, so only {name}'s own stats are
      shown, not what was read.
    </p>
  {:else if likelyName}
    <p class="notice warn">The name read as “{likelyName}”. Matched to {name} by its spelling and level, so check it is the right item.</p>
  {:else if trust === "unchecked"}
    <p class="notice warn">The item data isn't downloaded, so this reading can't be checked against the item's real stats.</p>
  {/if}
  {#if doubtful}
    <div class="notice-help doubt">
      <p>
        Not sure about this reading? Move the inventory or equipment window to a clear part of the screen, close any windows
        behind the tooltip, then hover the item and press the hotkey again.
      </p>
      <span class="actions">
        <button class="fix" onclick={savePicture}>Save this picture</button>
        <button class="fix" onclick={() => backend.openLink(reportUrl())}>Report it</button>
      </span>
      {#if savedTo}<p class="saved">Saved to {savedTo}. Attach it to the report.</p>{/if}
      {#if saveFailed}<p class="saved">Couldn't save the picture.</p>{/if}
    </div>
  {/if}
  <h3 class="name" class:upgraded>{name || "Unreadable name"}</h3>
  {#if summary.length}
    <p class="summary">{summary.join(" · ")}</p>
  {/if}
  {#if scrolling?.line}
    <p class="slots tabular">{scrolling.line}</p>
  {/if}

  {#if stats.length}
    <table class="stats">
      <tbody>
        {#each stats as stat (stat.label)}
          <tr>
            <th>{stat.label}</th>
            <td class="value tabular">{formatSigned(stat.value)}</td>
            <td class="detail tabular">
              {#if stat.base != null && stat.bonus != null}
                <span title="The item's own value, then what this copy has added or lost">
                  ({stat.base}
                  <span class="bonus" class:negative={stat.bonus < 0}>{formatSigned(stat.bonus)}</span>)
                </span>
              {:else if stat.note}
                <span class="unsure" title={stat.note}>base default · unsure</span>
              {:else if stat.fromData}
                <span title="Not read from the tooltip: this is the item's default base value">base default</span>
              {:else if stat.standard != null}
                {@const diff = stat.value - stat.standard}
                <span title="The item's own value, then what this copy has added or lost. A scroll, a rolled copy or a misread makes it differ.">
                  ({stat.standard}
                  <span class="bonus" class:negative={diff < 0}>{formatSigned(diff)}</span>)
                </span>
              {/if}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}

  {#each stats.filter((s) => s.note) as stat (stat.label)}
    <p class="notice warn small">{stat.label}: {stat.note}.</p>
  {/each}
  {#if unconfirmed.length}
    <p class="notice warn small">Also read but not confirmed, so not shown as stats: {unconfirmed.join(", ")}.</p>
  {/if}
  {#if checked?.slotsNote}
    <p class="notice warn small">{checked.slotsNote}.</p>
  {/if}

  {#each properties as property}
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

  <ItemDetails
    item={read}
    record={record ?? matched}
    {map}
    checked={checked ? checked.stats : null}
    slotsLeft={checked?.slotsLeft ?? null}
  />

  <!-- Only for a copy read from the game: that's the one the player just got. -->
  {#if read}<DropLogger item={name} itemId={matched?.id ?? null} {map} />{/if}

  <!-- No prices until a price source gives permission. Prices logged by earlier builds stay in
       the records file (src-tauri/src/userdata/prices.rs). -->
  <ItemNotes {name} />
  {/if}
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

  .notice {
    margin: 0 0 8px;
    padding: 6px 8px;
    border-radius: var(--radius-sm);
    font-size: var(--text-sm);
    color: var(--muted);
    border: 1px solid var(--border);
  }

  .notice.warn {
    color: var(--warn);
    background: var(--warn-soft);
    border-color: transparent;
  }

  .notice.bad {
    color: var(--bad);
    border-color: var(--bad);
  }

  .notice.small {
    margin: 4px 0;
    padding: 3px 8px;
    font-size: var(--text-xs);
  }

  .notice-help {
    margin: 0 0 8px;
    color: var(--muted);
    font-size: var(--text-sm);
  }

  .doubt p {
    margin: 0 0 6px;
  }

  .doubt .actions {
    display: flex;
    gap: 6px;
  }

  .doubt .saved {
    margin: 6px 0 0;
    font-size: var(--text-xs);
    word-break: break-all;
  }

  .fix {
    padding: 5px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: rgba(255, 255, 255, 0.04);
    color: var(--text);
    font-size: var(--text-sm);
    cursor: pointer;
  }

  .unsure {
    color: var(--warn);
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
    margin-left: 2px;
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
