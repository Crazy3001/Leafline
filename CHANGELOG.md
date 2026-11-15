# Changelog

What changed in each release of Leafline. Leafline is a work in progress: if something reads
wrong for you, [report it](https://github.com/Crazy3001/Leafline/issues/new?template=bug_report.yml).

## 0.3.1 (2026-11-14)

### Fixed
- **The Route pin fits what it shows.** Opening **All N steps** or **Travel** under the next step
  now grows the pinned Route window to fit, and closing them shrinks it back, instead of the
  window staying short and scrolling. A width you set by hand is kept; the height always follows.

## 0.3.0 (2026-11-14)

### Added

**Training**
- **Where to train, worked out for your character.** Maps are now ranked by a forecast built from
  your own stats, gear, buffs and skill (how many blows each monster takes, how often you'd miss,
  how fast you attack), blended with how you actually play on maps you've trained on. Each map
  shows a likely range and plain reasons. A long trip and deaths count against a map, maps where
  most of your blows would miss wait under **Every level**, and an untried map that may beat your
  best is marked **Worth a try**.
- **How close are the estimates?** Under **Assumptions**, a check hides each map you've played in
  turn and estimates it from the rest, so you can see how far to trust the numbers.
- **Kills and quest EXP told apart.** The session counts kills by matching each EXP gain to the
  map's monsters. A quest reward shows on its own and stays out of EXP per hour, and the Train tab
  shows kills per hour.
- **Time to level with a range,** and a second figure with your usual breaks counted, plus the
  clock time you'll get there at this pace (in the Train tab and the compact HUD's tooltip).
- **A session survives a crash or a restart.** If the app closes or crashes and you are back within
  30 minutes, the session carries on where it was: the same log entry, the same rates, with the
  time away left out. **New session** starts fresh as before.
- **Each character keeps its own numbers.** The Train tab follows the name on your HUD, and with two
  or more characters it shows a **Character** picker, with **All characters** too. Where to train,
  your best maps, the last sessions, the training log and Records use only that character's play,
  so a Lv 120 hunter no longer shifts the estimates of a Lv 30 mage. Potion slots, prices, a level
  goal and tracked items are kept per character. Sessions that never read a name count for
  everyone while you have one character, and for nobody once you have two.

**Your character**
- **Character → Stats.** Every number your Character Stat window shows: STR, DEX, INT and LUK with
  what gear adds, Max HP and MP, Fame, AP left, and with **Detail** open, Attack, Weapon and Magic
  Def., Magic, Accuracy, Evasion, Crit. rate and damage, Speed and Jump. It says when it was last
  read, and **Read** reads it again. Character → Level up shows each stat's total with what you put
  in and what your gear adds, such as STR 30 (28+2).
- **Buffs.** Your job's buffs, each at the level your Skill window last showed, with a switch to
  say which are running. When the Character Stat window shows a buff in red, such as
  Accuracy 90 (+14), the switches follow it. With any on, the stats show what they come to with
  the number without buffs beside each, and the training estimates count them.
- **Gear that climbs every tier.** Each gear slot has one path: the best piece you can wear now,
  then one piece for every level tier up to the plan's end, each better than the last, shown at
  the level it asks for. Overalls carry on where a job's tops and bottoms end, and a helmet for any
  job no longer ends the job's own hats early. A goal you added wins its tier. The Gear tab has a
  Male / Female switch; with neither set, the gear you wear decides. The Build Planner's Gear tab,
  its Level by level list and Character → Gear's Next all follow it.
- **Skills.** Lookup has a Skills half: every skill by job, searchable by name or what it does.
  Skill names open a skill card with its picture, what it does, what it needs first and what each
  level gives.

**Maps and getting around**
- **See the map.** Map details (the Map tab, Lookup → Maps and a map's card) have a **Map** section
  with the map's picture, its portals (hidden ones marked), NPCs and spawn points drawn where they
  stand. Point at a mark to see what it is, click it for its card, and point at a monster in the
  list to light up where it spawns. Scroll to zoom where the pointer is, drag to move around, and
  double-click to zoom in or fit the whole map again. The portal your trip takes next shows in
  amber with a **Next** flag.
- **Portals, placed and linked.** Each portal card has a small locator showing where it stands on
  the map, and portals on the same side follow their real order. Pointing at a portal card lights
  it on the picture, and the other way round. The trip's next portal is flagged and never folds
  away behind "+N more". **List** shows every portal in a row, left to right across the map.
- **More from the route bar.** Under the next step, a strip of the map you are on with the portal
  to take lit up. **All N steps** opens every step of the trip, with the ones you have taken
  ticked off, how many portals and about how long on foot. **Travel** turns taxis and return
  scrolls on or off right there, and offers to download them when they are not on this PC yet.
- **Florina Beach in routes.** Routes now include the NPC flight from Lith Harbor to Florina Beach
  and back, without needing the taxi setting. Its fare isn't recorded yet.

**Items, quests and lookups**
- **Closest or Most.** A quest objective's list of maps has a **Closest / Most** switch: the
  nearest map with a fair number of the monster, or the one with the most of it, however far.
- **Buy it instead.** When an NPC shop sells an item a quest wants, the objective says so: the
  cheapest shop, its price and what the rest you need would cost, with Go.
- **Rest of the chain.** A quest's page says how many quests follow it in its chain, the highest
  level they ask for, and the EXP and mesos they pay in all.
- **Quests to take now.** The Quests tab lists quests your level can start that need no other quest
  first, nearest to your level first. Leafline can't see which quests you've done, so some may be
  done already.
- **Tracked items.** **Track it** on any item's page stars it. Starred items are listed in
  **Lookup → Items**, each with the monster that drops it, the best map to hunt it, a route, and a
  shop when one sells it.
- **A new Crafting tab.** Disciplines as tiles, a skill level row, recipes grouped by level, and
  ingredients as pictures with their counts.
- **Lookup filters.** All levels in Lookup → Items → Equip, a sort for every kind of item, "any" in
  empty level boxes, and NPCs sorted by name, most quests or map. Filter boxes stay open once
  opened.
- **Tidier drop lists.** By item, each drop is one line under Equip, Use and Etc headings, with the
  monsters that drop it as small pictures. By monster, each monster's items sit in one wrap.

**Cards and pictures**
- **Game pictures.** Items, monsters, NPCs and skills show their icons beside their names, and maps
  show their area's emblem. Each picture is downloaded from OSMS Data Explorer the first time it is
  shown and kept on this PC. Turn them off with **Show game pictures** in **Settings → Data**;
  names then stand alone, and nothing is downloaded.
- **Cards for coloured names.** Clicking an item, monster, NPC, map, quest or skill name opens a
  small card beside it instead of jumping to another tab, in a window of its own that you can drag
  anywhere on the screen. Names inside a card open the next card, with a back arrow, and **Pin**
  keeps a card open so the next link opens a new one beside it. Cards open folded, and remember
  which sections you open. **Full view** opens the tab as before.
- **Tooltips** on map marks, drops, small pictures, icon buttons and names cut short in a narrow
  window, all in one style.

**Around the app**
- **Download and install updates.** With the update check on, Settings → About offers **Download
  and install** when a new version is out. Leafline downloads the installer from the release,
  checks its SHA-256 against the release's checksum list, runs it only if they match, then closes
  so it can install. Nothing is downloaded until you press it.
- **A new look.** Rounded corners, softer surfaces, pill tabs, and the desk view's panels as cards
  of their own. Routes read as a line of stops. Coloured text opens a card; a row or box moves you
  within the tab you're in, and shows a chevron.

### Changed
- **Routes take the quickest walk.** Routes are worked out by how far you walk across each map,
  with climbing counted extra, instead of the fewest portals. A taxi or scroll is taken when it
  beats the walk. Walking speed and ride times are rough guesses for a character with no speed
  bonus.
- **Hidden portals** in routes say "stand on the spot and press ↑", since the game draws nothing
  there. Portals named like `h001` and `hide01` are now marked hidden too.
- **EXP per hour from the second kill** instead of after half a minute, marked "still measuring"
  until about a minute of play. A break now counts only a usual kill's time, and the last five
  minutes are exactly five minutes.
- **Quieter views.** Drop rates, "seen" dates, credit lines and outside links are gone from the tabs
  and cards; Settings → Data keeps where the data comes from.

### Upgrading
- Your records are upgraded the first time 0.3 starts, and the map data downloads again once.
  Leafline keeps a daily backup of your records, so take one aside first if you want to be able to
  go back to 0.2.

## 0.2.0 (2026-11-12)

### Added
- **Ready-made build plans** for every 2nd job, Lv 1 to 70, each following one written guide
  level by level: the skill points, the AP, and the gear to aim for. The plan credits its guide
  and links to the exact page, and the link is saved with the build. Where a guide offers a
  choice (Arrow Blow or Double Shot, axe or sword, spear or polearm, Magic Claw or Energy Bolt),
  each choice is its own plan beside the first. They replace the plans Leafline had written
  itself.
- **Desk view tabs:** the desk view now has tabs: Training, Questing, Character, Training log,
  Build planner and two of your own. Rename any tab, change its columns and panels, or reset it;
  your tabs, the window's size and whether it is maximized are all remembered. The training log
  and build planner open in the desk view instead of windows of their own.
- **Training log → Records:** every map you trained on, with EXP and mesos per hour for each
  level, party size and job you played it at. Filter by level, party and job, and order by EXP,
  mesos or time.
- **Mesos per hour after potions:** name the potions you hunt with and set what each costs you,
  then type how many you had at the start of a session and how many you have now. The Train tab,
  the session's page in the training log and Records show mesos per hour with that cost taken
  out.
- **Level goal:** set a goal level on the Train tab and see how many levels are left, about how
  much play that is, and how many days at your usual pace. Leafline learns the EXP each level
  takes from your own HUD as you level.
- **Your best maps:** the Train tab lists your best single run on each map before a session,
  **This map** compares your mesos with your best there as well as your EXP, and Records has a
  Best run column.
- **Crafting:** an item's page shows the recipe that makes it and the recipes that use it, with
  the discipline, skill level, ingredients and meso cost. **Lookup → Crafting** lists every
  recipe, by discipline, searchable by the item made or any ingredient. The recipes come with the
  map data, which downloads again once after this update.
- **NPC shops:** an item's page lists the shops that sell it, cheapest first, with the NPC, the
  map, the price and any town standing the shop asks for. Map pages list their shops and NPC
  pages what they sell. Covers Maple Island and Victoria Island at launch prices, each with the
  day it was recorded; Leafline checks for an updated list once a day.

### Changed
- **Where to train → For you:** maps you haven't tried are now estimated from how fast you get
  through monster HP in your own solo play, near your level, and held to how many monsters the
  map can bring back in an hour. Each estimate shows how sure it is and why. The respawn time
  behind that limit is an assumption you can change under **Assumptions**.
- **Quest turn-ins** now come from the game's own quest data instead of mapleclassic.wiki, which
  stopped answering programs on 2026-11-11. Leafline names who takes a quest in on 333 of 343
  quests, up from 287.
- **Monster drops** now come from Leafline's own drop list, built from the game's item text,
  StrategyWiki and players' reports, instead of the wiki's table. It covers every common monster's
  etc drops, shared drops like Leather and ores included, but has no rates: it says who drops
  what, not how often. Drops are checked for updates once a day, without needing a download first.

- **Item check starts from the item itself:** the name read from the tooltip is matched to the item
  data, and the item's own stats, upgrade slots, attack speed and knockback chance are the
  baseline. What the tooltip says replaces a stat only when it can be believed (its base matches
  the item's, or it sits within a normal roll). Anything doubtful keeps the item's own value and
  says so. When the name is only a close guess the card says it may be wrong and shows the item's
  own stats; when nothing fits it shows no stats and a button to search. Weapon knockback comes
  with the map data, which downloads again once.

### Fixed
- Live reading kept saying "The game is minimised" after the game was brought back, until Live was
  turned off and on again. It now opens a fresh capture when the game window returns.
- When a download failed, every lookup tried again at once. Leafline now waits a day, unless you
  press a download button.

## 0.1.1 (2026-11-10)

### Added
- **Settings → Feedback:** open the bug form or the idea form on GitHub, copy the bug report
  details, and open the folder of saved pictures, all in one place.
- **Report it:** when Leafline can't read something, or live reading stops, the line saying so has
  a link to the bug form.
- Each release lists a SHA-256 for its installers, in the notes and as `SHA256SUMS.txt`, so a
  download can be checked.

### Fixed
- An item quest line past its target, such as **9 / 5 Leather**, showed as 0/5. Item lines now
  show what you hold; kill lines still stop at their target.
- A quest line Leafline hasn't read yet showed as 0 in the compact HUD and on monster pages. It
  now shows **–** until it's read, and no longer counts toward your kill pace when it is.
- **Release notes and download** in Settings → About did nothing when clicked.
- Settings → About and the Live button no longer mention reading saved screenshots, which
  Leafline no longer does.

## 0.1.0 (2026-11-07)

The first public release, for testers.

- **Training:** EXP and meso rates per hour, overall, per map and over the last five minutes;
  time to level; sessions that save themselves; a training log; maps ranked for your level; a
  share card.
- **Items:** item check by hotkey (stats, requirements, scrolls, whether your build can wear it),
  where to get each item, a drop log, and your own notes.
- **Getting around:** the map you're on, Go here directions one step at a time, and a lookup of
  every item, monster, map and NPC.
- **Quests:** the Quest Helper read live, with steps, directions and who takes each quest in;
  quest search.
- **Your character:** the Character tab, a build planner that follows your Stat and Skill
  windows, gear suggestions, and a job advancement checklist.
- **Around the game:** the overlay, a compact HUD, pinned panels, a desk view for a second
  screen, hotkeys that don't send the game input, and countdown timers.

Known limitations are listed in the [README](README.md#known-limitations).
