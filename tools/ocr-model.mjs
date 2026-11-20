// Puts ocrs's text recognition model into the built website (build-web/ocr/), where the live
// reading worker loads it to read the minimap title (crates/leafline-capture/src/title.rs). It is
// downloaded from ocrs's own model store once, checked against the SHA-256 below, and kept in
// target/ocr/ for later builds: never committed. ocrs is MIT/Apache-2.0; its models are trained on
// HierText (CC BY-SA 4.0). Run by tools/build-web.mjs.
import { createHash } from "node:crypto";
import { copyFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const NAME = "text-recognition.onnx";
const URL = `https://ocrs-models.s3-accelerate.amazonaws.com/${NAME}`;
const SHA256 = "86c145c2edb96c8caed5b1ebb8f44d706408922211451c309c625157dd6061c5";

const sha = (bytes) => createHash("sha256").update(bytes).digest("hex");
const kept = join(root, "target/ocr", NAME);

export async function placeOcrModel(site = join(root, "build-web")) {
  if (!existsSync(kept) || sha(readFileSync(kept)) !== SHA256) {
    console.log(`Downloading ${URL}`);
    const response = await fetch(URL);
    if (!response.ok) throw new Error(`${URL}: ${response.status}`);
    const bytes = Buffer.from(await response.arrayBuffer());
    if (sha(bytes) !== SHA256) throw new Error(`${NAME} doesn't match its SHA-256; not used`);
    mkdirSync(dirname(kept), { recursive: true });
    writeFileSync(kept, bytes);
  }
  mkdirSync(join(site, "ocr"), { recursive: true });
  copyFileSync(kept, join(site, "ocr", NAME));
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) await placeOcrModel();
