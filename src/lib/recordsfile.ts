// The records file, on the website: when this browser last saved one, since the browser holds the
// site's records and can clear them. And a note Settings shows after a restore reloads every
// window. Both stay in this browser.

const SAVED_KEY = "leafline-records-saved";

/** Settings' note after records were replaced, kept across the reload that follows. */
export const RECORDS_NOTE = "leafline-records-note";

/** When this browser last saved a records file, in milliseconds, or null. */
export function recordsSavedAt(): number | null {
  try {
    const at = Number(localStorage.getItem(SAVED_KEY));
    return Number.isFinite(at) && at > 0 ? at : null;
  } catch {
    return null;
  }
}

export function noteRecordsSaved() {
  try {
    localStorage.setItem(SAVED_KEY, String(Date.now()));
  } catch {
    // Not kept: the reminder just says it hasn't been saved.
  }
}
