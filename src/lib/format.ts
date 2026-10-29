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
