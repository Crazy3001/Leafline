# Changelog

What changed in each release of Leafline. Leafline is a work in progress: if something reads
wrong for you, [report it](https://github.com/Crazy3001/Leafline/issues/new?template=bug_report.yml).

## Unreleased

- **A compass at the top of every desk page** says where you're heading and the one thing to do
  next from the map you're on: an arrow and the portal to take, how many maps are left and any
  rides. It heads for the place set with Go here; with none set it offers the first Next up step
  that has somewhere to go, which Go sets. Click it for the whole route; × stops. On a phone it's
  one line under the header. The arrow is from the portal you come out of, by the map data; on
  the map you start from only the portal's side of the map is known. When the minimap's name is
  out of view for a few seconds it says the way is from the map you were last seen on.
- On the website, Go here now follows you on the desk too (it only did in the phone view).
- Now's This map no longer stops updating on a map with two ways out to the same map.
- **Search as you type.** The search box at the top of every page drops down matches as you type:
  items, monsters, maps, NPCs and quests, each with its picture and a line (an item's type, a
  monster's level, a quest's level and region), with chips and counts to show one kind. Arrow
  keys move, Enter opens it in Look up (a quest on Quests), Esc closes, and / still jumps there.
- **Share the game window sits by the search box** on the website, and says "Reading live" with
  Stop while it reads. The Read the game card is gone from Now; what it said is in the button's
  tooltip, and the sidebar's foot says it all: unofficial, Nexon hasn't approved overlays, use at
  your own risk, with a link to the details.
- **Now leads with the map you're on**, with its monsters, drops and ways out, and the session's
  chart beside it; Next up and Quests come under them. Not reading, it shows the map you were last
  on, marked Last seen. Typing a session in moved to Train.
- **Quests is one list.** Every quest, region by region with "x of y done", and what's ready to
  hand in first: who takes it, where, and Go. Quests in progress show their counts from the Quest
  Helper or the Quest window. The filter chips stay; On now, Quest log and To take are gone, and a
  pinned Quests window still shows what you're on.
- **Questlines fold into one row** that opens: the line's name, how far along you are (1 of 4)
  and the quest you're on, marked. On by default; the Questlines switch shows every quest flat,
  and is remembered. A quest the game lists by its series' name (the Quest window's "Collecting
  For Maya" for "Maya's First Collection") lands on its step instead of "Not in the map data".
- **A quest opens beside the list** on a wide window, with its questline on top (done, on now,
  next) and the quest under it: its counts, where to hunt, who gives and takes it, and what it
  pays. On a phone it opens full screen with Back.
- **Each region's count goes the way the game's Completed tab does** with questlines on: a
  questline counts once, when every step is done; flat, every quest counts. Only what the Quest
  window showed done counts. Quests Leafline takes to be done because a later one is show
  apart, as "likely" and "Probably done". Where the game's own count differs, it shows beside
  Leafline's. The totals are what the quest data lists, which may include quests Classic
  doesn't have.
- Item counts past their target (28 / 20 held) are kept, when the target reads as the data has it.
- **The player bar is on Now too**, and its Session buttons fit: Pause and New sit under the
  clock and wrap instead of running past the window's edge, at any text size. "Autosaves" is an
  info mark now. The map you're on has its own line under your level and job, whole, with its
  region. Now drops the Welcome banner and the Your character card, which said the same things,
  so Next up and Quests come first.
- **Pick a character from any page.** The desk sidebar has the character switch at its top,
  under the logo: who Leafline shows, their level and job, and "as of …" when it was read
  earlier. It opens a list of every character this copy has seen, each with a last-seen line
  (level, job, how long ago, last map, or "Playing now"), with All characters once there are two.
  Arrow keys, Enter and Esc work in the list. In the overlay and on a phone, the name in the header opens the same list. It shows
  from the first character read; with none yet, it says how to get one. Logging in on another
  character still switches by itself, and the switch says so for a few seconds.
- **A tidier desk sidebar.** Light or dark is a sun/moon button beside the fold button (on a
  phone, in the Tools menu); it keeps the look you picked in Settings and swaps its light or dark
  half. The "Not reading" card is gone: the player bar says whether Leafline is reading (on the
  website, the Share button does). Folded,
  the character is a round button with their first letter.
- On the website, the Share button turns amber and says "Can't read the picture" when the shared
  picture is black, frozen or washed out by Windows HDR; its tooltip says what to do.
- **Look up shows the list and the open card side by side** on a wide window, for items,
  monsters, maps, NPCs and skills; the picked row stays marked. A narrow window opens the card in
  the list's place, with a way back, as before.
- Equipment names the classes that can wear it under its name, in Look up's list and on its card.
- An item's **Where to get it** groups its sources: Dropped by, Sold by, From quests and Crafted
  from, each folded with its count when an item opens. Scrolling odds come after it.
- **New build:** the bar at the bottom holds the build's name, who it's for, the level it starts
  at and who its gear is made for, beside Create build.
- **Weapons have their Attack again.** The map data names weapon attack differently now. Leafline
  reads the new name, but map data downloaded earlier kept no Attack, so weapons showed none and
  the build planner's weapon path passed over most planned weapons. The map data downloads once
  more to pick it up.
- Tooltips and hover cards sit beside the mouse, follow it, and stay inside the window, at any
  text size on the website (at 125% they used to land well away from the mouse; link cards too). An
  equipment card names the classes that can use it under its level, and shows its NPC sell price.
- **Build planner:** the Create build button stays in view at the bottom of New build, and says
  what it will make. New build asks who the gear is made for. A build's header says which
  character it follows, with a button to use it for the character Leafline reads, and has
  duplicate and delete buttons. Goals the gear path passes over fold into one "N passed over"
  button per slot. Leafline's own ready-made plans no longer link a guide.
- **Hover cards on every link.** Rest the mouse on a coloured name or a linked picture (an item,
  monster, NPC, map, quest or skill) and a small card shows what it is: an item's level, stats,
  requirements and where it comes from; a monster's level, HP, EXP, elements, maps and drops; an
  NPC's map, shop and quests; a map's monsters and NPCs; a quest's giver and rewards; a skill's
  job, max level and what it does. A click still opens the full card. The ready-made plan cards'
  key skills and weapons are links now too.
- **Shorter ready-made build plans.** Each plan card, in New build and in the planner's Settings,
  now shows one line on what the plan does, the Lv 1 stats, its key skills and weapons, and for
  Leafline's own plans a few short pros and cons. A guide plan names its guide in one linked line,
  with the full credit on the link.
- **Every quest in one list.** Quests → Quest log now lists every quest by region, marked done,
  in progress, ready to hand in, can take, or not yet with why (the level it needs, or the quest
  to finish first). Quests before one you're on or have done count as done, and a quest the game
  lists under its series' name shows as the step you're on. "Can take" comes from the quest data,
  which doesn't know every condition; looking through your Available tab corrects it.
- Quest window rows the game lists under a series' name, such as Collecting For Maya, are kept.
- The Quest window is read even when the picture wobbles a little, as a shared window's does.
- The website says why the game isn't being read: Firefox sharing a still picture of the game's
  window (share the screen instead), a black picture, or Windows HDR.

## 0.5.0 (2026-11-24)

### The website
Leafline now also runs in a browser at **[crazy3001.github.io/Leafline](https://crazy3001.github.io/Leafline/)**,
with nothing to install. It is the same Leafline, built from the same code: lookups, Go here,
where to train, the build planner, the calculators, the drop log, timers and the training log, in
the four looks, with the desk layout on a computer and the overlay's tabs on a phone.

- **Read the game in the browser.** On a computer, in Firefox, Chrome or Edge, share the game
  window from **Now → Read the game**. The website reads your level, EXP, mesos, map, name and
  job, the Quest Helper and the Pickup Log, and tracks the session the way the program does.
  Rest the mouse on an item for its card with its stats, and open your Stat, Skill, inventory or
  Quest window to have it read. Each has a switch in **Settings → Reading**. The picture never
  leaves your computer, and the website carries the same ban risk as the program.
- **A typed-in session** for when nothing is read: type your level and EXP at the start and the
  end, and get EXP per hour, time to level and a session in the training log.
- **Pop out** keeps Now's numbers in a small window over the game.
- **Records stay in your browser.** Save them with **Settings → Data → Records file** now and then, since
  a browser can clear a site's storage.

### Added
- **Item cards while you play.** Rest the mouse on an item in game and its card comes up in Look
  up, read from the tooltip, with no hotkey. The card changes only when the item does.
- **Windows read while open.** Hold your Character Stat or Skill window still and your build moves
  to what it shows; hold an inventory tab still for Sell or keep; scroll your Quest window and the
  Quests tab keeps what it lists. Each has a switch in **Settings → Live reading**, and the
  hotkeys work as before.
- **A records file.** **Settings → Data** saves all your records to one file, and opens one: the
  same file works in the program, the website or on another PC.
- **A typed-in session** in the program too, shown on Now while live reading is off.

### Changed
- **Mesos are read by the game's own digit shapes**, with OCR only as a fallback, so they read
  faster and slip less.
- **Routes into Forgotten Hollow.** Its two ways in are portals the game's server sends you
  through, so the data had no destination for them and Go here found no way in. Map data is
  downloaded again once to pick this up.
- **Map data after a patch.** When OSMS hasn't caught up with a game patch yet, Leafline uses
  Leafline-data's copy with the patched client's maps, quests and items laid over it.
- **Sell or keep** no longer starts a new check when an item's tooltip is drawn over the tab.

### Fixed
- **Critical Damage** on an item was read as Critical Rate. **Evasion** is now read too.

## 0.4.0 (2026-11-18)

### A new look
Every screen is redrawn. Everything Leafline did before is still there, reached from a tab, the
Tools list or search.

- **Four looks:** Ember Dark (the default), Ember Light, Sprout Light and Sprout Dark. Pick one in
  setup or in **Settings → Look**.
- **Five tabs:** **Now**, **Quests**, **Map**, **Train** and **Me**. Now shows the session at a
  glance with the quest you're on. The header carries your level, name, job, EXP and time to level.
- **Views by chips.** Quests has **On now**, **Quest log** and **To take**; Train has **Session**,
  **Where to train** and **History**. The Map tab puts the map's own facts first, then its parts as
  cards you can jump to.
- **One search box** for items, monsters, maps, NPCs, skills and tools. Picking a kind's list keeps
  typing in the same box.
- **A Tools list** for everything that isn't a tab: Crafting, Sell or keep, Timers, Drop log,
  Tracked items, Potion costs, Gear finder, Training log, Build planner, Share card, Route planner
  and Characters.
- **Go here** sits under the header as one solid card.
- **The compact HUD is one strip:** level, an EXP bar, EXP and mesos per hour with how the last five
  minutes compare, time to level and session time, with the next step and quest counters under it.
- **A Now pin:** pin the session and your quests into their own small window.
- **The desk view is rebuilt** for a wide window: a sidebar of views (Now, Quests, Map, Train, Me,
  Look up, the training log, the build planner and the tools) with one search and the Go here bar on
  top, each view laid out across the width. **My layout** keeps the old columns you arranged
  yourself. **Pop out** pins Now beside the game.
- **The build planner opens on a Plan tab:** a chart of your levels with job bands, when each goal
  is reached, where the AP goes, new gear and advancements, and where you are now. Pick a level for
  everything that happens at it. Goals are listed in order with done, now, next and short tags, and
  can be dragged into a new order.
- **The training log opens on an Overview:** six tiles, EXP per day, your best maps, recent
  sessions, the next level and the time spent at each level.
- **Setup** has a Look step, and its steps run down the side. **Settings** has a side menu that
  follows where you are.
- **Sessions under 10 minutes** of play aren't listed, ranked or used as your rate, so a few seconds
  on a map can't top Where to train.

### Added
- **Your fight.** Every monster's page shows how a fight with it goes for your character: how
  many attacks a kill takes and how often each count comes up, the chance your blows land, what a
  blow does after its defence and the level gap, and the seconds of attacking a kill takes. It
  uses your attack from the Character Stat window, or what you typed in Assumptions. The formulas
  are the community's, and the defence part learns from your own fights.
- **Scrolling odds.** A scroll's page has **Odds**, and a piece of gear **Scrolling odds**: the
  chance of reaching the passes you want, of the item breaking, the stat it ends with, the scrolls
  used, the chance of each number of passes, and with prices typed, what one item that gets there
  costs. Every number can be changed.
- **Plan levelling a craft.** Lookup → Crafting, with a discipline picked, has **Plan levelling**:
  type your skill level and the EXP your crafting window shows, and the recipes open to you are
  ranked by mesos per EXP or by fewest crafts, with how many crafts it takes, what they cost and
  what to farm.
- **Monsters on the map picture.** Spawn points show the monster's own picture instead of a dot,
  a little bigger as you zoom in.
- **What spawns where.** Lookup → Maps rows show each map's monsters with their share of the
  spawns, how many spawn and how thickly (spawns per 1,000 px of floor), and sort by either.

### Changed
- **The Train tab's EXP bar** looks like the game's, with your EXP and percent written on it.

**Training**
- **What deaths cost.** Each saved session keeps the EXP its deaths lost. The training log shows
  deaths in the session list, the EXP lost under the **Deaths** tile with about how much play that
  undid, and a **Deaths** column per map. The Train tab says the same while you play. Characters
  whose HUD shows only a percentage get it as a share of a level. The CSV export has two new
  columns, deaths and exp_lost.
- **Solo and party apart.** In a party, the EXP you get is your share, so one rate over both says
  little about either. With **Party** set to **Either**, the training log's Maps and Records views
  show **Solo EXP / hr** and **Party EXP / hr** side by side, and a map you pick shows both with
  the party sizes you played in. A best run says whether it was solo or in a party. The Train tab's
  **This map** compares you with your best there played the same way, and gives the other way's
  average.
- **Potions counted for you.** Pick the quick slot each potion sits on in **Potions → Edit potions**,
  and Leafline counts what you drink from the number the game shows there, while the quick-slot bar
  is in view. Buying more doesn't count as drunk. Potions not on a quick slot are still typed in.
- **Sell or keep.** Open your Item Inventory on the Equip, Use, Set-up or Etc tab and press the new
  **Sell or keep** hotkey (set it in Settings). Leafline names each item by its picture and marks
  what to keep: what a quest on your Quest Helper or one you follow asks for, what crafting uses,
  gear your build will wear, items you track, and scrolls. The rest is marked to sell, with what the
  bag fetches at NPC prices. Items the game draws alike, such as a tier of scrolls, say so. The
  first check downloads the pictures of the items most likely at your level; **Look further**
  tries every item of the tab. It needs **Show game pictures** on. NPC prices come from the item
  data and haven't been checked in game yet.
- **Loot counted for you.** While live reading is on, Leafline reads the lines at the bottom right
  of the game that say what you picked up (`Leaf x1 earned. (Etc)`, `You have gained mesos (+27)`),
  each line once, even when the same words come twice in a row or several arrive at once. The
  Train tab's new **Loot** section lists each item with how many, what one is worth (your own
  logged price if you have one, else the NPC price) and the total, plus the mesos picked up. A line
  Leafline can't read or name is left out and said so, so loot can come out short but never over.
  Each saved session keeps its loot, the training log shows it, and **Export loot CSV** saves it.
  Turn it off under **Settings → Live reading → Advanced → Count loot**.
- **A word about UI Size.** When the game draws its interface at UI Size Default Ratio, which
  smooths its text, Settings and the overlay say that some readings may be missed and that Ideal
  Ratio reads best.
- **Profit with loot.** The Potions line now reads mesos plus loot, less potions, per hour. When
  your inventory's mesos weren't seen twice, **Mesos / hr** uses the mesos picked up instead, and
  when they were, it shows the picked-up rate beside it as a check.
- **When another map gets better.** While you train, once you've gone up a level on a map and
  another map's forecast now beats it by 15% or more (the trip there counted), **This map** says so
  in one quiet line, with **Go** and **Not now**. Not now hides it until your next level. It never
  pops up or makes a sound.

### Changed
- **Your records are upgraded** the first time 0.4 opens them, so that sessions keep what their
  deaths cost, the potions read from your quick slots and the loot picked up. Sessions saved before keep their deaths
  and get the EXP lost from their maps.
- **Quest Helper counts read as drawn.** At UI Size Ideal Ratio, each line under a quest's title
  (`23 / 20 Orange Mushroom Cap`) is now read letter by letter against the game's own lettering,
  so counts and names come out exactly as shown. Lines over bright clouds are no longer lost.
  Quest titles, the other UI Sizes and the map name are read as before.

**Quests**
- **Your quest log, read from the Quest window.** While the game's Quest window is open, Leafline
  reads its **Available**, **In Progress** and **Completed** lists and the quest picked beside
  them, and keeps what it learns for each character on this PC. Scroll each tab to its end once so
  it sees every row; the region counts on the bars tell it when a tab has been seen whole. It is
  only true as of the last look, and everything says when that was. Reading never forgets a quest
  you've done.
- **Done, in progress and available, everywhere a quest is named.** Quest names in the Quests tab,
  on quest pages, on map and NPC cards carry a small mark, and quest pages say when the game showed
  it done or how far along it was.
- **Quests to take at your level** leaves out the ones you've done and the ones you've taken, counts
  a quest whose earlier quests you've finished as one you can start, and marks the ones the game
  listed as **Available**. The quest search has an **Only quests not done** filter.
- **Every quest in progress, not only the Quest Helper's five,** shown with its steps and directions
  and the counts the Quest window last showed.
- **Ready to hand in:** quests whose counts were full, or that the game marked done, each with who
  takes it in and a **Go** button.
- **Done by region:** how many quests you've done in each region against how many there are.
- **The rest of a chain shows where you are** along it, and map and NPC cards say how many of their
  quests you haven't done.
- **Where the game and the data differ.** When the Quest window shows a level, a count or a reward
  the quest data says otherwise, the quest page says so. It stays on this PC.

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
