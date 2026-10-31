<script lang="ts">
  import Card from "./Card.svelte";
  import PriceLog from "./PriceLog.svelte";
  import { formatSigned, titleCase } from "$lib/format";
  import type { ItemTooltip, Requirements } from "$lib/types";

  let { item, title = "Item", plain = false }: { item: ItemTooltip; title?: string; plain?: boolean } = $props();

  const requirementKeys: [keyof Requirements, string][] = [
    ["level", "LEV"],
    ["str", "STR"],
    ["dex", "DEX"],
    ["int", "INT"],
    ["luk", "LUK"],
    ["fam", "FAM"],
  ];

  const upgraded = $derived((item.upgrades ?? 0) > 0);
  const summary = $derived(
    [
      item.itemType,
      item.requirements.level != null ? `Lv ${item.requirements.level}` : null,
      item.remainingEnhancements != null ? `${item.remainingEnhancements} slots left` : null,
    ].filter(Boolean),
  );
</script>

<Card {title} {plain}>
  {#snippet aside()}
    <!-- Upgrades are read from the name's (+N) suffix, so say nothing when the name is unreadable. -->
    {#if item.name && !item.plain}
      <span class="verdict" class:upgraded>
        {upgraded ? `Upgraded +${item.upgrades}` : "No upgrades"}
      </span>
    {/if}
  {/snippet}

  <h3 class="name" class:upgraded>{item.name || "Unreadable name"}</h3>
  {#if summary.length}
    <p class="summary">{summary.join(" · ")}</p>
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
                <span title="Base value and bonus as shown in the tooltip">{stat.base} base</span>
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

  {#if !item.plain}
    <div class="chips">
      {#each requirementKeys as [key, label] (key)}
        {@const value = item.requirements[key]}
        <span class="chip req" class:zero={!value} class:unread={value == null}>
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

  <PriceLog {item} />
</Card>

<style>
  .name {
    margin: 0;
    font-family: var(--font-display);
    font-size: 16px;
    font-weight: 600;
    line-height: 1.25;
  }

  .name.upgraded {
    color: var(--accent-strong);
  }

  .summary {
    margin: 2px 0 10px;
    color: var(--muted);
    font-size: 12px;
  }

  .verdict {
    padding: 2px 8px;
    border-radius: var(--radius-sm);
    font-size: 11px;
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
    font-size: 11.5px;
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
    font-size: 11px;
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
