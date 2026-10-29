# Leafline

An unofficial, read-only overlay for **MapleStory Classic World**: price checks, EXP tracking, routes and quest help, read from what is visible on screen.

> The project is not affiliated with or endorsed by Nexon.

## What it does, and what it never does

Leafline only looks at your screen. It captures what is visible, reads it with OCR and shows information in a separate always-on-top window.

It never reads or writes game memory or files, never touches network traffic, never injects into or draws inside the game client, and never sends keyboard or mouse input to the game. There is no automation of any kind. No Nexon assets are included.

**Current status: offline mode only.** Until Nexon confirms that read-only overlays are allowed, Leafline reads saved screenshots and never runs alongside the live game.

## Getting started (Windows)

Prerequisites: [Rust](https://rustup.rs), [Node.js](https://nodejs.org) 20+, Visual Studio C++ build tools, and WebView2 (built into Windows 11).

```bash
npm install
npm run tauri dev
```

Two windows open:

- **Overlay**: the compact, transparent, always-on-top panel. Drag it by its title bar.
- **Workbench**: offline test mode. Open a screenshot or a folder of them; the pipeline runs on each, outlines what it found and shows the readings. The overlay updates too, as it would during play. With a folder open, **Replay as session** reads every frame in order as one play session and shows EXP/hour, mesos/hour and time to level, overall and per map (see below).

You can also pass an image on the command line (`leafline.exe shot.png`), or use *Open with*.

| Hotkey | Action |
|---|---|
| `Ctrl+Alt+O` | Show or hide the overlay |
| `Ctrl+Alt+L` | Lock the overlay (clicks pass through to whatever is underneath) |
| `Ctrl+Alt+W` | Open the workbench |

Closing a window hides it. Leafline keeps running in the tray until you choose **Quit**.

### Teaching it your HUD

The HUD's numbers use a tiny pixel font that general OCR misreads, so Leafline reads them by matching character shapes it learns from your own screen. Open a screenshot in the workbench, type the values your HUD shows (level, name, HP, MP, EXP and EXP %) under **Teach the HUD**, and choose **Learn from this screenshot**. A second screenshot with different numbers fills in any digits the first one did not show. The shapes are saved in the app's data folder and never leave your PC. Re-teach after changing resolution or UI size.

### Session tracking

The session tracker turns a run of readings into rates: EXP gained, EXP/hour, % per hour, time to the next level, mesos gained and mesos/hour, overall and per map. Offline, it runs on a folder of screenshots:

- **File time** orders frames by each file's modified time and takes that as the moment it shows. Suits screenshots saved by hand. Copying files can reset their times, so replay the folder they were saved to.
- **File name** orders frames by name, a fixed number of seconds apart. Suits frames extracted from a recording (for example `ffmpeg -i grind.mp4 -vf fps=1 frame_%05d.png`).

OCR is sometimes wrong, so the tracker checks readings against each other rather than trusting each one:

- EXP points and percent must agree. Together they imply how much EXP the level needs, which cannot change within a level, so a reading that implies a different amount is dropped as a misread. Tracking starts once two readings agree.
- Levels only go up, and a level-up counts once the percent wraps round or two readings agree on the new level.
- A meso amount counts once readings agree (two in a row for recordings; a single screenshot has to do for hand-taken ones). Changes while a storage or trade window is open (a second meso box on screen) are left out of the rates.
- The clock pauses after 15 seconds without EXP, and at once on a map change without EXP, so time in town does not dilute the rates. Rates appear after 30 seconds of active time.
- A missing reading is skipped, never guessed. No EXP table is built in: each level's requirement is measured from the HUD, since Classic's table may differ from old GMS.

## Testing the readers

Put screenshots in `test-screenshots/` (git-ignored: game screenshots must never be committed or shipped), then:

```bash
cd src-tauri
cargo test                                                      # unit tests
cargo run --release --example check                             # score readers against expected.json
cargo run --release --example analyze -- ../test-screenshots/   # readings as JSON
cargo run --release --example replay -- ../test-screenshots/ --font ../test-screenshots/hud-font.json
                                                                # replay a folder as a session
```

`check` needs a `test-screenshots/expected.json` that maps each file to what the screenshot really shows (see `examples/check.rs`). It teaches the HUD from the files listed under `_calibrate`, saves that as `test-screenshots/hud-font.json`, then prints every mismatch and a score. For `analyze`, add `--font ../test-screenshots/hud-font.json` to read HUD numbers and `--overview` to include every line from the whole-frame OCR pass.

`replay` prints what each frame read and the session summary as JSON; add `--every <seconds>` to order by name instead of file time.

`npm run dev` serves the UI in a plain browser with a mock backend that returns a saved analysis and session, which is handy for design work. The mock is excluded from production builds.

## How it works

```
frame ─┬─ HUD reader ──────── finds the HP/MP/EXP gauges by colour; numbers by learned glyphs,
       │                     job and name by OCR (calibrated names by glyphs)
       ├─ meso reader ─────── white box beside a gold coin; amount only when OCR readings agree
       ├─ map reader ──────── anchored on the minimap's orange WORLD button; reads the expanded
       │                     header or the collapsed title bar, four OCR passes voting per word
       └─ overview OCR ─┬──── tooltip reader ── finds each tooltip by its separator rule; REQ rows
                        │                       by position, the rest read plain and colour-keyed
                        └──── quest reader ──── anchored on the "Quest Helper" header; three
                                                passes vote on each counter

readings over time ─── session tracker ─── EXP and meso rates, per map, with misreads dropped
```

Every reader locates its UI element by appearance (colour, shape, nearby text) rather than fixed coordinates, and places the rest in units of the element's own size, so resolution should not matter. Readers prefer giving no value to giving a wrong one.

```
src/                     Svelte frontend
  lib/components/          cards shared by the overlay and the workbench
  lib/workbench/           image viewer and results panel
  routes/overlay/          overlay window
  routes/workbench/        workbench window
src-tauri/src/
  app/                     Tauri wiring: windows, tray, hotkeys, commands
  vision/                  image handling, colour blocks, OCR (Windows.Media.Ocr)
  readers/                 one module per game UI element: locate, read, parse
  pipeline.rs              frame → readings, shared by live capture and offline mode
  session.rs               readings over time → EXP and meso rates
  replay.rs                a folder of screenshots fed through both, in order
```

## Known limitations

- Tuned on 2560×1440 screenshots at the default UI size. Other resolutions and UI sizes are untested.
- **Windows HDR washes out screen captures** (the game itself looks fine). Readers expect HDR off; live capture will need to detect it.
- The Quest Helper is translucent with black text: it reads well over dark maps and poorly over bright, busy ones such as Henesys.
- Meso amounts are skipped when OCR readings disagree (the pixel font's 6 and 8 look alike). When a storage window shows a second box, the amount most boxes agree on wins.
- Tooltip `REQ` values sometimes go unread when OCR saw no clean digit to learn from, and the `(+N)` on wrapped item names can be garbled.
- Map names are read as shown, not matched against a list of known maps (there is no data source yet), so an OCR slip shows up as a separate map.
- NPC shop windows show only one meso box, so selling and buying in a shop still count as meso gains and losses. Storage and trade windows are recognised.
- When two readings are far apart and on different maps, the tracker cannot tell where the EXP was earned and lists it under *Between maps*. Frequent screenshots or a recording fix this.
- A jump of several levels between two readings adds no EXP, because the levels in between were never measured.
- Session rates have been checked on 14 hand-taken screenshots and synthetic tests, not yet on a continuous recording of real play.
- The price log and routes are not built yet.
- macOS OCR (Apple Vision) and the Tesseract fallback are not built yet.
