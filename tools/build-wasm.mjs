// Builds the website's WebAssembly and its JavaScript glue: crates/leafline-web into
// src/lib/platform/wasm/, which the website's storage worker loads, and crates/leafline-capture
// into src/lib/capture/wasm/, which only the capture test page loads. Run by `npm run build:web`.
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

const run = (cmd, args, extra = {}) => execFileSync(cmd, args, { cwd: root, env: { ...env, ...extra }, stdio: "inherit" });
// The storage worker is built small; the capture test's readers fast (see Cargo.toml's profiles).
// The readers also use WebAssembly SIMD, which ocrs's text recognition is several times quicker
// with: every browser that can share a window has it. They get a target folder of their own, so
// the two builds' different flags don't rebuild each other's dependencies. The storage worker,
// which every browser loads, stays without.
const targetDir = env.CARGO_TARGET_DIR ?? join(root, "target");
const builds = [
  ["leafline-web", "web", "src/lib/platform/wasm", targetDir, {}],
  [
    "leafline-capture",
    "web-fast",
    "src/lib/capture/wasm",
    join(targetDir, "simd"),
    { CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS: "-C target-feature=+simd128" },
  ],
];
for (const [crate, profile, , dir, extra] of builds) {
  run("cargo", ["build", "--profile", profile, "--target", "wasm32-unknown-unknown", "-p", crate, "--target-dir", dir], extra);
}

for (const [name, profile, folder, dir] of builds) {
  const crate = name.replace("-", "_");
  const out = join(root, folder);
  mkdirSync(out, { recursive: true });
  const wasm = join(dir, `wasm32-unknown-unknown/${profile}/${crate}.wasm`);
  run("wasm-bindgen", ["--target", "web", "--remove-name-section", "--remove-producers-section", "--out-dir", out, "--out-name", crate, wasm]);
  const built = join(out, `${crate}_bg.wasm`);
  const size = statSync(built).size;
  const gzip = gzipSync(readFileSync(built), { level: 9 }).length;
  console.log(`${crate}_bg.wasm: ${(size / 1024).toFixed(0)} KB, ${(gzip / 1024).toFixed(0)} KB gzipped`);
}
