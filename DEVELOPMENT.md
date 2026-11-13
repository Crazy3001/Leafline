# Working on Leafline

Leafline is a [Tauri 2](https://tauri.app) app: a Rust backend that captures and reads the game window, and a Svelte frontend for its windows. Windows only for now.

## Build and run

Prerequisites: [Rust](https://rustup.rs), [Node.js](https://nodejs.org) 20+, the Visual Studio C++ build tools, and WebView2 (built into Windows 11).

```bash
npm install
npm run tauri dev      # development build with hot reload
npm run tauri build    # installers in src-tauri/target/release/bundle/
```

`npm run dev` serves the UI in a plain browser with a mock backend that returns a saved analysis and session, which is handy for design work. The mock is left out of production builds.

Releases are built by `.github/workflows/release.yml`: push a tag such as `v0.1.0` that matches the version in `tauri.conf.json`, and it attaches both installers to a draft release.

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

`replay` prints what each frame read and the session summary as JSON. Frames are ordered by file time by default, which suits screenshots saved by hand (copying files can reset their times, so replay the folder they were saved to). Add `--every <seconds>` to order by name a fixed interval apart, which suits frames taken from a recording, for example `ffmpeg -i grind.mp4 -vf fps=1 frame_%05d.png`.

CI (`.github/workflows/ci.yml`) runs the unit tests, clippy with warnings denied, and `svelte-check`. The accuracy check stays local, because its screenshots never go on GitHub.

## How it works

```
frame ─┬─ HUD reader ──────── finds the HP/MP/EXP gauges by colour; numbers by shape rules,
       │                     job and name by OCR
       ├─ meso reader ─────── white box beside a gold coin; amount only when OCR readings agree
       ├─ map reader ──────── anchored on the minimap's orange WORLD button; reads the expanded
       │                     header or the collapsed title bar, four OCR passes voting per word
       ├─ party reader ────── counts HP bars in an evenly spaced column; never reads names
       ├─ shop reader ─────── an NPC shop, storage or trade window, by its column of buttons
       └─ overview OCR ─┬──── tooltip reader ── finds each tooltip by its separator rule; REQ rows
                        │                       by position, the rest read plain and colour-keyed
                        └──── quest reader ──── anchored on the "Quest Helper" header; three
                                                passes vote on each counter

readings over time ─── session tracker ─── EXP and meso rates, per map, with misreads dropped
```

Every reader locates its UI element by appearance (colour, shape, nearby text) rather than fixed coordinates, and places the rest in units of the element's own size, so resolution should not matter. Readers prefer giving no value to giving a wrong one.

### Live capture

Every few seconds Leafline takes a picture of the game window with Windows Graphics Capture, the API OBS and Discord use. The picture comes from the window itself, so the overlay and anything in front of it are never in it, and a covered game is still read. Leafline asks Windows only for the window's title, class, position and size, and never sends the game a message or opens its process. A picture that hasn't changed isn't read again, and pictures are at least a second apart. Item tooltips are read only on the player's hotkey, from a single picture.

### Reading the HUD

The HUD's numbers use a tiny pixel font that general OCR misreads, so Leafline tells each character apart by rules about its shape: how many closed holes it has, whether its top or bottom row is solid, and which sides have ink partway down. Nothing has to be set up or learned, and no font data ships with the app. A shape that fits no rule is left unread rather than guessed. The game lays its UI out on a 1024×768 grid and scales that to the window, so Leafline puts the HUD back on that grid before reading it.

### Session tracking

OCR is sometimes wrong, so the tracker checks readings against each other rather than trusting each one:

- EXP points and percent must agree. Together they imply how much EXP the level needs, which can't change within a level, so a reading that implies a different amount is dropped. Tracking starts once two readings agree.
- Levels only go up, and a level-up counts once the percent wraps round or two readings agree on the new level.
- A meso amount counts once readings agree. In live play a big jump needs three and a tenfold one eight, unless the mesos have been out of sight for two minutes. Changes while a shop, storage or trade window is open are left out of the rates.
- EXP lost to a death is left out of the rates once two readings agree on the drop.
- At high levels the HUD shows only a percentage. EXP/hour then stays blank, and % per hour and time to level come from the percentage alone.
- Another character is noticed by a lower level or a different name, three readings in a row. The old session is saved and a new one starts.
- If the minimap's name is hidden for 45 seconds, gains stop going to the last map read.
- The clock pauses after 15 seconds without EXP (changeable in Settings), and at once on a map change without EXP, so time in town doesn't dilute the rates.
- A missing reading is skipped, never guessed. No EXP table is built in: each level's requirement is measured from the HUD, since Classic's table may differ from old GMS.

Live sessions save themselves every 30 seconds once they have a minute of active time. Records are copied to a `backups` folder once a day, and the last 7 copies are kept.

## Project layout

```
src/                     Svelte frontend
  lib/components/          cards shared by the windows
  lib/ui/                  shared UI pieces
  routes/                  one route per window: overlay, desk (tabs, with the training log and build planner), pin, settings, setup
src-tauri/src/
  app/                     Tauri wiring: windows, tray, hotkeys, commands
  capture/                 live pictures of the game window (Windows Graphics Capture)
  vision/                  image handling, colour blocks, OCR (Windows.Media.Ocr)
  readers/                 one module per game UI element: locate, read, parse
  pipeline.rs              frame → readings, shared by live capture and the examples
  session.rs               readings over time → EXP and meso rates
  replay.rs                a folder of screenshots fed through both, in order (examples only)
  mapdata/                 community data download, cache and lookups
  userdata/                the player's records (SQLite)
  build/                   the build planner
```

## Ground rules

- Read-only, always: no memory reading, no packet inspection, no injection, no input sent to the game, no automation.
- No Nexon assets in the repo or a release, test fixtures included: use hand-made fake records.
- Community data is fetched at runtime and credited, never bundled.
- Nothing read from the game leaves the player's PC.
