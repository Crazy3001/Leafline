// Builds the website's WebAssembly (crates/leafline-web) and its JavaScript glue into
// src/lib/platform/wasm/, which the website's worker loads. Run by `npm run build:web`.
//
// Needs: the wasm32-unknown-unknown Rust target, wasm-bindgen-cli at the same version as the
// wasm-bindgen crate (0.2.130), and a clang that can build C for wasm32 (SQLite is C). Set
// CC_wasm32_unknown_unknown to it if `clang` on PATH can't; on Windows the clang that comes with
// .NET's WebAssembly tools is found by itself.
import { execFileSync } from "node:child_process";
import { existsSync, readdirSync, mkdirSync, statSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { gzipSync } from "node:zlib";
import { readFileSync } from "node:fs";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const out = join(root, "src/lib/platform/wasm");
const env = { ...process.env };

function dotnetClang() {
  const packs = "C:/Program Files/dotnet/packs";
  if (!existsSync(packs)) return null;
  for (const pack of readdirSync(packs).filter((p) => p.includes("Emscripten") && p.includes("Sdk"))) {
    for (const version of readdirSync(join(packs, pack)).reverse()) {
      const bin = join(packs, pack, version, "tools/bin");
      if (existsSync(join(bin, "clang.exe"))) return bin;
    }
  }
  return null;
}

if (!env.CC_wasm32_unknown_unknown && process.platform === "win32") {
  const bin = dotnetClang();
  if (bin) {
    env.CC_wasm32_unknown_unknown = join(bin, "clang.exe");
    env.AR_wasm32_unknown_unknown = join(bin, "llvm-ar.exe");
  }
}

const run = (cmd, args) => execFileSync(cmd, args, { cwd: root, env, stdio: "inherit" });
run("cargo", ["build", "--profile", "web", "--target", "wasm32-unknown-unknown", "-p", "leafline-web"]);
mkdirSync(out, { recursive: true });
const wasm = join(root, "target/wasm32-unknown-unknown/web/leafline_web.wasm");
run("wasm-bindgen", ["--target", "web", "--remove-name-section", "--remove-producers-section", "--out-dir", out, "--out-name", "leafline_web", wasm]);
const built = join(out, "leafline_web_bg.wasm");
const size = statSync(built).size;
const gzip = gzipSync(readFileSync(built), { level: 9 }).length;
console.log(`leafline_web_bg.wasm: ${(size / 1024).toFixed(0)} KB, ${(gzip / 1024).toFixed(0)} KB gzipped`);
