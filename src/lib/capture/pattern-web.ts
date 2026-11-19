// The test pattern drawn on the website by the capture test's WebAssembly, for a pattern window
// opened from the test page. Website only.

// Built by tools/build-wasm.mjs; not in the repository.
// @ts-ignore
import init, * as built from "./wasm/leafline_capture.js";

import type { WindowInfo } from "./types";

const wasm = built as { draw_pattern(width: number, height: number, scaleMilli: number): Uint8Array };
const ready = (init as () => Promise<unknown>)();

export async function patternPixels(info: WindowInfo): Promise<Uint8ClampedArray> {
  await ready;
  return new Uint8ClampedArray(wasm.draw_pattern(info.width, info.height, info.scaleMilli));
}
