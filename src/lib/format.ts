const grouped = new Intl.NumberFormat("en-US");

/**
 * How far along a quest objective is, from 0 to 1. Items to use up count down to a target of 0
 * (`1 / 0 Roger's Apple`, then `0 / 0`), so those are done at 0.
 */
export function objectiveProgress({ current, required }: { current: number; required: number }): number {
  return required === 0 ? Number(current === 0) : Math.min(1, current / required);
}

export function formatNumber(value: number | null | undefined): string {
  return value == null ? "—" : grouped.format(value);
}

export function formatPercent(value: number | null | undefined): string {
  return value == null ? "—" : `${value.toFixed(2)}%`;
}

export function formatSigned(value: number): string {
  return value >= 0 ? `+${value}` : `${value}`;
}

export function formatMs(ms: number): string {
  return ms >= 1000 ? `${(ms / 1000).toFixed(2)} s` : `${Math.round(ms)} ms`;
}

/** Title case for job flags such as `MAGICIAN`. */
export function titleCase(text: string): string {
  return text.charAt(0) + text.slice(1).toLowerCase();
}

/** Large amounts in short form: `22.6k`, `1.35M`. */
export function formatCompact(value: number | null | undefined): string {
  if (value == null) return "—";
  const size = Math.abs(value);
  if (size >= 1e6) return `${(value / 1e6).toFixed(size >= 1e7 ? 1 : 2)}M`;
  if (size >= 1e4) return `${(value / 1e3).toFixed(size >= 1e5 ? 0 : 1)}k`;
  return grouped.format(Math.round(value));
}

/** A span of time: `45s`, `12m`, `2h 05m`. */
export function formatDuration(seconds: number | null | undefined): string {
  if (seconds == null || !Number.isFinite(seconds)) return "—";
  const total = Math.max(0, Math.round(seconds));
  if (total < 60) return `${total}s`;
  const minutes = Math.floor(total / 60);
  if (minutes < 60) return `${minutes}m`;
  return `${Math.floor(minutes / 60)}h ${String(minutes % 60).padStart(2, "0")}m`;
}

/** A map as the collapsed minimap writes it, `Victoria Road : Henesys`, which is how the session tracker keys maps. */
export function mapLabel(map: { street: string | null; name: string }): string {
  return map.street ? `${map.street} : ${map.name}` : map.name;
}

/** How far a map is from the player: portals, and steps with any travel turned on. */
export function groundDistance(ground: { hops: number | null; travel: { hops: number } | null }, located: boolean): string {
  if (ground.hops === 0) return "you're here";
  const walk = ground.hops == null ? "no portal route" : `${ground.hops} portal${ground.hops === 1 ? "" : "s"}`;
  if (ground.travel) return `${walk} · ${ground.travel.hops} with travel`;
  return ground.hops == null && !located ? "" : walk;
}

/** A party size as the log shows it: `Solo`, `4 ppl`. Null when it wasn't read. */
export function partyLabel(size: number | null | undefined): string | null {
  if (size == null) return null;
  return size <= 1 ? "Solo" : `${size} ppl`;
}

/** The street and name of a map label, for two-line display. */
export function splitMapLabel(label: string): { street: string | null; name: string } {
  const at = label.indexOf(" : ");
  return at < 0 ? { street: null, name: label } : { street: label.slice(0, at), name: label.slice(at + 3) };
}

/**
 * A price as typed: `120000`, `120,000`, `120k`, `1.2m`. Null when it isn't one.
 */
export function parsePrice(text: string): number | null {
  const match = text.trim().toLowerCase().replace(/[,\s_]/g, "").match(/^(\d+(?:\.\d+)?)([kmb]?)$/);
  if (!match) return null;
  const scale = { "": 1, k: 1e3, m: 1e6, b: 1e9 }[match[2] as "" | "k" | "m" | "b"];
  const value = Math.round(Number(match[1]) * scale);
  return value > 0 && Number.isSafeInteger(value) ? value : null;
}

/** A day as `29 Oct`, for logged records. */
export function formatDay(unixSeconds: number): string {
  return new Date(unixSeconds * 1000).toLocaleDateString("en-GB", { day: "numeric", month: "short" });
}

/** Mesos in short form without a trailing `.0`: `85k`, `1.2M`, `950`. */
export function formatMesos(value: number | null | undefined): string {
  return formatCompact(value).replace(/\.0(?=[kM])/, "");
}
