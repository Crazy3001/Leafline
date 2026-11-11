# Changelog

What changed in each release of Leafline. Leafline is a work in progress: if something reads
wrong for you, [report it](https://github.com/Crazy3001/Leafline/issues/new?template=bug_report.yml).

## Unreleased

### Added
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
