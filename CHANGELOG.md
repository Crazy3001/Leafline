# Changelog

What changed in each release of Leafline. Leafline is a work in progress: if something reads
wrong for you, [report it](https://github.com/Crazy3001/Leafline/issues/new?template=bug_report.yml).

## Unreleased

### Added
- **Game pictures.** Items, monsters and NPCs show their icons beside their names, in lists and on
  their cards, and maps show their area's emblem and a picture of the whole map. Each picture is
  downloaded from OSMS Data Explorer the first time it is shown and kept on this PC. Turn them off
  with **Show game pictures** in **Settings → Data**; names then stand alone, and nothing is
  downloaded.
- **Cards for coloured names.** Clicking an item, monster, NPC, map or quest name now opens a
  small card beside it instead of jumping to another tab: an item's stats, needs, price, where to
  get it and your notes; a monster's numbers, where it ranks for HP per EXP, its best maps and
  drops; a map's monsters, exits and Go here; an NPC's maps, shop and quests; a quest's steps and
  your counts. Names inside a card open the next card, with a back arrow. Esc or a click outside
  closes it, and **Full view** opens the tab as before.
- **A new look.** Rounded corners, softer surfaces, pill tabs, and the desk view's panels as
  cards of their own. Routes read as a line of stops, the last one where you end up.
- **One rule for links.** Coloured text opens a card; a row or box moves you within the tab
  you're in, and shows a chevron. A card's **Full view** is the way into a tab. Places you headed
  for lately are Go buttons under the search.
- **Tooltips.** Resting the mouse on a mark on the map picture says what it is: a portal's
  destination (and whether it is hidden or a teleport within the map), an NPC with its picture,
  or a monster with its level and how many spawn points it has there. A drop in **Drops → By
  item** shows the item's picture, type, level and which monsters drop it. Small pictures,
  icon buttons, short numbers and names cut short in a narrow window explain themselves the
  same way, all in one style.
- **Skill pictures.** Skills show their icons in Character → Skills and the Build Planner.
- **Gear that climbs every tier.** Each gear slot now has one path: the best piece you can wear
  now, then one piece for every level tier the data has for your job up to the plan's end, each
  better than the last. A goal you added wins its tier; goals you've outgrown show as such.
  The Build Planner's Gear tab, its Level by level list and Character → Gear's Next all follow it.
- **Gear by level.** Gear paths no longer wait for your stats: each piece shows at the level it
  asks for. The Gear tab has a Male / Female switch; with neither set, the gear you wear decides.
  The Level by level list shows new gear as a row of pictures and skills with their icons.
- **Skills.** Lookup has a Skills half: every skill by job, searchable by name or what it does.
  Skill names in Character → Skills and the Build Planner open a skill card with its picture,
  what it does, what it needs first and what each level gives.
- **Cards you can move and pin.** Drag a card by its header. The pin button keeps it where it is,
  so you can open more beside it and compare; the card you click comes to the top. In the app a
  pinned card gets a small window of its own, to place anywhere on the screen, and dragging an
  open card past the overlay’s edge does the same. Cards always open with their sections folded. Skill names link everywhere, the Level by
  level list included.
- **Gear past Lv 40 and 50.** Where a job's tops and bottoms end, overalls carry on (a Bowman's
  Kismet, Tai and Linnex). A helmet for any job no longer ends the job's own hats early.
- **More in the map picture's tooltips:** a monster's picture, level, HP and EXP; an NPC's
  quests; a portal's area emblem. Character → Level up shows each stat's total with what you
  put in and what your gear adds, such as STR 30 (28+2).
- **A new Crafting tab.** Disciplines as tiles with a picture of what they make, a skill level
  row, recipes grouped by level, and ingredients as pictures with their counts.
- **Cards open folded**, as a short list of headings; what you open is remembered for cards.
- **Sort NPCs** by name, most quests or map. During a session, Where to train sits under your
  live numbers.
- **Quieter views.** Drop rates, "seen" dates, credit lines and outside links are gone from the
  tabs and cards; Settings → Data keeps where the data comes from.
- **All levels** in Lookup → Items → Equip, a sort for every kind of item, and "any" in empty
  level boxes. Filter boxes stay open once opened, through Clear; Monsters starts with them open.
- **See the map.** Map details (the Map tab, Lookup → Maps and a map's card) have a **Map**
  section with the map's picture, its portals (hidden ones marked), NPCs and spawn points drawn
  where they stand. Click a mark for its card; point at a monster in the list to light up where
  it spawns. It starts folded over the game and open in the desk view, and is downloaded the
  first time you open it.
- **Tidier drop lists.** By item, each drop is one line under Equip, Use and Etc headings, with
  the monsters that drop it as small pictures. By monster, each monster's items sit in one wrap.
- **More hidden portals marked in routes.** Portals named like `h001` and `hide01`, which the game
  uses for hidden ones, are now marked hidden too, not only `h_east` style names.
- **Florina Beach in routes.** The map data has no way into Florina Beach, because it is reached
  by an NPC's flight from Lith Harbor. Routes now include that flight and the one back, without
  needing the taxi setting. The ride comes from a player's report and its fare isn't recorded
  yet; check it in game.
- **A session survives a crash or a restart.** Leafline keeps what the tracker has worked out
  beside the saved session. If the app closes or crashes and you are back within 30 minutes, the
  session carries on where it was: the same log entry, the same rates, with the time away left
  out. **New session** starts fresh as before.
- **Each character keeps its own numbers.** The Train tab follows the name on your HUD, and with
  two or more characters it shows a **Character** picker (with the game closed, or to look at
  another one). Where to train, your best maps and the last sessions use only that character's
  play, so a Lv 120 hunter no longer shifts the estimates of a Lv 30 mage. Potion slots and
  prices are kept per character too; a character with none of its own shows the ones you had set
  before, until you change them. Sessions that never read a name count for everyone while you
  have one character, and for nobody once you have two. Nothing is hidden: the picker also has
  **All characters**, and any other character you have played, for the Train tab and the training
  log (Records and best maps follow it). Potions and a level goal always belong to one character:
  the one you pick, or on screen, or played last. A past session's potions use its own
  character's slots.

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
