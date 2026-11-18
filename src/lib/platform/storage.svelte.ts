// The website's link to its worker, which holds Leafline's storage in this browser. Only one tab
// can hold it: a second tab says so and can ask the first to hand it over.

import { emit } from "./bus";

type Reply = { id: number; value?: unknown; error?: string; fatal?: boolean };

/** Where the storage stands, for the banner that explains it. */
export const storage = $state<{
  state: "opening" | "open" | "in-use" | "moved" | "failed";
  problem: string | null;
}>({ state: "opening", problem: null });

let worker: Worker | null = null;
let next = 0;
const pending = new Map<number, { resolve: (value: unknown) => void; reject: (err: Error) => void }>();
let opened: Promise<void> | null = null;

const tabs = typeof BroadcastChannel === "undefined" ? null : new BroadcastChannel("leafline-tabs");

function start(): Worker {
  const w = new Worker(new URL("./web.worker.ts", import.meta.url), { type: "module" });
  w.onmessage = ({ data }: MessageEvent<Reply>) => {
    const waiting = pending.get(data.id);
    pending.delete(data.id);
    if (data.fatal) {
      // The Rust side panicked: start afresh on the next request.
      w.terminate();
      if (worker === w) {
        worker = null;
        opened = null;
      }
    }
    if (!waiting) return;
    if (data.error !== undefined) waiting.reject(new Error(data.error));
    else waiting.resolve(data.value);
  };
  w.onerror = (event) => {
    console.error("Leafline's worker failed:", event.message);
    storage.state = "failed";
    storage.problem = event.message || "Leafline couldn't start in this browser.";
  };
  return w;
}

function send<T>(message: Record<string, unknown>, transfer: Transferable[] = []): Promise<T> {
  worker ??= start();
  const id = ++next;
  return new Promise<T>((resolve, reject) => {
    pending.set(id, { resolve: resolve as (value: unknown) => void, reject });
    worker!.postMessage({ id, ...message }, transfer);
  });
}

/** Opens the storage once; later requests wait for it. Fails while another tab holds it. */
function ensureOpen(): Promise<void> {
  opened ??= (async () => {
    storage.state = "opening";
    try {
      const got = await send<boolean>({ op: "open", wait: false });
      if (!got) {
        storage.state = "in-use";
        throw new Error("Leafline is open in another tab of this browser.");
      }
      storage.state = "open";
      storage.problem = null;
      void navigator.storage?.persist?.();
    } catch (err) {
      if (storage.state !== "in-use") {
        storage.state = "failed";
        storage.problem = err instanceof Error ? err.message : String(err);
      }
      opened = null;
      throw err;
    }
  })();
  return opened;
}

/** Runs a command, as Tauri's `invoke` does, and passes on the events it raised. */
export async function invoke<T>(name: string, args?: Record<string, unknown>): Promise<T> {
  await ensureOpen();
  const reply = await send<string>({ op: "call", name, args: JSON.stringify(args ?? {}) });
  const { value, events } = JSON.parse(reply) as { value: T; events: [string, unknown][] };
  for (const [event, payload] of events) void emit(event, payload);
  return value;
}

/** Asks the tab holding the storage to let go, then takes it, and starts this page afresh. */
export async function takeOver(): Promise<void> {
  storage.state = "opening";
  tabs?.postMessage({ type: "hand-over" });
  await send<boolean>({ op: "open", wait: true });
  location.reload();
}

// Another tab asked for the storage: let go, and say where it went.
tabs?.addEventListener("message", async ({ data }: MessageEvent<{ type: string }>) => {
  if (data.type !== "hand-over" || storage.state !== "open") return;
  await send({ op: "close" });
  opened = null;
  storage.state = "moved";
});

/** A copy of every record, for the player to save as a file. */
export async function exportRecords(): Promise<Uint8Array> {
  await ensureOpen();
  return send<Uint8Array>({ op: "export" });
}

/** How much a records file holds, checked before anything is replaced. */
export async function checkRecords(bytes: Uint8Array): Promise<RecordsSummary> {
  await ensureOpen();
  return JSON.parse(await send<string>({ op: "check", bytes }));
}

/** Replaces the records with the file `checkRecords` took in. */
export async function importRecords(): Promise<RecordsSummary> {
  await ensureOpen();
  const { value, events } = JSON.parse(await send<string>({ op: "import" })) as {
    value: RecordsSummary;
    events: [string, unknown][];
  };
  for (const [event, payload] of events) void emit(event, payload);
  return value;
}

export async function recordsSummary(): Promise<RecordsSummary> {
  await ensureOpen();
  return JSON.parse(await send<string>({ op: "summary" }));
}

export interface RecordsSummary {
  sessions: number;
  drops: number;
  builds: number;
  notes: number;
  characters: number;
  spots: number;
  prices: number;
}
