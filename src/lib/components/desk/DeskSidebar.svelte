<script lang="ts">
  // The desk view's sidebar: who you are and how the session goes, then every view and tool in
  // one place. It folds to icons in a narrow window.
  import { tip } from "$lib/tip.svelte";
  import {
    ChartLine,
    ChevronDown,
    ChevronRight,
    FlaskConical,
    Gem,
    Hammer,
    House,
    LayoutGrid,
    ListChecks,
    ListTree,
    Map as MapIcon,
    PackageSearch,
    PanelLeftClose,
    PanelLeftOpen,
    Route,
    Scale,
    Search,
    Settings,
    Share2,
    Timer,
    UserRound,
  } from "@lucide/svelte";

  import { backend } from "$lib/api";
  import Logo from "$lib/components/Logo.svelte";
  import { ZOOMS, type DeskView } from "$lib/desk";
  import { formatCompact } from "$lib/format";
  import { ago, lastKnown, profile, watchProfile } from "$lib/profile.svelte";
  import { live } from "$lib/readings.svelte";
  import LevelRing from "$lib/ui/LevelRing.svelte";
  import Segmented from "$lib/ui/Segmented.svelte";

  let {
    view,
    onview,
    folded,
    onfold,
    zoom,
    onzoom,
  }: {
    view: DeskView;
    onview: (view: DeskView) => void;
    folded: boolean;
    onfold: () => void;
    zoom: number;
    onzoom: (zoom: number) => void;
  } = $props();

  const VIEWS: { id: DeskView; title: string; icon: typeof House }[] = [
    { id: "now", title: "Now", icon: House },
    { id: "quests", title: "Quests", icon: ListChecks },
    { id: "map", title: "Map", icon: MapIcon },
    { id: "train", title: "Train", icon: ChartLine },
    { id: "me", title: "Me", icon: UserRound },
    { id: "lookup", title: "Look up", icon: Search },
  ];
  const TOOLS: { id: DeskView; title: string; icon: typeof House }[] = [
    { id: "planner", title: "Build planner", icon: ListTree },
    { id: "crafting", title: "Crafting", icon: Hammer },
    { id: "drops", title: "Drop log", icon: Gem },
    { id: "timers", title: "Timers", icon: Timer },
  ];
  const MORE: { id: DeskView; title: string; icon: typeof House }[] = [
    { id: "sell", title: "Sell or keep", icon: Scale },
    { id: "tracked", title: "Tracked items", icon: PackageSearch },
    { id: "potions", title: "Potion costs", icon: FlaskConical },
    { id: "route", title: "Route planner", icon: Route },
    { id: "share", title: "Share card", icon: Share2 },
  ];

  /** The less-used tools open under "More tools"; the one shown keeps them open. */
  let more = $state(false);
  const moreOpen = $derived(more || MORE.some((t) => t.id === view));

  watchProfile();

  // Read now when this character is on screen; else what was read of them last, faded and dated.
  const hud = $derived(profile.live ? (live.analysis?.hud ?? null) : null);
  const session = $derived(profile.live ? live.session : null);
  const known = $derived(profile.known);
  const levelShown = $derived(lastKnown(hud?.level ?? session?.level, known?.level, known?.seenAt));
  const level = $derived(levelShown.value);
  const percentShown = $derived(
    lastKnown(hud?.exp?.percent ?? hud?.expPercent ?? session?.percent, known?.expPercent, known?.seenAt),
  );
  const percent = $derived(percentShown.value);
  const name = $derived(hud?.name ?? session?.name ?? profile.name);
  const job = $derived(lastKnown(hud?.job ?? session?.job, known?.job, known?.seenAt).value);
  /** When what is shown was read, for a character not on screen now. */
  const seenAt = $derived(levelShown.at ?? percentShown.at);
  const reading = $derived(live.liveStatus.state === "reading");
  const status = $derived.by(() => {
    const s = live.liveStatus;
    if (s.state === "reading") return { tone: "good", text: "Reading" };
    if (s.state === "waiting") return { tone: "warn", text: "Waiting" };
    if (s.state === "failed") return { tone: "bad", text: "Stopped" };
    return { tone: "quiet", text: "Not reading" };
  });
  const statusTip = $derived.by(() => {
    const s = live.liveStatus;
    if (s.state === "reading") return `Reading ${s.window}`;
    if (s.state === "waiting") return s.reason;
    if (s.state === "failed") return `Live reading stopped: ${s.message}`;
    return "Not reading the game";
  });
  const expRate = $derived(
    session?.percentOnly
      ? session.percentPerHour != null
        ? `${session.percentPerHour.toFixed(1)}% / hr`
        : null
      : session?.expPerHour != null
        ? `${formatCompact(session.expPerHour)} EXP / hr`
        : null,
  );
  const mesosShown = $derived(
    lastKnown(live.analysis?.mesos?.mesos ?? session?.mesos, known?.mesos, known?.mesosSeenAt),
  );
  const mesos = $derived(mesosShown.value);
  const zoomOptions = ZOOMS.map((z) => ({ id: z, label: `${Math.round(z * 100)}%` }));
</script>

{#snippet item(entry: { id: DeskView; title: string; icon: typeof House }, count?: string | null)}
  {@const Icon = entry.icon}
  <button
    class="nav"
    class:on={view === entry.id}
    aria-current={view === entry.id ? "page" : undefined}
    aria-label={folded ? entry.title : undefined}
    use:tip={folded ? entry.title : undefined}
    onclick={() => onview(entry.id)}
  >
    <Icon size={16} />
    {#if !folded}<span class="label">{entry.title}</span>{#if count}<span class="count num">{count}</span>{/if}{/if}
  </button>
{/snippet}

<aside class:folded aria-label="Desk view">
  <div class="brand">
    {#if !folded}<Logo size={18} /><span class="word">Leafline</span>{/if}
    <button class="fold" aria-label={folded ? "Show the sidebar" : "Fold the sidebar"} use:tip={folded ? "Show the sidebar" : "Fold the sidebar"} onclick={onfold}>
      {#if folded}<PanelLeftOpen size={16} />{:else}<PanelLeftClose size={16} />{/if}
    </button>
  </div>

  {#if folded}
    <div class="ring" use:tip={`${name ?? "Your character"}${level != null ? `, Lv ${level}` : ""} · ${statusTip}`}>
      <LevelRing {level} {percent} size={40}><span class="num small-lv">{level ?? "?"}</span></LevelRing>
      <span class="dot {status.tone}" aria-hidden="true"></span>
    </div>
  {:else}
    <div class="who">
      <div class="who-top">
        <b class="name">{name ?? "Your character"}</b>
        <span class="status {status.tone}" use:tip={statusTip}><span class="dot" aria-hidden="true"></span>{status.text}</span>
      </div>
      <div class="sub" class:stale={levelShown.at != null}>
        {#if level != null}Lv {level}{job ? ` ${job}` : ""}{:else}Level not read yet{/if}
        {#if percent != null}<span class="num exp" class:dim={!reading}> · {percent.toFixed(1)}%</span>{/if}
      </div>
      <div class="track" class:stale={percentShown.at != null} aria-hidden="true"><span class:dim={!reading} style:width="{Math.max(0, Math.min(100, percent ?? 0))}%"></span></div>
      {#if expRate || mesos != null || seenAt != null}
        <div class="line">
          <span>{#if expRate}{expRate}{:else if seenAt != null}<span class="stale">Last seen {ago(seenAt)}</span>{/if}</span>
          {#if mesos != null}<span class="num mesos" class:stale={mesosShown.at != null} use:tip={mesosShown.at != null ? `As of ${ago(mesosShown.at)}` : undefined}>{mesos.toLocaleString()}</span>{/if}
        </div>
      {/if}
    </div>
  {/if}

  <nav aria-label="Views">
    {#if !folded}<p class="group">Views</p>{/if}
    {#each VIEWS as v (v.id)}
      {@render item(v, v.id === "quests" ? String((live.analysis?.quest?.quests ?? []).length || "") || null : null)}
    {/each}

    {#if !folded}<p class="group">Tools</p>{:else}<span class="rule"></span>{/if}
    {#each TOOLS as t (t.id)}
      {@render item(t)}
    {/each}
    {#if folded || moreOpen}
      {#each MORE as t (t.id)}
        {@render item(t)}
      {/each}
    {/if}
    {#if !folded}
      <button class="nav more" aria-expanded={moreOpen} onclick={() => (more = !moreOpen)}>
        {#if moreOpen}<ChevronDown size={14} />{:else}<ChevronRight size={14} />{/if}<span class="label">{moreOpen ? "Fewer tools" : "More tools"}</span>
      </button>
    {/if}
    {@render item({ id: "layout", title: "My layout", icon: LayoutGrid })}
  </nav>

  <div class="foot">
    {#if !folded}
      <div class="text-size">
        <span class="faint">Text</span>
        <Segmented label="Text size" size="sm" options={zoomOptions} value={zoom} onchange={onzoom} />
      </div>
    {/if}
    <button class="nav" aria-label={folded ? "Settings" : undefined} use:tip={folded ? "Settings" : undefined} onclick={() => backend.showSettings()}>
      <Settings size={16} />{#if !folded}<span class="label">Settings</span>{/if}
    </button>
    {#if !folded}
      <p class="credits">
        Unofficial: not made or approved by Nexon.<br />
        Data: OSMS (CC BY 4.0), mapleclassic.wiki (CC BY-NC-SA 4.0).
      </p>
    {/if}
  </div>
</aside>

<style>
  aside {
    display: flex;
    flex-direction: column;
    gap: 2px;
    width: 216px;
    flex: none;
    min-height: 0;
    padding: 12px 10px;
    overflow-y: auto;
    border-right: var(--line-w) solid var(--border);
    background: color-mix(in srgb, var(--surface) 35%, var(--bg));
  }

  aside.folded {
    width: 60px;
    align-items: center;
    padding: 12px 8px;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 2px 4px 10px 8px;
  }

  .folded .brand {
    padding: 2px 0 10px;
  }

  .word {
    font-family: var(--font-display);
    font-size: 16px;
    font-weight: 600;
  }

  .fold {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    margin-left: auto;
    padding: 0;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--faint);
    cursor: pointer;
  }

  .folded .fold {
    margin: 0;
  }

  .fold:hover {
    background: var(--hover);
    color: var(--text);
  }

  /* Remembered from an earlier read, not read now. */
  .stale {
    opacity: 0.6;
  }

  .who {
    display: grid;
    gap: 3px;
    margin-bottom: 8px;
    padding: 11px 12px;
    border: var(--line-w) solid var(--line);
    border-radius: var(--radius);
    background: var(--surface);
    box-shadow: var(--card-shadow);
  }

  .who-top,
  .line {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 8px;
  }

  .name {
    min-width: 0;
    overflow: hidden;
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .sub,
  .line {
    color: var(--muted);
    font-size: var(--text-sm);
  }

  .line {
    margin-top: 4px;
    font-size: var(--text-xs);
  }

  .num {
    font-family: var(--font-display);
    font-variant-numeric: tabular-nums;
  }

  .exp {
    color: var(--exp-text);
    font-weight: 600;
  }

  .exp.dim {
    color: var(--muted);
  }

  .mesos {
    color: var(--mesos);
  }

  .track {
    height: 5px;
    margin-top: 5px;
    border-radius: var(--radius-pill);
    background: var(--track);
    overflow: hidden;
  }

  .track span {
    display: block;
    height: 100%;
    border-radius: var(--radius-pill);
    background: var(--exp);
  }

  .track span.dim {
    background: var(--faint);
  }

  .status {
    display: inline-flex;
    flex: none;
    align-items: center;
    gap: 5px;
    color: var(--faint);
    font-size: var(--text-xs);
    --dot: var(--faint);
  }

  .status.good {
    color: var(--good);
    --dot: var(--good);
  }

  .status.warn {
    color: var(--warn);
    --dot: var(--warn);
  }

  .status.bad {
    color: var(--bad);
    --dot: var(--bad);
  }

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--dot, var(--faint));
  }

  .ring {
    position: relative;
    margin-bottom: 8px;
  }

  .ring .dot {
    position: absolute;
    right: 0;
    bottom: 2px;
    width: 9px;
    height: 9px;
    border: 2px solid var(--bg);
  }

  .ring .dot.good {
    --dot: var(--good);
  }

  .ring .dot.warn {
    --dot: var(--warn);
  }

  .ring .dot.bad {
    --dot: var(--bad);
  }

  .small-lv {
    font-size: 13px;
    font-weight: 600;
  }

  nav {
    display: flex;
    flex-direction: column;
    gap: 1px;
    width: 100%;
  }

  .group {
    margin: 10px 0 3px;
    padding: 0 10px;
    color: var(--faint);
    font-size: var(--text-xs);
    font-weight: 500;
  }

  .rule {
    display: block;
    margin: 8px 6px;
    border-top: 1px solid var(--border);
  }

  .nav {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    height: 34px;
    padding: 0 10px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--muted);
    font: inherit;
    font-size: var(--text-md);
    font-weight: 500;
    text-align: left;
    cursor: pointer;
  }

  .folded .nav {
    justify-content: center;
    width: 40px;
    padding: 0;
  }

  .nav:hover {
    background: var(--hover);
    color: var(--text);
  }

  .nav.on {
    background: var(--accent-soft);
    color: var(--accent-text-strong);
  }

  .nav:focus-visible,
  .fold:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }

  .label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .count {
    color: var(--faint);
    font-size: var(--text-xs);
  }

  .nav.on .count {
    color: inherit;
  }

  .nav.more {
    height: 28px;
    color: var(--faint);
    font-size: var(--text-sm);
  }

  .foot {
    display: grid;
    gap: 4px;
    width: 100%;
    margin-top: auto;
    padding-top: 10px;
  }

  .text-size {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 4px 6px;
    padding: 0 0 0 10px;
  }

  .faint {
    color: var(--faint);
    font-size: var(--text-xs);
  }

  .credits {
    margin: 6px 0 0;
    padding: 8px 10px 0;
    border-top: 1px solid var(--border);
    color: var(--faint);
    font-size: 10.5px;
    line-height: 15px;
  }
</style>
