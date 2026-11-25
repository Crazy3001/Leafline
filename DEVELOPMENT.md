# Working on Leafline

Leafline is a [Tauri 2](https://tauri.app) app: a Rust backend that captures and reads the game window, and a Svelte frontend for its windows. The same pages and Rust also build into a website. The program is Windows only for now.

## Build and run

Prerequisites: [Rust](https://rustup.rs), [Node.js](https://nodejs.org) 20+, the Visual Studio C++ build tools, and WebView2 (built into Windows 11).

```bash
npm install
npm run tauri dev      # development build with hot reload
npm run tauri build    # installers in target/release/bundle/
```

`npm run dev` serves the UI in a plain browser with a mock backend that returns a saved analysis and session, which is handy for design work. The mock is left out of production builds.

Releases are built by `.github/workflows/release.yml`: bump the version in `tauri.conf.json`, every crate's `Cargo.toml` and `package.json` on main (with the lock files), then run the workflow by hand from the Actions tab. It attaches both installers and `SHA256SUMS.txt` to a draft release tagged `v<version>`; check the draft's tag before publishing it.

## The website

The website is the same pages and Rust, built for the browser. `crates/leafline-core` holds everything the program and the website share; `src-tauri` adds the program's windows, tray, hotkeys and live reading; `crates/leafline-web` runs core in a Web Worker, with the records in the browser's storage for the site (OPFS); `crates/leafline-capture` runs the readers and session tracker on a shared game window's frames in another worker. Commands are plain functions in `crates/leafline-core/src/service/`; after changing one's arguments, run `python tools/gen-commands.py`, which rewrites the program's Tauri wrappers and the website's dispatcher.

Prerequisites: `rustup target add wasm32-unknown-unknown`, `cargo install wasm-bindgen-cli --version 0.2.130` (the same version as the crate), and a clang that can build C for WebAssembly (SQLite is C). On Windows, `tools/build-wasm.mjs` finds the one in .NET's WebAssembly tools; otherwise set `CC_wasm32_unknown_unknown`.

```bash
npm run build:web      # the site in build-web/, served from /Leafline
npm run dev:web        # the site at http://localhost:1433/Leafline/ (run npm run build:wasm after changing Rust)
```

`.github/workflows/pages.yml` publishes it to GitHub Pages; it is run by hand from the Actions tab.

### Capture test (website stage 2)

How the website's readers were checked against the program's on a window shared to Firefox, Chrome or Edge. The shared parts are in `crates/leafline-core/src/capturetest/`; `crates/leafline-capture` carries them to the browser (the same WebAssembly as live reading, built for speed), and `src-tauri/src/app/capturetest.rs` is the program's half.

- **Program:** a dev build's Settings › Feedback › Capture test opens a window with the steps. It shows the test pattern (`/test/pattern`, pixel for pixel), checks it with the program's own capture, saves the game's frames as `app-<unix ms>.png` into a folder you pick, and compares them with the website's.
- **Website:** `/test/capture` shares a window and measures it against the pattern (pixel format, size, error per channel, colour bleed beside 1-px lines, the game fonts read back), saves frames as `web-<unix ms>.png` on the same clock ticks, and logs a 15-minute endurance run minute by minute. Frames come from the stream, not timers; nothing is uploaded.
- **Comparison from a terminal:** `cargo run --release --example capturecompare -- <folder>` reads both sides' frames with the same readers, pairs them within 0.1 s and prints each reader's agreement; the report is written into the folder as `leafline-capture-report.json`.

Frames are pictures of the game: keep the folder outside OneDrive and never commit them.

## Testing the readers

Put screenshots in `test-screenshots/`. It's git-ignored: game screenshots are Nexon's images and must never be committed or shipped. Then:

```bash
cd src-tauri
cargo test                                                      # unit tests
cargo run --release --example check                             # score readers against expected.json
cargo run --release --example analyze -- ../test-screenshots/   # readings as JSON
cargo run --release --example replay -- ../test-screenshots/    # replay a folder as a session
```

`check` needs a `test-screenshots/expected.json` that maps each file to what the screenshot really shows (see `examples/check.rs`). It prints every mismatch and a score. For `analyze`, add `--overview` to include every line from the whole-frame OCR pass.

The website's readers have their own examples in `crates/leafline-capture/examples/` (`webreaders`, `webwindows`, `heldreads`, `pickupcmp`, `livetrack`), which read the same saved frames both ways and print where they agree.

`replay` prints what each frame read and the session summary as JSON. Frames are ordered by file time by default, which suits screenshots saved by hand (copying files can reset their times, so replay the folder they were saved to). Add `--every <seconds>` to order by name a fixed interval apart, which suits frames taken from a recording, for example `ffmpeg -i grind.mp4 -vf fps=1 frame_%05d.png`.

CI (`.github/workflows/ci.yml`) runs the unit tests, clippy with warnings denied, and `svelte-check`. The accuracy check stays local, because its screenshots never go on GitHub.

## How it works

```
frame ─┬─ HUD reader ──────── finds the HP/MP/EXP gauges by colour; numbers by shape rules,
       │                     job and name by the game font's letters
       ├─ meso reader ─────── white box beside a gold coin; digits by their shapes, OCR as fallback
       ├─ map reader ──────── anchored on the minimap's orange WORLD button; reads the expanded
       │                     header or the collapsed title bar, four OCR passes voting per word
       ├─ party reader ────── counts HP bars in an evenly spaced column; never reads names
       ├─ shop reader ─────── an NPC shop, storage or trade window, by its column of buttons
       ├─ window readers ──── Character Stat, Skill, Item Inventory and Quest windows, read
       │                     once each holds still
       ├─ Pickup Log ──────── the corner that lists what was picked up, by its pixel font
       └─ overview OCR ─┬──── tooltip reader ── finds each tooltip by its separator rule; REQ rows
                        │                       by position, the rest read plain and colour-keyed
                        └──── quest reader ──── anchored on the "Quest Helper" header; objectives
                                                by the game font, titles by OCR

readings over time ─── session tracker ─── EXP and meso rates, per map, with misreads dropped
```

Every reader locates its UI element by appearance (colour, shape, nearby text) rather than fixed coordinates, and places the rest in units of the element's own size, so resolution should not matter. Readers prefer giving no value to giving a wrong one. The program's OCR is Windows' own; the website has none, so it reads by the game's letter shapes and uses [ocrs](https://github.com/robertknight/ocrs) for the minimap title, quest titles and item names.

### Live capture

Every few seconds Leafline takes a picture of the game window with Windows Graphics Capture, the API OBS and Discord use. The picture comes from the window itself, so the overlay and anything in front of it are never in it, and a covered game is still read. Leafline asks Windows only for the window's title, class, position and size, and never sends the game a message or opens its process. A picture that hasn't changed isn't read again, and full readings are at least a second apart. Between them, while **Count loot** is on, Leafline looks at the game four times a second and reads only the Pickup Log's corner (the lines that say what was picked up), since a line there starts fading at once and several can arrive within half a second. An item tooltip is read on the player's hotkey, or once the mouse has rested on an item for a moment, and only again when it changes. The Character Stat, Skill, Item Inventory and Quest windows are read while open, once they read the same twice; each has a switch in **Settings → Live reading**.

### Reading the HUD

The HUD's numbers use a tiny pixel font that general OCR misreads, so Leafline tells each character apart by rules about its shape: how many closed holes it has, whether its top or bottom row is solid, and which sides have ink partway down. Nothing has to be set up or learned. A shape that fits no rule is left unread rather than guessed. Other game text drawn the same way every time (mesos, the Pickup Log, the name and job, Quest Helper objectives) is matched against tables of the game font's letter shapes kept in the code. The game lays its UI out on a 1024×768 grid and scales that to the window, so Leafline puts the HUD back on that grid before reading it.

### Session tracking

OCR is sometimes wrong, so the tracker checks readings against each other rather than trusting each one:

- EXP points and percent must agree. Together they imply how much EXP the level needs, which can't change within a level, so a reading that implies a different amount is dropped. Tracking starts once two readings agree.
- Levels only go up, and a level-up counts once the percent wraps round or two readings agree on the new level.
- A meso amount counts once readings agree. In live play a big jump needs three and a tenfold one eight, unless the mesos have been out of sight for two minutes. Changes while a shop, storage or trade window is open are left out of the rates.
- EXP lost to a death is left out of the rates once two readings agree on the drop.
- At high levels the HUD shows only a percentage. EXP/hour then stays blank, and % per hour and time to level come from the percentage alone.
- Another character is noticed by a lower level or a different name, three readings in a row. The old session is saved and a new one starts.
- If the minimap's name is hidden for 45 seconds, gains stop going to the last map read.
- The clock pauses after 15 seconds without EXP (changeable in Settings), and at once on a map change without EXP, so time in town doesn't dilute the rates. A longer wait counts only a usual kill's time, as the hunt that ended it.
- Each gain is matched against the map's monster EXP to count kills. A gain far bigger than any kill there is quest EXP, shown apart and left out of the rates.
- A missing reading is skipped, never guessed. No EXP table is built in: each level's requirement is measured from the HUD, since Classic's table may differ from old GMS.

Live sessions save themselves every 30 seconds once they have a minute of active time. Records are copied to a `backups` folder once a day, and the last 7 copies are kept.

## Project layout

```
src/                       Svelte frontend, shared by the program and the website
  routes/                    one route per window: overlay, desk, pin, card, settings, setup;
                             test/ holds the capture test's pages
  lib/components/            cards and panels shared by the windows
  lib/ui/, lib/shell/        shared UI pieces and the overlay and desk frames
  lib/platform/              the backends: Tauri (program), web workers (website), and what each can do
  lib/capture/               the capture test's pages and worker
  lib/dev/                   the mock backend and sample data for `npm run dev`
crates/leafline-core/src/  everything the program and the website share
  readers/                   one module per game UI element: locate, read, parse
  vision/, capture/          image handling, OCR, and Windows Graphics Capture (program only)
  pipeline.rs                frame → readings, shared by live capture and the examples
  session.rs                 readings over time → EXP and meso rates
  mapdata/                   community data download, cache and lookups, routes, training forecasts
  userdata/                  the player's records (SQLite)
  build/                     the build planner
  service/                   the commands both backends call
crates/leafline-web/       core in the website's records worker (OPFS)
crates/leafline-capture/   the website's readers and session tracker on a shared window
src-tauri/src/app/         the program: windows, tray, hotkeys, live reading, Tauri command wrappers
tools/                     the website's build, the OCR model fetch, and gen-commands.py
```

## Ground rules

- Read-only, always: no memory reading, no packet inspection, no injection, no input sent to the game, no automation.
- No Nexon assets in the repo or a release, test fixtures included: use hand-made fake records.
- Community data is fetched at runtime and credited, never bundled.
- Nothing read from the game leaves the player's PC.
