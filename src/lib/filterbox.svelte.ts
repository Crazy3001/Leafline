// Whether a list's filter box is showing, remembered on this PC: a player who opens the filters
// keeps them, through Clear and the next start, until they close them.
const PREFIX = "leafline.filters.";

export class FilterBoxState {
  #open = $state(false);
  readonly #key: string;

  constructor(name: string, initial = false) {
    this.#key = PREFIX + name;
    let stored: string | null = null;
    try {
      stored = localStorage.getItem(this.#key);
    } catch {
      // A convenience only.
    }
    this.#open = stored == null ? initial : stored === "1";
  }

  get open() {
    return this.#open;
  }

  set open(value: boolean) {
    this.#open = value;
    try {
      localStorage.setItem(this.#key, value ? "1" : "0");
    } catch {
      // A convenience only.
    }
  }
}
