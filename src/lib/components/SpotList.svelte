<script lang="ts" module>
  /** The section's element id, for the Train tab's jump link. */
  export const SPOTS_ID = "where-to-train";
</script>

<script lang="ts">
  // Where to train: maps near the character's level, ranked for this player. "For you" uses the
  // EXP per hour measured on each map, and for maps not tried yet an estimate from the player's
  // kill speed: the monster HP they get through an hour solo, over the HP of the map's average
  // monster, held to what the map can respawn at an assumed respawn time (see training.rs). No
  // damage formulas are modelled. "Played" lists every map the player has data for, whatever its
  // level, by their own rate. "Saved" lists the maps the player bookmarked, whatever their level.
  import { clipped, tip } from "$lib/tip.svelte";
  import Ref from "$lib/components/Ref.svelte";
  import GoButton from "$lib/components/GoButton.svelte";
  import { mapPlace } from "$lib/destination.svelte";
  import { Bookmark, ChevronRight, CircleHelp, ExternalLink, Minus, Plus } from "@lucide/svelte";
  import { onMount, untrack } from "svelte";

  import SaveSpot from "./SaveSpot.svelte";
  import { spots as spotStore, watchSpots } from "$lib/spots.svelte";

  import MapNote from "./MapNote.svelte";
  import MapDataPanel from "./map/MapDataPanel.svelte";
  import RouteSteps from "./map/RouteSteps.svelte";
  import { backend } from "$lib/api";
  import { damage, effective, followDamage } from "$lib/damage.svelte";
  import { live } from "$lib/readings.svelte";
  import { openTrainingLog } from "$lib/nav.svelte";
  import { formatCompact, formatDuration, monsterLabel } from "$lib/format";
  import { measuredRates, watchHistory } from "$lib/history.svelte";
  import { mapData, watchMapData } from "$lib/mapdata.svelte";
  import { travelKey } from "$lib/settings.svelte";
  import {
    assumptions,
    DEFAULT_ASSUMPTIONS,
    ATTACK_RANGE,
    EXP_RATE_RANGE,
    SKILL_FACTOR_RANGE,
    isDefault,
    RESPAWN_RANGE,
    setAssumptions,
    watchAssumptions,
  } from "$lib/assumptions.svelte";
  import type { Estimate, MapReading, MapRoute, SessionSummary, TrainingSpot, TrainingSpots } from "$lib/types";
  import Block from "$lib/ui/Block.svelte";
  import Button from "$lib/ui/Button.svelte";
  import Segmented from "$lib/ui/Segmented.svelte";

  let {
    level: readLevel,
    map = null,
    session = null,
    last = true,
  }: {
    /** Read from the HUD or the session; the player can set it when it isn't. */
    level: number | null;
    map?: MapReading | null;
    session?: SessionSummary | null;
    /** The bottom section of the tab, with no rule under it. */
    last?: boolean;
  } = $props();

  type Sort = "you" | "played" | "near" | "saved";
  /** Maps whose average monster level is this far below or above the character count as near. */
  const BELOW = 8;
  const ABOVE = 3;
  const SHOWN = 6;

  const SORT_KEY = "leafline.train.sort";
  const LEVEL_KEY = "leafline.train.level";
  const ALL_LEVELS_KEY = "leafline.train.allLevels";

  watchMapData();
  watchHistory();
  watchAssumptions();
  const ready = $derived(mapData.status?.ready ?? false);

  let sort = $state<Sort>(storedSort());
  let manualLevel = $state<number | null>(Number(stored(LEVEL_KEY, "")) || null);
  let allLevels = $state(stored(ALL_LEVELS_KEY, "false") === "true");
  let more = $state(false);
  let data = $state<TrainingSpots | null>(null);
  let open = $state<number | null>(null);
  let route = $state<MapRoute | null>(null);
  let editing = $state(false);

  onMount(() => {
    watchSpots();
  });

  const saved = $derived(spotStore.saved);

  const level = $derived(readLevel ?? manualLevel);
  // What Leafline read of the player's attack and skills fills what they haven't typed.
  $effect(() => {
    const name = session?.name ?? live.analysis?.hud?.name ?? null;
    untrack(() => followDamage(name));
  });
  const used = $derived(effective(assumptions.value, damage.value, damage.stats));
  const measured = $derived(measuredRates(session));
  const queryKey = $derived(
    JSON.stringify([map?.name, map?.street, map?.twin, measured, travelKey(), level, used.assumptions]),
  );
  const fromId = $derived(data?.from?.id);

  // Follows the key alone: every live reading brings new objects with the same content.
  $effect(() => {
    const key = queryKey;
    void mapData.status?.dataUpdatedAt;
    if (!ready) {
      data = null;
      return;
    }
    untrack(() =>
      backend.trainingSpots(map?.name ?? null, map?.street ?? null, map?.twin ?? null, measured, level, used.assumptions),
    )
      .then((result) => {
        if (key === queryKey) data = result;
      })
      .catch(() => (data = null));
  });

  $effect(() => {
    const id = open;
    const from = fromId;
    void travelKey();
    route = null;
    if (id == null || from == null) return;
    backend.mapRoute(from, id).then((r) => {
      if (open === id) route = r;
    });
  });

  const steps = (s: TrainingSpot) => s.travel?.hops ?? s.hops;
  /** Maps the player has put real time into stay in For you a while after their level passes them,
   *  so their own rate there is weighed against the estimates. */
  const PLAYED_WELL = 10 * 60;
  const FAR_BELOW = 15;
  const playedWell = (s: TrainingSpot) =>
    s.measured != null && s.measured.activeSeconds >= PLAYED_WELL && (level == null || s.averageLevel >= level - FAR_BELOW);
  const near = (s: TrainingSpot) => level == null || (s.averageLevel >= level - BELOW && s.averageLevel <= level + ABOVE);
  const savedIds = $derived(new Set(saved.map((s) => s.mapId)));
  const sorts = $derived<{ id: Sort; label: string; title: string }[]>([
    { id: "you", label: "For you", title: "Your EXP per hour where you have played, an estimate from your kill speed where you haven't" },
    { id: "played", label: "Played", title: "Every map you have played, whatever its level, by your own EXP per hour" },
    { id: "near", label: "Nearest", title: "Fewest portals from where you are" },
    {
      id: "saved",
      label: saved.length ? `Saved ${saved.length}` : "Saved",
      title: "The maps you bookmarked, whatever their level, best for you first",
    },
  ]);

  const speed = $derived(data?.speed ?? null);
  /** The estimate for a map the player hasn't measured. */
  const estimate = (s: TrainingSpot) => (s.measured ? null : s.estimate);
  const forYou = (s: TrainingSpot) => s.measured?.expPerHour ?? estimate(s)?.expPerHour ?? -1;

  const ranked = $derived.by(() => {
    const spots = (data?.spots ?? []).filter((s) =>
      sort === "saved"
        ? savedIds.has(s.map.id)
        : sort === "played"
          ? s.measured != null && s.measured.activeSeconds >= 60
          : s.reachable && (allLevels || near(s) || savedIds.has(s.map.id) || playedWell(s)),
    );
    // With no kill speed yet, maps not tried fall back to base EXP, below every measured one.
    const you = (s: TrainingSpot) => (speed == null && !s.measured ? s.expPerClear / 1e6 - 1 : forYou(s));
    const by: Record<Sort, (s: TrainingSpot) => number> = {
      you,
      played: (s) => s.measured?.expPerHour ?? -1,
      near: (s) => -(steps(s) ?? 9999),
      saved: you,
    };
    const value = by[sort];
    const sorted = [...spots].sort((a, b) => value(b) - value(a) || b.expPerClear - a.expPerClear);
    // Strictly by the rate shown: a close call is still the higher number first.
    return sorted;
  });
  const shown = $derived(sort === "saved" ? ranked : ranked.slice(0, more ? 30 : SHOWN));
  const missingSaved = $derived(saved.filter((s) => !(data?.spots ?? []).some((spot) => spot.map.id === s.mapId)));

  function stored(key: string, fallback: string) {
    try {
      return localStorage.getItem(key) ?? fallback;
    } catch {
      return fallback;
    }
  }

  function storedSort(): Sort {
    const value = stored(SORT_KEY, "you");
    // "EXP / HP" and "Your rate" were sorts of their own until the Train tab was redone.
    return value === "played" || value === "near" || value === "saved" ? value : "you";
  }

  function remember(key: string, value: string) {
    try {
      localStorage.setItem(key, value);
    } catch {
      // A convenience only.
    }
  }

  function pickSort(next: Sort) {
    sort = next;
    remember(SORT_KEY, next);
  }

  function setLevel(next: number) {
    manualLevel = Math.max(1, Math.min(200, next));
    remember(LEVEL_KEY, String(manualLevel));
  }

  /** How many of a map's monsters the row names, most common first. */
  const NAMED = 3;

  /** Opens or closes a row from the keyboard, as its button role promises. */
  function rowKey(event: KeyboardEvent, toggle: () => void) {
    if (event.target !== event.currentTarget) return;
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      toggle();
    }
  }

  /**
   * The row's one line about the map: the levels of its main monsters, then their names. The
   * full list, bosses and rare spawns included, is in the open row.
   */
  function summary(spot: TrainingSpot) {
    const common = spot.mobs.filter((m) => !m.boss).sort((a, b) => b.count - a.count || a.level - b.level);
    const main = (common.length ? common : spot.mobs).slice(0, NAMED);
    const [low, high] = main.length
      ? [Math.min(...main.map((m) => m.level)), Math.max(...main.map((m) => m.level))]
      : spot.levelRange;
    // Variants that share a displayed name (Jr. Boogie 1 and 2) are one monster to the player.
    const mobs = [...main]
      .sort((a, b) => a.level - b.level)
      .filter((m, i, all) => all.findIndex((o) => monsterLabel(o.name) === monsterLabel(m.name)) === i);
    return { levels: `Lv ${low}${high !== low ? `–${high}` : ""}`, mobs };
  }

  /** A map's monsters with the numbered variants of one name merged into a line, their counts added. */
  function distinctMobs(mobs: TrainingSpot["mobs"]) {
    const out: TrainingSpot["mobs"] = [];
    for (const mob of mobs) {
      const same = out.find((o) => monsterLabel(o.name) === monsterLabel(mob.name) && o.level === mob.level && o.boss === mob.boss);
      if (same) out[out.indexOf(same)] = { ...same, count: same.count + mob.count };
      else out.push(mob);
    }
    return out;
  }

  /** The same line as plain text, for the row's tooltip. */
  const summaryText = (spot: TrainingSpot) => {
    const { levels, mobs } = summary(spot);
    return [levels, ...(mobs.length ? [mobs.map((m) => monsterLabel(m.name)).join(", ")] : [])].join(" · ");
  };

  function metric(spot: TrainingSpot): { value: string; unit: string; quiet: boolean; sure?: Estimate["sureness"] } {
    if (sort === "near") {
      const hops = steps(spot);
      if (hops == null) return { value: "—", unit: "no route", quiet: true };
      return { value: String(hops), unit: hops === 0 ? "you are here" : spot.travel ? "with travel" : hops === 1 ? "portal" : "portals", quiet: false };
    }
    if (spot.measured) return { value: formatCompact(spot.measured.expPerHour), unit: "your EXP / hr", quiet: false };
    const guess = estimate(spot);
    if (guess) return { value: `≈${formatCompact(guess.expPerHour)}`, unit: "est. EXP / hr", quiet: true, sure: guess.sureness };
    return { value: formatCompact(spot.expPerClear), unit: "EXP / clear", quiet: true };
  }

  const SURENESS = { 3: "Fairly sure", 2: "Rough", 1: "A guess" } as const;

  const times = (ratio: number) => (ratio >= 10 ? Math.round(ratio) : Math.round(ratio * 10) / 10);

  /** Where an estimate comes from, then what makes it less sure, one sentence each. */
  function reasons(spot: TrainingSpot, guess: Estimate): string[] {
    if (!speed) return [];
    const levels = speed.levels
      ? ` at Lv ${speed.levels[0]}${speed.levels[1] !== speed.levels[0] ? `–${speed.levels[1]}` : ""}`
      : "";
    const out = [
      `From ${formatDuration(speed.activeSeconds)} of your solo play on ${speed.maps} map${speed.maps === 1 ? "" : "s"}${levels}: ` +
        `about ${formatCompact(speed.hpPerHour)} monster HP an hour, so about ${formatCompact(guess.killsPerHour)} kills an hour here.`,
    ];
    if (guess.capped)
      out.push(
        `Held to what the map can respawn: ${formatCompact(guess.ceiling)} kills an hour if every monster is back ` +
          `after ${assumptions.value.respawnSeconds} s. That respawn time is an assumption you can change below.`,
      );
    if (guess.walkSecondsPerKill != null && speed.walkSecondsPerKill != null) {
      const here = guess.walkSecondsPerKill;
      const there = speed.walkSecondsPerKill;
      const layout = spot.layout;
      const spread = layout ? ` Monsters spawn on ${layout.spawnPlatforms} of its ${layout.platforms} platforms.` : "";
      out.push(
        `About ${here.toFixed(1)} s of walking between kills here, against ${there.toFixed(1)} s where you trained, ` +
          `from each map's floor and platforms at base walking speed.${spread}`,
      );
    }
    if (guess.hitsPerKill != null) {
      out.push(
        `About ${guess.hitsPerKill < 10 ? guess.hitsPerKill.toFixed(1) : Math.round(guess.hitsPerKill)} blows to defeat one here at your attack, ` +
          `counting its weapon defence of ${Math.round(guess.weaponDef ?? 0)}.`,
      );
    } else if ((guess.weaponDef ?? 0) - (speed.defence ?? 0) > 100) {
      out.push(
        `Its monsters have ${Math.round(guess.weaponDef ?? 0)} weapon defence, against about ${Math.round(speed.defence ?? 0)} where you trained. ` +
          "That can cut your damage a lot, so they may take far more blows than their HP says. Enter your attack under Assumptions to count it.",
      );
    }
    if (guess.hpRatio > 2)
      out.push(`Its monsters have ${times(guess.hpRatio)}× the HP of the ones you usually fight, so they may take longer than HP alone says.`);
    else if (guess.hpRatio < 0.5)
      out.push(`Its monsters have ${times(1 / guess.hpRatio)}× less HP than the ones you usually fight. If those already fall in one hit, this reads high.`);
    if (guess.levelGap != null && guess.levelGap > 3)
      out.push(`They average ${Math.round(guess.levelGap)} levels above you, so expect more misses.`);
    if (speed.widened)
      out.push("Little of your solo play is near your level, so older sessions count too. You likely kill faster now.");
    else if (speed.activeSeconds < 30 * 60) out.push("Under half an hour of play to go on.");
    return out;
  }

  function why(spot: TrainingSpot) {
    if (spot.measured) return `Your rate over ${formatDuration(spot.measured.activeSeconds)} of play here.`;
    if (estimate(spot)) return null;
    return "Not tried yet. Play ten minutes solo on any map and Leafline can estimate this one from your kill speed.";
  }
</script>

<Block id={SPOTS_ID} title="Where to train" {last}>
  {#snippet aside()}
    <span
      class="help"
      use:tip={"Where you have played, your own rate. Elsewhere, an estimate from how fast you get through monster HP solo, held to an assumed respawn time. Party bonus and misses aren't modelled."}
    >
      <CircleHelp size={13} />
    </span>
    {#if readLevel != null}
      <span class="tabular">For Lv {readLevel}</span>
    {:else}
      <span class="stepper">
        Lv
        <button aria-label="Lower level" onclick={() => setLevel((manualLevel ?? 2) - 1)}><Minus size={10} /></button>
        <input
          type="number"
          min="1"
          max="200"
          aria-label="Character level"
          value={manualLevel ?? ""}
          placeholder="?"
          onchange={(e) => setLevel(Number(e.currentTarget.value) || 1)}
        />
        <button aria-label="Raise level" onclick={() => setLevel((manualLevel ?? 0) + 1)}><Plus size={10} /></button>
      </span>
    {/if}
  {/snippet}

  {#if !ready}
    {#if mapData.status}<MapDataPanel compact />{/if}
  {:else if !data}
    <p class="quiet">Loading maps…</p>
  {:else}
    <Segmented options={sorts} value={sort} label="Rank by" stretch onchange={pickSort} />
    {#if spotStore.error}<p class="warn">{spotStore.error}</p>{/if}

    <ol class="spots">
      {#each shown as spot, i (spot.map.id)}
        {@const m = metric(spot)}
        {@const line = summary(spot)}
        {@const expanded = open === spot.map.id}
        <li>
          <div class="line">
            <!-- A div, not a button: the map and monster names inside are buttons of their own. -->
            <div
              class="row"
              role="button"
              tabindex="0"
              aria-expanded={expanded}
              onclick={() => (open = expanded ? null : spot.map.id)}
              onkeydown={(e) => rowKey(e, () => (open = expanded ? null : spot.map.id))}
            >
              <span class="rank tabular">{i + 1}</span>
              <span class="names">
                <span class="name"
                  ><Ref kind="map" id={spot.map.id} name={spot.map.name} /></span
                >
                <span class="meta" use:tip={summaryText(spot)}>
                  {#if steps(spot) === 0}<span class="here">You're here</span>{" · "}{/if}{#if sort === "played" && spot.measured}{formatDuration(spot.measured.activeSeconds)} played · {/if}{line.levels}{#each line.mobs as mob, n (mob.id)}{n === 0 ? " · " : ", "}<Ref kind="mob" id={mob.id} name={monsterLabel(mob.name)} icon={false} />{/each}
                </span>
              </span>
              <span class="metric">
                <span class="value num">{m.value}</span>
                <span class="unit" class:quiet={m.quiet}>
                  {#if m.sure}
                    <span class="sure" use:tip={SURENESS[m.sure]} aria-label={SURENESS[m.sure]}>
                      {#each [1, 2, 3] as step (step)}<i class:on={step <= m.sure}></i>{/each}
                    </span>
                  {/if}{m.unit}
                </span>
              </span>
              <span class="chevron" class:open={expanded}><ChevronRight size={12} /></span>
            </div>
            {#if steps(spot) !== 0}<span class="go"><GoButton place={mapPlace(spot.map)} /></span>{/if}
            <SaveSpot mapId={spot.map.id} name={spot.map.name} />
          </div>
          {#if expanded}
            {@const guess = estimate(spot)}
            <div class="detail">
              {#if guess}
                <div class="why">
                  <p><strong>{SURENESS[guess.sureness]}.</strong> {reasons(spot, guess)[0]}</p>
                  {#each reasons(spot, guess).slice(1) as line (line)}<p>{line}</p>{/each}
                </div>
              {:else}
                <p class="why">{why(spot)}</p>
              {/if}
              <MapNote mapId={spot.map.id} name={spot.map.name} flush />
              <table>
                <thead><tr><th>Monster</th><th>Lv</th><th>Count</th><th>EXP</th></tr></thead>
                <tbody>
                  {#each distinctMobs(spot.mobs) as mob (mob.id)}
                    <tr>
                      <td>
                        <Ref kind="mob" id={mob.id} name={monsterLabel(mob.name)} />{mob.boss ? " (boss)" : ""}
                      </td>
                      <td class="tabular">{mob.level}</td>
                      <td class="tabular">{mob.count}</td>
                      <td class="tabular">{formatCompact(mob.exp)}</td>
                    </tr>
                  {/each}
                </tbody>
              </table>
              <p class="facts tabular">{formatCompact(spot.expPerClear)} base EXP per clear · {spot.mobTotal} monsters</p>
              {#if data.from && route}
                <RouteSteps {route} />
              {:else if !data.from}
                <p class="quiet">Routes show once the minimap is in view.</p>
              {:else if spot.hops == null}
                <p class="quiet">No route through portals is known from {data.from.name}.</p>
              {/if}
            </div>
          {/if}
        </li>
      {:else}
        {#if sort === "played"}
          <li class="quiet empty">No maps played yet. Maps you train on show here with your own rate.</li>
        {:else if sort !== "saved"}
          <li class="quiet empty">
            {level != null ? `No maps with monsters near Lv ${level}. Tick Every level to see the rest.` : "Set your level to see maps that suit it."}
          </li>
        {:else if !missingSaved.length}
          <li class="quiet empty">No saved spots yet. Click the bookmark beside a map, here or on the Map tab, to keep it here.</li>
        {/if}
      {/each}
      {#each sort === "saved" ? missingSaved : [] as spot (spot.mapId)}
        <li>
          <div class="line">
            <span class="row static">
              <span class="rank"></span>
              <span use:clipped class="names"><span class="name">{spot.name}</span><span class="meta">Not in the map data any more</span></span>
            </span>
            <SaveSpot mapId={spot.mapId} name={spot.name} />
          </div>
        </li>
      {/each}
    </ol>

    <div class="footer">
      <label class="check" class:hidden={sort === "saved" || sort === "played"}>
        <input
          type="checkbox"
          checked={allLevels}
          onchange={(e) => {
            allLevels = e.currentTarget.checked;
            remember(ALL_LEVELS_KEY, String(allLevels));
          }}
        />
        Every level
      </label>
      {#if sort !== "saved" && ranked.length > SHOWN}
        <Button variant="ghost" size="sm" onclick={() => (more = !more)}>
          {more ? "Fewer" : `${Math.min(30, ranked.length) - SHOWN} more`}
        </Button>
      {/if}
      <span class="grow"></span>
      {#if sort === "you" || sort === "saved"}
        <Button variant="ghost" size="sm" pressed={editing} onclick={() => (editing = !editing)}>
          Assumptions{isDefault(assumptions.value) ? "" : " *"}
        </Button>
      {/if}
      <Button variant="primary" onclick={openTrainingLog}>Training log <ExternalLink size={12} /></Button>
    </div>
    {#if editing && (sort === "you" || sort === "saved")}
      <div class="assume">
        <p>Estimates take these as given. None is a known fact about the game; change them if you know better.</p>
        <label>
          <span>Monsters respawn after</span>
          <input
            type="number"
            min={RESPAWN_RANGE[0]}
            max={RESPAWN_RANGE[1]}
            step="1"
            value={assumptions.value.respawnSeconds}
            onchange={(e) => setAssumptions({ respawnSeconds: Number(e.currentTarget.value) })}
          />
          <span class="suffix">s</span>
        </label>
        <label>
          <span>EXP per kill, times base</span>
          <input
            type="number"
            min={EXP_RATE_RANGE[0]}
            max={EXP_RATE_RANGE[1]}
            step="0.1"
            value={assumptions.value.expRate}
            onchange={(e) => setAssumptions({ expRate: Number(e.currentTarget.value) })}
          />
          <span class="suffix">×</span>
        </label>
        <p>
          The respawn time only matters on maps where you'd defeat monsters faster than they come back. The EXP figure is
          used to count your kills from the EXP you gained.
        </p>
        <p>
          Your attack counts each monster's weapon defence, which can make a monster take many more blows than its HP says.
          {#if damage.value?.attackMin != null || damage.value?.skills.length}
            Leafline read these from your game; type over them to use your own.
          {:else}
            Open your Character Stat window with Detail showing, and the Skill window, and Leafline fills these in. Or type them.
          {/if}
        </p>
        <label>
          <span>Your attack</span>
          <input
            type="number"
            min={ATTACK_RANGE[0]}
            max={ATTACK_RANGE[1]}
            step="1"
            placeholder={damage.value?.attackMin?.toString() ?? "min"}
            aria-label="Lowest attack"
            value={assumptions.value.attackMin ?? ""}
            onchange={(e) => setAssumptions({ attackMin: e.currentTarget.value === "" ? null : Number(e.currentTarget.value) })}
          />
          <span class="suffix">–</span>
          <input
            type="number"
            min={ATTACK_RANGE[0]}
            max={ATTACK_RANGE[1]}
            step="1"
            placeholder={damage.value?.attackMax?.toString() ?? "max"}
            aria-label="Highest attack"
            value={assumptions.value.attackMax ?? ""}
            onchange={(e) => setAssumptions({ attackMax: e.currentTarget.value === "" ? null : Number(e.currentTarget.value) })}
          />
        </label>
        <label>
          <span>Main skill</span>
          <select
            aria-label="Main skill"
            value={assumptions.value.skillName}
            onchange={(e) => setAssumptions({ skillName: e.currentTarget.value })}
          >
            <option value="">{damage.value?.skills.length ? "Best one, automatically" : "Not read yet"}</option>
            {#each damage.value?.skills ?? [] as skill (skill.name)}
              <option value={skill.name}>{skill.name} · Lv {skill.level} · {skill.percent}%</option>
            {/each}
            <option value="custom">Other, I'll type it</option>
          </select>
        </label>
        {#if used.skill}
          <p>Counting <Ref kind="skill" name={used.skill.name} /> at Lv {used.skill.level}: {used.skill.percent}% of your attack.</p>
        {/if}
        {#if used.buffed}
          <p>
            With your buffs on, your attack counts as {used.assumptions.attackMin}–{used.assumptions.attackMax}. Turn buffs on or
            off in Character → Stats.
          </p>
        {/if}
        {#if assumptions.value.skillName === "custom" || !damage.value?.skills.length}
          <label>
            <span>Your skill hits for</span>
            <input
              type="number"
              min={SKILL_FACTOR_RANGE[0]}
              max={SKILL_FACTOR_RANGE[1]}
              step="0.1"
              value={assumptions.value.skillFactor}
              onchange={(e) => setAssumptions({ skillFactor: Number(e.currentTarget.value) })}
            />
            <span class="suffix">×</span>
          </label>
          <p>Your skill's damage as a multiple of a plain attack: its tooltip's Damage line, so 240% is 2.4.</p>
        {/if}
        {#if !isDefault(assumptions.value)}
          <Button variant="ghost" size="sm" onclick={() => setAssumptions(DEFAULT_ASSUMPTIONS)}>Back to defaults</Button>
        {/if}
      </div>
    {/if}
  {/if}
</Block>

<style>
  .go {
    display: inline-flex;
    align-items: center;
    flex: none;
  }

  .help {
    display: inline-grid;
    place-items: center;
    color: var(--faint);
    cursor: help;
  }

  .stepper {
    display: inline-flex;
    align-items: center;
    gap: 3px;
  }

  .stepper button {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    padding: 0;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    color: var(--text);
    cursor: pointer;
  }

  .stepper input {
    width: 34px;
    height: 20px;
    padding: 0 2px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
    text-align: center;
    -moz-appearance: textfield;
    appearance: textfield;
  }

  .stepper input::-webkit-inner-spin-button {
    -webkit-appearance: none;
  }

  .spots {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .spots > li {
    border-top: 1px solid var(--border);
  }

  .spots > li:first-child {
    border-top: 0;
  }

  .line {
    display: flex;
    align-items: center;
    gap: 2px;
  }

  .row {
    display: flex;
    flex: 1;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
    padding: var(--space-2) 0;
    border: 0;
    background: transparent;
    text-align: left;
    cursor: pointer;
  }

  .row.static {
    cursor: default;
  }

  .row:hover .name {
    color: var(--accent-strong);
  }

  .row:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  /* The names inside the row are links; the line still trims to one row. */
  .meta :global(.ref) {
    font-size: inherit;
  }

  .rank {
    width: 14px;
    flex: none;
    font-size: var(--text-xs);
    color: var(--faint);
  }

  .names {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-width: 0;
  }

  .name {
    overflow: hidden;
    font-size: var(--text-md);
    font-weight: 600;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .meta {
    overflow: hidden;
    font-size: var(--text-xs);
    color: var(--muted);
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .meta .here {
    color: var(--accent-strong);
  }

  .metric {
    display: flex;
    flex: none;
    flex-direction: column;
    align-items: flex-end;
  }

  .value {
    font-size: var(--text-lg);
    font-weight: 600;
    line-height: 1.2;
  }

  .unit {
    font-size: var(--text-xs);
    color: var(--muted);
  }

  .unit.quiet {
    color: var(--faint);
  }

  .chevron {
    display: inline-grid;
    color: var(--faint);
    transition: transform 0.12s ease;
  }

  .chevron.open {
    transform: rotate(90deg);
  }

  .detail {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 2px 0 var(--space-3) 22px;
  }

  .why,
  .facts {
    margin: 0;
    font-size: var(--text-xs);
    color: var(--muted);
  }

  div.why {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .why p {
    margin: 0;
  }

  .why strong {
    font-weight: 600;
    color: var(--text);
  }

  .sure {
    display: inline-flex;
    gap: 2px;
    margin-right: 5px;
    vertical-align: middle;
  }

  .sure i {
    width: 4px;
    height: 4px;
    border-radius: 50%;
    background: var(--border-strong);
  }

  .sure i.on {
    background: var(--muted);
  }

  .assume {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: var(--space-2);
    padding: var(--space-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface-2);
  }

  .assume p {
    margin: 0;
    font-size: var(--text-xs);
    color: var(--muted);
  }

  .assume label {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: var(--text-sm);
  }

  .assume label > span:first-child {
    flex: 1;
  }

  .assume select {
    max-width: 200px;
    height: 22px;
    padding: 0 4px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--text);
    font: inherit;
  }

  .assume input {
    width: 56px;
    height: 22px;
    padding: 0 4px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--text);
    font: inherit;
    text-align: right;
  }

  .assume .suffix {
    width: 10px;
    color: var(--faint);
  }

  .assume :global(button) {
    align-self: flex-start;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--text-sm);
  }

  th {
    padding: 0 0 4px;
    font-size: var(--text-xs);
    font-weight: 400;
    color: var(--faint);
    text-align: right;
  }

  td {
    padding: 4px 0;
    border-top: 1px solid var(--border);
    text-align: right;
  }

  th:first-child,
  td:first-child {
    text-align: left;
  }


  .footer {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .grow {
    flex: 1;
  }

  .check {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: var(--text-sm);
    color: var(--muted);
    cursor: pointer;
  }

  .check.hidden {
    visibility: hidden;
  }

  .check input {
    margin: 0;
    accent-color: var(--accent);
  }

  .quiet {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--faint);
  }

  .empty {
    padding: var(--space-3) 0;
  }

  .warn {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--warn);
  }

  @media (prefers-reduced-motion: reduce) {
    .chevron {
      transition: none;
    }
  }
</style>
