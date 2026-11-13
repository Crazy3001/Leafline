// How shop prices are dated on screen. Every entry in Leafline's shop file carries the day its
// price was last seen; prices can change after launch, so the date always shows.

/** "Nov 10" for a `YYYY-MM-DD` day, or the text as it is when it isn't one. */
export function shortDate(day: string): string {
  const [y, m, d] = day.split("-").map(Number);
  if (!y || !m || !d) return day;
  return new Date(y, m - 1, d).toLocaleDateString("en-GB", { day: "numeric", month: "short" });
}

/** "Prices updated 10 Nov", or a range when the entries were seen on different days. */
export function seenLine(days: string[]): string | null {
  const sorted = [...new Set(days)].sort();
  if (!sorted.length) return null;
  const first = shortDate(sorted[0]);
  const last = shortDate(sorted[sorted.length - 1]);
  return first === last ? `Prices updated ${first}` : `Prices updated ${first} to ${last}`;
}
