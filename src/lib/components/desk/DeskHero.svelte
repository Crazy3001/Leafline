<script lang="ts">
  // The desk view's banner on Now: a greeting, where you are, and when the next level comes, over
  // the picture of the map you're on. Without one (pictures off, no map data, not matched yet) a
  // drawn scene tinted by the region stands in. Decoration that also tells you something.
  import { MapPin } from "@lucide/svelte";

  import { formatClockIn } from "$lib/format";
  import { ago, shownCharacter, watchProfile } from "$lib/profile.svelte";
  import { live } from "$lib/readings.svelte";
  import { place, regionHue, watchRegion } from "$lib/region.svelte";
  import Pic from "$lib/ui/Pic.svelte";

  watchRegion();
  watchProfile();

  // The character looked at, read now or remembered; with no map read now, the one they were last on.
  const who = $derived(shownCharacter());
  const session = $derived(who.onScreen ? live.session : null);
  const name = $derived(who.name);
  const level = $derived(who.level.value);
  const map = $derived(who.onScreen ? (live.analysis?.map ?? null) : null);
  const readMap = $derived(map?.name ?? session?.map?.split(" : ").pop() ?? null);
  const mapName = $derived(readMap ?? who.lastMap);
  const toLevel = $derived(session && !session.paused && session.activeSeconds > 0 ? session.secondsToLevel : null);
  const at = $derived(formatClockIn(toLevel));
  const hue = $derived(regionHue(place.region) ?? 142);
</script>

<section class="hero" style:--hue={hue} aria-label="Where you are">
  {#key place.mapId}
    {#if place.mapId != null}
      <span class="pic"><Pic kind="mapview" id={place.mapId} size={null} fit="cover">{#snippet fallback()}{@render scene()}{/snippet}</Pic></span>
    {:else}
      {@render scene()}
    {/if}
  {/key}
  <div class="words">
    <h1>{name ? `Welcome back, ${name}` : "Welcome to Leafline"}</h1>
    <p>
      {#if mapName}<span class="pill" class:stale={readMap == null}><MapPin size={13} />{mapName}</span>{/if}
      {#if place.region && readMap != null}<span class="pill">{place.region}</span>{/if}
      {#if toLevel != null && level != null && at}<span class="line">On track for Lv {level + 1} by {at}</span>
      {:else if readMap == null && who.seenAt != null}<span class="line">Last seen {ago(who.seenAt)}{level != null ? ` at Lv ${level}` : ""}.</span>
      {:else if !mapName}<span class="line">Share or start the game and Leafline follows along.</span>{/if}
    </p>
  </div>
</section>

{#snippet scene()}
  <svg class="scene" viewBox="0 0 800 140" preserveAspectRatio="xMidYMid slice" aria-hidden="true">
    <defs>
      <linearGradient id="hero-sky" x1="0" x2="0" y1="0" y2="1">
        <stop offset="0" stop-color="hsl({hue} 55% 62%)" />
        <stop offset="1" stop-color="hsl({hue} 45% 84%)" />
      </linearGradient>
    </defs>
    <rect width="800" height="140" fill="url(#hero-sky)" />
    <g fill="#fff" opacity="0.8"><ellipse cx="520" cy="30" rx="44" ry="12" /><ellipse cx="555" cy="24" rx="30" ry="12" /><ellipse cx="690" cy="44" rx="36" ry="10" /></g>
    <path d="M0 95 Q 120 60 260 88 T 520 80 T 800 70 V140 H0Z" fill="hsl({hue} 42% 52%)" />
    <path d="M0 118 Q 160 92 340 112 T 640 104 T 800 98 V140 H0Z" fill="hsl({hue} 45% 38%)" />
    <g><rect x="606" y="62" width="8" height="34" fill="#6b4a2b" /><circle cx="610" cy="54" r="24" fill="hsl({hue} 40% 34%)" /><circle cx="596" cy="62" r="16" fill="hsl({hue} 42% 40%)" /></g>
    <g><rect x="716" y="52" width="9" height="44" fill="#5e3f24" /><circle cx="720" cy="44" r="30" fill="hsl({hue} 44% 30%)" /><circle cx="742" cy="52" r="19" fill="hsl({hue} 40% 36%)" /></g>
  </svg>
{/snippet}

<style>
  .hero {
    position: relative;
    isolation: isolate;
    display: flex;
    align-items: center;
    min-height: 128px;
    padding: var(--space-5) var(--space-6);
    overflow: hidden;
    border-radius: var(--radius-lg);
    background: hsl(var(--hue) 40% 30%);
  }

  .pic,
  .scene {
    position: absolute;
    inset: 0;
    z-index: -2;
    width: 100%;
    height: 100%;
  }

  .pic :global(img) {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  /* Dark on the left, where the words are, whatever the picture is. */
  .hero::after {
    content: "";
    position: absolute;
    inset: 0;
    z-index: -1;
    background: linear-gradient(90deg, rgba(8, 14, 11, 0.85) 0%, rgba(8, 14, 11, 0.55) 45%, rgba(8, 14, 11, 0) 80%);
  }

  h1 {
    margin: 0;
    color: #fff;
    font-family: var(--font-display);
    font-size: var(--text-2xl);
    font-weight: var(--weight-strong);
    line-height: 1.15;
  }

  p {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
    margin: var(--space-2) 0 0;
    color: rgba(255, 255, 255, 0.9);
    font-size: var(--text-md);
  }

  .pill {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    padding: var(--space-0-5) var(--space-2);
    border-radius: var(--radius-pill);
    background: rgba(255, 255, 255, 0.16);
    color: #fff;
    font-size: var(--text-xs);
  }

  /* The map they were last on, not read now. */
  .pill.stale {
    background: rgba(255, 255, 255, 0.08);
    color: rgba(255, 255, 255, 0.75);
  }
</style>
