# Leafline

An unofficial, read-only overlay for **MapleStory Classic World**: price checks, EXP tracking, routes and quest help, read from what is visible on screen.

> The project is not affiliated with or endorsed by Nexon.

## What it does, and what it never does

Leafline only looks at your screen. It captures what is visible, reads it with OCR and shows information in a separate always-on-top window.

It never reads or writes game memory or files, never touches network traffic, never injects into or draws inside the game client, and never sends keyboard or mouse input to the game. There is no automation of any kind. No Nexon assets are included.

**Current status: live reading is opt-in, and at your own risk.** Leafline reads saved screenshots by default. Nexon has not said whether read-only overlays are allowed, and one player reported a two-week ban while running a similar overlay (unconfirmed), so live reading stays off until you turn it on in Settings. If you do, use a character you can afford to lose, ideally on a separate Nexon account.

## Getting started (Windows)

Prerequisites: [Rust](https://rustup.rs), [Node.js](https://nodejs.org) 20+, Visual Studio C++ build tools, and WebView2 (built into Windows 11).

```bash
npm install
npm run tauri dev
```

Two windows open:

- **Overlay**: the compact, transparent, always-on-top panel. Drag it by its title bar.
- **Workbench**: offline mode. Open a screenshot or a folder of them; the pipeline runs on each, outlines what it found and shows the readings. The overlay updates too, as it would during play. With a folder open, **Replay as session** reads every frame in order as one play session and shows EXP/hour, mesos/hour and time to level, overall and per map (see below).

You can also pass an image on the command line (`leafline.exe shot.png`), or use *Open with*.

| Hotkey | Action |
|---|---|
| `Ctrl+Alt+O` | Show or hide the overlay |
| `Ctrl+Alt+L` | Lock the overlay (clicks pass through to whatever is underneath) |
| `Ctrl+Alt+P` | Show or hide pinned panels |
| `Ctrl+Alt+H` | Compact HUD |
| `Ctrl+Alt+R` | Start or stop live reading |
| `Ctrl+Alt+I` | Read the item you're hovering, for a price check |
| `Ctrl+Alt+K` | Pause or resume tracking |
| `Ctrl+Alt+N` | Start a new session |
| `Ctrl+Alt+J` | Keep this run in the history or out of it |
| `Ctrl+Alt+F` | Save the picture live reading last read, to `Pictures\Leafline` |
| `Ctrl+Alt+W` | Open the workbench |

Change any hotkey, turn one off or turn them all off in Settings.

Closing a window hides it. Leafline keeps running in the tray until you choose **Quit**.

On Windows, the overlay and pinned panels snap to the edges of the screen and to each other as you drag them. Hold `Shift` to place one freely.

### Live reading

Turn it on under **Settings → Live reading**, then press `Ctrl+Alt+R` or the **Offline** badge on the overlay. Every few seconds Leafline takes a picture of the game window with Windows Graphics Capture, the API OBS and Discord use, and runs it through the same pipeline and session tracker as the workbench. The picture comes from the window itself, so the overlay and anything else in front of it are never in it, and a covered game is still read.

It only looks at the window. It asks Windows for the window's title, class, position and size, and never sends the game a message or opens its process. Pictures stay in memory unless you press **Save frame**. A picture that hasn't changed isn't read again, and pictures are always at least a second apart.

Item tooltips aren't read as you play. Hover an item and press `Ctrl+Alt+I`, and Leafline reads it from a single picture, whether live reading is running or not. The **Live reading** switch in Settings has to be on for this too.

The window is found by its title or class. If Leafline can't tell which window is the game, pick it in Settings; the choice is remembered. If Windows HDR is on for the game's display, the overlay warns you, because readings are unreliable until HDR is off and the game restarted.

### Teaching it your HUD

The HUD's numbers use a tiny pixel font that general OCR misreads, so Leafline reads them by matching character shapes it learns from your own screen. Open a screenshot in the workbench, type the values your HUD shows (level, name, HP, MP, EXP and EXP %) under **Teach the HUD**, and choose **Learn from this screenshot**. A second screenshot with different numbers fills in any digits the first one did not show. The shapes are saved in the app's data folder and never leave your PC.

The game lays its UI out on a 1024×768 grid and scales that to the window, so Leafline puts the HUD back on that grid before reading it. What you teach at one resolution or UI size works at the others.

### Session tracking

The session tracker turns a run of readings into rates: EXP gained, EXP/hour, % per hour, time to the next level, mesos gained and mesos/hour, overall, per map and over the last five minutes of active time, with a trend line. Live sessions save themselves to your history every 30 seconds once they have a minute of active time, with a line for each map, level, party size and class. Open a session under **History** to add a note, copy a line, or fix a misread map name, which stays fixed the next time it is read that way. Towns aren't logged. **Pause**, **New session** and **Log this run** sit on the Session card. Your records are copied to a `backups` folder once a day, and the last 7 copies are kept. Offline, the tracker runs on a folder of screenshots:

- **File time** orders frames by each file's modified time and takes that as the moment it shows. Suits screenshots saved by hand. Copying files can reset their times, so replay the folder they were saved to.
- **File name** orders frames by name, a fixed number of seconds apart. Suits frames extracted from a recording (for example `ffmpeg -i grind.mp4 -vf fps=1 frame_%05d.png`).

OCR is sometimes wrong, so the tracker checks readings against each other rather than trusting each one:

- EXP points and percent must agree. Together they imply how much EXP the level needs, which cannot change within a level, so a reading that implies a different amount is dropped as a misread. Tracking starts once two readings agree.
- Levels only go up, and a level-up counts once the percent wraps round or two readings agree on the new level.
- A meso amount counts once readings agree (two in a row for recordings; a single screenshot has to do for hand-taken ones). In live play a big jump needs three and a tenfold one eight, unless the mesos have been out of sight for two minutes. Changes while a shop, storage or trade window is open are left out of the rates.
- EXP lost to a death is left out of the rates, once two readings agree on the drop. The Session card says how many deaths and how much they cost.
- At high levels the HUD shows only a percentage. EXP/hour then stays blank, and % per hour and time to level come from the percentage alone.
- Another character is noticed by a lower level or a different name, three readings in a row. The old session is saved and a new one starts.
- If the minimap's name is hidden for 45 seconds, gains stop going to the last map read.
- The clock pauses after 15 seconds without EXP (change it in Settings), and at once on a map change without EXP, so time in town does not dilute the rates. Rates appear after 30 seconds of active time.
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
       ├─ party reader ────── counts HP bars in an evenly spaced column; never reads names
       ├─ shop reader ─────── an NPC shop, storage or trade window, by its column of buttons
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
  capture/                 live pictures of the game window (Windows Graphics Capture)
  vision/                  image handling, colour blocks, OCR (Windows.Media.Ocr)
  readers/                 one module per game UI element: locate, read, parse
  pipeline.rs              frame → readings, shared by live capture and offline mode
  session.rs               readings over time → EXP and meso rates
  replay.rs                a folder of screenshots fed through both, in order
```

## Known limitations

- Tuned on 2560×1440 screenshots. HUD reading is checked on real frames at both 2x and 1.875x; other resolutions are checked on simulated frames only, and Default Ratio (Filter Applied) not at all.
- **Windows HDR washes out screen captures** (the game itself looks fine). Readers expect HDR off; live reading warns when it is on.
- Live reading has been tested on saved screenshots shown in a window, not yet on the running game.
- Windows only. macOS capture and OCR are not built yet.
- The Quest Helper is translucent with black text: it reads well over dark maps and poorly over bright, busy ones such as Henesys.
- Meso amounts are skipped when OCR readings disagree (the pixel font's 6 and 8 look alike). When a storage window shows a second box, the amount most boxes agree on wins.
- Tooltip `REQ` values sometimes go unread when OCR saw no clean digit to learn from, and the `(+N)` on wrapped item names can be garbled.
- Without the map data, the session tracker keeps map names as read, so an OCR slip shows up as a separate map. Fix one under **History**.
- Shop, storage and trade windows are recognised by their buttons. This has not been tried on real frames of those windows yet.
- The party reader is checked on another project's reference frames, not on a party seen through Leafline.
- Digits your HUD hasn't shown yet aren't known until you teach them, so a new character needs teaching again as its numbers grow.
- When two readings are far apart and on different maps, the tracker cannot tell where the EXP was earned and lists it under *Between maps*. Frequent screenshots or a recording fix this.
- A jump of several levels between two readings adds no EXP, because the levels in between were never measured.
- Session rates have been checked on 14 hand-taken screenshots and synthetic tests, not yet on a continuous recording of real play.
- macOS OCR (Apple Vision) and the Tesseract fallback are not built yet.

## Data and credits

Leafline ships no game data. Map details are downloaded on your machine when you ask for them, kept in a local cache and credited in the app:

- Map, monster, NPC and quest data: [OSMS Data Explorer](https://osmsdataexplorer.com), CC BY 4.0.
- Player-observed drop rates, taxi routes and return scrolls: [mapleclassic.wiki](https://mapleclassic.wiki) contributors, CC BY-NC-SA 4.0.

Game content belongs to Nexon. Leafline is unofficial and not affiliated with or endorsed by Nexon.

## Licence

Leafline's code is under the [MIT licence](LICENSE). It does not cover game content or the downloaded data above, which keep their own terms.
