# Leafline

An unofficial, read-only overlay for **MapleStory Classic World**: EXP and meso tracking, item checks, routes, quest help and build planning, beside what is visible on screen. Windows only for now.

> The project is not affiliated with or endorsed by Nexon.

## What it does, and what it never does

Leafline only looks at your screen. It captures what is visible, reads it with OCR and shows information in a separate always-on-top window.

It never reads or writes game memory or files, never touches network traffic, never injects into or draws inside the game client, and never sends keyboard or mouse input to the game. There is no automation of any kind. No Nexon assets are included.

Nothing read from the game leaves your PC. Leafline goes online only to download the community data below when you ask for it, and, if you turn it on in setup or Settings, to ask GitHub for the latest version number at startup. That request carries Leafline's name and version and nothing else; when a newer release is out, Settings → About says so and links to it. Nothing is downloaded or installed for you.

**Use it at your own risk.** Leafline reads the game window from the moment it starts (you can turn that off in Settings). Nexon has not said whether read-only overlays are allowed. Its support replies quote the rule against unauthorised third-party software, and one player reported a two-week ban while running a similar screen-reading overlay (unconfirmed). Nexon's FAQ says a penalty in one of its games carries to the others on the same account. If that worries you, play on a character you can afford to lose, ideally on a separate Nexon account.

## Getting started (Windows)

Prerequisites: [Rust](https://rustup.rs), [Node.js](https://nodejs.org) 20+, Visual Studio C++ build tools, and WebView2 (built into Windows 11).

```bash
npm install
npm run tauri dev
```

The first launch opens **setup**: a line on what Leafline is, the hotkeys, game settings that help it read, and the map data download. Run it again from Settings → About. Then the **overlay** is there: the compact, transparent, always-on-top panel. Drag it by its title bar.

Development builds also open the **workbench**, a tool for working on Leafline that release builds don't have. Open a screenshot or a folder of them; the pipeline runs on each, outlines what it found and shows the readings. The overlay updates too, as it would during play. With a folder open, **Replay as session** reads every frame in order as one play session and shows EXP/hour, mesos/hour and time to level, overall and per map (see below). You can also pass it an image on the command line (`leafline.exe shot.png`), or use *Open with*.

No hotkey is set until you pick it in setup or Settings, and a hotkey takes its key from the game. Setup suggests:

| Suggested | Action |
|---|---|
| `Win+F8` | Read the item you're hovering |
| `Win+F9` | Show or hide the overlay |
| `Pause` | Lock the overlay (clicks pass through to whatever is underneath) |

The game can't bind Win, and adding it also keeps a key away from programs that grab plain F-keys. Setup then asks you to press each key, and ticks the ones that arrive. F12 can't be a hotkey: Windows keeps it for itself.

Setup also asks about the compact HUD, reading your Character Stat and Skill windows, and saving the picture last read. Settings has the rest: pinned panels, starting or stopping live reading, pausing tracking, a new session, keeping a run out of the history, and the workbench (development builds).

F-keys, Pause and numpad keys work on their own; any other key needs Ctrl, Alt or Win with it, so it keeps working in chat. The game can't bind Pause, Win or the numpad, so Win+F9 or a numpad key never clashes with it. Avoid Ctrl and Alt: the game sees them while you hold them, so a hotkey with Ctrl can make your character attack. If another program on your PC already uses a key, setup and Settings say so.

Closing a window hides it. Leafline keeps running in the tray until you choose **Quit**.

On Windows, the overlay and pinned panels snap to the edges of the screen and to each other as you drag them. Hold `Shift` to place one freely.

### Live reading

It starts with Leafline and waits for the game window if the game isn't open. Turn it off under **Settings → Live reading**, or stop it with the **Live** badge on the overlay. Every few seconds Leafline takes a picture of the game window with Windows Graphics Capture, the API OBS and Discord use, and runs it through the same pipeline and session tracker as the workbench. The picture comes from the window itself, so the overlay and anything else in front of it are never in it, and a covered game is still read.

It only looks at the window. It asks Windows for the window's title, class, position and size, and never sends the game a message or opens its process. Pictures stay in memory unless you press **Save frame**, which saves one to a local folder that doesn't sync (Settings → Data shows it and lets you choose another). A picture that hasn't changed isn't read again, and pictures are always at least a second apart.

Item tooltips aren't read as you play. Hover an item and press the Read item hotkey, and Leafline reads it from a single picture, whether live reading is running or not. The **Live reading** switch in Settings has to be on for this too.

### Item check

The Read item hotkey opens an **item check** in the Lookup tab: the hovered item's stats with their base and bonus, its requirements and jobs, whether it's clean or scrolled, whether your build can wear it, who drops it and which quests give it, and your own note and prices for it.

Successful scrolls show in game as the `(+N)` after the name. Failed ones only show as fewer slots left than the item started with, so Leafline counts them against the slot count in the map data, and says that's where the number comes from, since Classic could differ. A copy with no `(+N)` reads as "No upgrades" until its slots can be checked.

Leafline has no market prices. Write a note on any item (keep it, sell it, who wants one), and log prices you saw or paid, each marked traded or asking: the card shows their median, range and the ones for copies like this one. Links open price sites in your browser with the item's name copied, ready to paste. The note and prices stay on your PC and come back whenever you check or look up the item.

The window is found by its title or class. If Leafline can't tell which window is the game, pick it in Settings; the choice is remembered. If Windows HDR is on for the game's display, the overlay warns you, because readings are unreliable until HDR is off and the game restarted.

### Reading the HUD

The HUD's numbers use a tiny pixel font that general OCR misreads, so Leafline tells each character apart by rules about its shape: how many closed holes it has, whether its top or bottom row is solid, and which sides have ink partway down. There is nothing to set up, and no font data ships with the app. A shape that fits no rule is left unread rather than guessed.

The game lays its UI out on a 1024×768 grid and scales that to the window, so Leafline puts the HUD back on that grid before reading it.

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

### Build planner

The **Character** tab is your character at a glance: level, job, HP, MP, EXP and mesos, this session's rates, and under them what your build plan says at your level: this level's AP and SP, the next job advancement and what comes after. **Edit** opens the planner. The session's totals, your history and your timers follow below.

A build is a job and a list of goals in order, the way written guides put them: this skill to 20, then that one; DEX to 25, the rest into LUK. Each new point goes to the first goal that can take it, and the planner lays the result out level by level. It says when a prerequisite comes too late, a goal can't finish in time, or 1st-job SP would be lost at the 2nd job advancement. Add gear to see the first level the plan can wear it, and a hovered item (the Read item hotkey) shows whether it suits the build.

Starting late, or spent points your own way? While live reading runs, open your Character Stat or Skill window: when what it shows changes, Leafline moves the build's starting point to match and lays out the rest of the plan from there. These windows are read from the same pictures live reading already takes, and only while one is open. With live reading stopped, use **Read stats** in the Character tab or its hotkey to read them from a single picture. The skill window shows one job tab at a time, so open each tab you've spent points on.

The Character tab also lists gear to look for at your level, ranked by the stats your plan raises. It doesn't know prices or how easy an item is to find, and nobody has checked its picks in game, so treat it as a list to look through.

### Quest search

The Quests tab and its pinned window show each quest in the Quest Helper with its counters and payout. Open one to see its steps, with directions to each monster and to whoever takes it in, plus its rewards and the quests before and after it. Search covers all the quests in the map data, by name, person, monster or region.

Once a tracked quest matches the data it stays locked in, so scenery moving behind the Quest Helper can't change it.

The game's data only says who gives a quest, so who takes it in comes from mapleclassic.wiki. Where the wiki doesn't say, Leafline goes by the next quest in a chain or a title like `to Heena`, and otherwise says it doesn't know rather than guess.

### Lookup

The Lookup tab searches Items, Monsters, Maps and NPCs. None of it needs live reading.

**Items** searches every item in the map data by name or by what it does, with categories and filters for slot, job, level and stat. An item's card shows its stats, whether your build can wear it, your note and logged prices, who players saw drop it and which quests give it. Names in a build, its suggestions and a map's drops open here.

Under **Equip**, the list starts with gear for you, slot by slot, the best first. It goes by your level, your job and your build's stats, using the same ranking as the Character tab's suggestions. Your level and job come from the game or your build, and you can change either, pick a gender, or look 5 to 20 levels ahead. Without a build it can't check stat requirements, and says so. Staff and novelty items are left out unless you search for them by name. **Everything** lists all equipment. Nobody has checked these picks in game, and they don't take prices into account.

**Monsters** opens with what's on your map and near your level, above every monster players meet. Search by name, by something it drops, or by a map, and narrow it by boss, undead, element weakness, region or level. A monster's card shows its stats, elements and what it can do, each map it lives on with a route from where you are, what players saw it drop, the quests that want it and any other versions of it. Monster names on the Map, Train and Quests tabs open here.

**Maps** starts with the map you're on, above every map. Search by name, area, a monster that lives there or an NPC who stands there, and narrow it by towns or hunting maps, region or monster level. A map opens the same card as the Map tab, with a route from where you are.

**NPCs** starts with who's on your map, above everyone. Search by name, a quest they give or a map they stand on. An NPC's card shows where they stand with a route there, the quests they give, and the ones handed in to them where the data shows it. NPC names on a map's card and in quest steps open here.

The numbers are the data's base values. Leafline doesn't work out damage, hits to kill or hit chance, because those are unsettled for Classic. Few monsters have drops logged on mapleclassic.wiki so far, and a card says so when there are none. A boss's listed respawn is a raw value from the client data, and the game's servers decide the real timing.

**Start from a plan** offers one ready-made Lv 1-70 plan for each 2nd job, and using one fills in goals you can then change. Leafline's plans were written for it from what each skill does and what each weapon needs. They aren't copied from any guide, and nobody has played them yet, so treat them as a starting point. A blank build is always there for your own.

**Guides** links to written guides for your job on MapleScanner and NiaDB. Those stay on their sites. If you follow another guide, keep its link on the build. **Copy plan** turns a build's goals into a line of text you can pass on, and **Paste a code** adds one. It holds goals only, not your character, stats or link.

The planner counts points. It doesn't model damage, accuracy or HP, because those are unsettled for Classic. The numbers it counts by come from community guides and in-game checks, not from Nexon, so each can be changed under **How points are counted**. Your stats have to be typed in, because they differ by character. Everything is a plan you follow by hand: nothing is spent for you, and Leafline can't yet tell whether you followed it.

## Testing the readers

Put screenshots in `test-screenshots/` (git-ignored: game screenshots must never be committed or shipped), then:

```bash
cd src-tauri
cargo test                                                      # unit tests
cargo run --release --example check                             # score readers against expected.json
cargo run --release --example analyze -- ../test-screenshots/   # readings as JSON
cargo run --release --example replay -- ../test-screenshots/    # replay a folder as a session
```

`check` needs a `test-screenshots/expected.json` that maps each file to what the screenshot really shows (see `examples/check.rs`). It prints every mismatch and a score. For `analyze`, add `--overview` to include every line from the whole-frame OCR pass.

`replay` prints what each frame read and the session summary as JSON; add `--every <seconds>` to order by name instead of file time.

`npm run dev` serves the UI in a plain browser with a mock backend that returns a saved analysis and session, which is handy for design work. The mock is excluded from production builds.

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
- Windows only. macOS capture and OCR are not built yet.
- The Quest Helper is translucent with black text: it reads well over dark maps and poorly over bright, busy ones such as Henesys.
- Meso amounts are skipped when OCR readings disagree (the pixel font's 6 and 8 look alike). When a storage window shows a second box, the amount most boxes agree on wins.
- Tooltip `REQ` values sometimes go unread when OCR saw no clean digit to learn from, and the `(+N)` on wrapped item names can be garbled.
- Without the map data, the session tracker keeps map names as read, so an OCR slip shows up as a separate map. Fix one under **History**.
- Shop, storage and trade windows are recognised by their buttons. This has not been tried on real frames of those windows yet.
- The party reader is checked on another project's reference frames, not on a party seen through Leafline.
- The level's bolder digits are only proven for 0 to 3. Levels showing 4 to 9 rely on those digits having the same shapes as the smaller font.
- When two readings are far apart and on different maps, the tracker cannot tell where the EXP was earned and lists it under *Between maps*. Frequent screenshots or a recording fix this.
- A jump of several levels between two readings adds no EXP, because the levels in between were never measured.
- Session rates have been checked on 14 hand-taken screenshots and synthetic tests, not yet on a continuous recording of real play.
- macOS OCR (Apple Vision) and the Tesseract fallback are not built yet.
- The build planner's item checks count your base stats and earlier gear goals, not what you really have on.
- An SP reset can't be planned yet, so a route that resets SP at a job advancement needs a second build from that point.

## Data and credits

Leafline ships no game data. Map details are downloaded on your machine when you ask for them, kept in a local cache and credited in the app:

- Map, monster, NPC, quest, skill and equipment data: [OSMS Data Explorer](https://osmsdataexplorer.com), CC BY 4.0.
- Player-observed drop rates, quest turn-ins, taxi routes and return scrolls: [mapleclassic.wiki](https://mapleclassic.wiki) contributors, CC BY-NC-SA 4.0.

Game content belongs to Nexon. Leafline is unofficial and not affiliated with or endorsed by Nexon.

## Licence

Leafline's code is under the [MIT licence](LICENSE). It does not cover game content or the downloaded data above, which keep their own terms.
