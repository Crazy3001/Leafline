// Builds the website into build-web/ (the WebAssembly first, then the pages in Vite's "web" mode),
// or with --dev serves it for working on (http://localhost:1433/Leafline/). The wasm is built only
// when it isn't there yet in --dev; run `npm run build:wasm` after changing Rust.
import { execFileSync } from "node:child_process";
import { existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const dev = process.argv.includes("--dev");
const env = { ...process.env, LEAFLINE_TARGET: "web" };
const vite = join(root, "node_modules/vite/bin/vite.js");

if (!dev || !existsSync(join(root, "src/lib/platform/wasm/leafline_web.js"))) {
  execFileSync(process.execPath, [join(root, "tools/build-wasm.mjs")], { cwd: root, stdio: "inherit" });
}
const args = dev ? [vite, "dev", "--mode", "web", "--port", "1433", "--strictPort"] : [vite, "build", "--mode", "web"];
execFileSync(process.execPath, args, { cwd: root, env, stdio: "inherit" });
// The minimap title's OCR model, beside the pages (see tools/ocr-model.mjs).
if (!dev) await (await import("./ocr-model.mjs")).placeOcrModel();
