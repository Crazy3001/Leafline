const grouped = new Intl.NumberFormat("en-US");

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

/** The street and name of a map label, for two-line display. */
export function splitMapLabel(label: string): { street: string | null; name: string } {
  const at = label.indexOf(" : ");
  return at < 0 ? { street: null, name: label } : { street: label.slice(0, at), name: label.slice(at + 3) };
}
