// The website's worker: Leafline's Rust (crates/leafline-web) with its storage. One tab at a time
// holds the storage, under a Web Lock; another tab asks this one to let go (see storage.svelte.ts).

// Built by tools/build-wasm.mjs, which `npm run build:web` runs first. Not in the repository, so the
// type check sees it only once built; what it offers is written out below.
// @ts-ignore
import init, * as built from "./wasm/leafline_web.js";

const wasm = built as {
  open(): Promise<string>;
  close(): void;
  call(name: string, args: string): string;
  export_records(): Uint8Array;
  check_records(bytes: Uint8Array): string;
  import_records(): string;
  records_summary(): string;
};

type Request =
  | { id: number; op: "open"; wait: boolean }
  | { id: number; op: "close" }
  | { id: number; op: "call"; name: string; args: string }
  | { id: number; op: "export" }
  | { id: number; op: "check"; bytes: Uint8Array }
  | { id: number; op: "import" }
  | { id: number; op: "summary" };

const ready = (init as () => Promise<unknown>)();
/** Lets go of the Web Lock. Set while this worker holds the storage. */
let release: (() => void) | null = null;

/** Takes the lock (waiting for it with `wait`), then opens the storage. False when another tab has it. */
async function open(wait: boolean): Promise<boolean> {
  if (release) return true;
  const got = await new Promise<boolean>((answer) => {
    void navigator.locks
      .request("leafline-records", wait ? {} : { ifAvailable: true }, (lock) => {
        if (!lock) {
          answer(false);
          return;
        }
        answer(true);
        return new Promise<void>((done) => (release = done));
      })
      .catch(() => answer(false));
  });
  if (!got) return false;
  try {
    await wasm.open();
    return true;
  } catch (err) {
    // Set by the lock's callback, which the type check can't see.
    (release as (() => void) | null)?.();
    release = null;
    if (String(err).startsWith("in-use")) return false;
    throw err;
  }
}

async function run(request: Request): Promise<unknown> {
  await ready;
  switch (request.op) {
    case "open":
      return open(request.wait);
    case "close":
      if (release) {
        wasm.close();
        release();
        release = null;
      }
      return null;
    case "call":
      return wasm.call(request.name, request.args);
    case "export":
      return wasm.export_records();
    case "check":
      return wasm.check_records(request.bytes);
    case "import":
      return wasm.import_records();
    case "summary":
      return wasm.records_summary();
  }
}

self.onmessage = async ({ data }: MessageEvent<Request>) => {
  try {
    const value = await run(data);
    self.postMessage({ id: data.id, value });
  } catch (err) {
    // A Rust panic leaves the module unusable: the page starts a new worker.
    const fatal = err instanceof WebAssembly.RuntimeError;
    const message = err instanceof Error ? err.message : String(err);
    self.postMessage({ id: data.id, error: message, fatal });
  }
};
