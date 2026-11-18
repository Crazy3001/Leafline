// Game pictures on the website: the page asks the browser for each one at its address in the map
// data (OSMS publishes them), and the browser keeps them in its cache. The worker hands over the
// table of addresses once the map data is there. A missing picture leaves the name alone.

interface PictureTable {
  urls: Record<string, Record<string, string>>;
  names: Record<string, Record<string, number>>;
}

let table = $state.raw<PictureTable | null>(null);

export function setPictureTable(next: PictureTable | null) {
  table = next;
}

/** The address of a picture by id, or by name; null when it isn't known (yet). */
export function webPictureUrl(kind: string, key: number | string): string | null {
  if (!table) return null;
  const id = typeof key === "number" ? key : table.names[kind]?.[key.trim().toLowerCase()];
  return id == null ? null : (table.urls[kind]?.[String(id)] ?? null);
}
