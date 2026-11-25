//! The website's live reading: a shared game window's frames read by the program's own readers
//! and fed to the program's own session tracker, as `src-tauri`'s live loop does. Only what reads
//! without Windows OCR runs here: level and EXP from the HUD's digits, mesos from the game's digit
//! shapes, the character's name and job and the Quest Helper's objectives by the game's fonts, and
//! once the page hands over ocrs's model and the reference data, the map from the minimap title
//! (see `title`) and the Quest Helper's titles (see `words`). Frames come in as RGBA bytes from the
//! page's worker and never leave it.

use serde::Serialize;
use serde_json::Value;

use leafline_core::build::catalog::Catalog;
use leafline_core::loot::Pickup;
use leafline_core::mapdata::QuestTitles;
use leafline_core::mapdata::loot::name_item;
use leafline_core::mapdata::atlas::Atlas;
use leafline_core::mapdata::osms::{Dataset, Records};
use leafline_core::pipeline::{self, Analysis};
use leafline_core::readers::pickup::{self, PickupReader};
use leafline_core::readers::ui;
use leafline_core::session::{Sample, SessionSummary, SessionTracker, TrackerConfig};
use leafline_core::vision::{Frame, Ocr};

use std::sync::Arc;

use leafline_core::readers::tooltip::hover::{HOLD, same_cards as same_items, same_places};
use leafline_core::readers::tooltip::{self, ItemTooltip, NameLines};
use leafline_core::vision::Rect;

use leafline_core::pipeline::CharacterScan;
use leafline_core::readers::{character, inventory, questlog};
use leafline_core::readers::questlog::QuestLogReading;
use leafline_core::service::bag::SeenBag;

use crate::engine::OcrsEngine;
use crate::identity::IdentityReader;
use crate::items::{ReadName, TooltipReader};
use crate::ocr::Recogniser;
use crate::title::{self, Settled, TitleReader};
use crate::words::DarkText;

/// Live frames are plentiful, so a reading must agree twice before it counts, as in the program.
const CONFIRMATIONS: usize = 2;
/// Full reads in a row without the Quest window before it counts as closed, and the next opening
/// as a fresh look, as in the program.
const QUEST_WINDOW_CLOSED_AFTER: u32 = 3;


/// What one frame gave: the readings for the page to show, and the session so far.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveRead {
    pub analysis: Analysis,
    pub session: SessionSummary,
    /// The HUD was found in the frame.
    pub hud: bool,
    /// How long reading took, in milliseconds.
    pub ms: f32,
}

/// What the game's windows gave since this was last asked, for the page to keep: each is read once
/// its window has shown the same at two reads in a row (held still, not being dragged or
/// scrolled), and again only when what it shows changes.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowReads {
    /// The Character Stat and Skill windows, with the name and level from the HUD.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub character: Option<CharacterScan>,
    /// The Item Inventory's open tab, for sell or keep.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bag: Option<SeenBag>,
    /// The Quest window, for the character's quest log.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quest_log: Option<QuestLogSeen>,
}

impl WindowReads {
    pub fn is_empty(&self) -> bool {
        self.character.is_none() && self.bag.is_none() && self.quest_log.is_none()
    }
}

/// One read of the Quest window, and whose.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestLogSeen {
    pub name: String,
    pub reading: QuestLogReading,
    /// The window was shut since the last read: its tabs start afresh.
    pub fresh: bool,
}

/// The player's switches for the windows read when open.
#[derive(Debug, Clone, Copy)]
struct WindowChoice {
    character: bool,
    bag: bool,
    quest_log: bool,
}

/// What follows the windows from one read to the next.
#[derive(Default)]
struct Windows {
    character: character::Watch,
    /// The inventory tab last seen, and the one last handed over.
    bag: inventory::TabWatch,
    quest_log: questlog::Watch,
    quest_closed: u32,
    quest_fresh: bool,
    reads: WindowReads,
}

pub struct LiveReader {
    ocr: Ocr,
    /// ocrs as an OCR engine, for the readers that take words from a picture (skill names, the
    /// Quest window).
    text: Option<Ocr>,
    choice: WindowChoice,
    windows: Windows,
    /// ocrs, once the page has handed over its model; until then no text is read (the map, the
    /// character's name and job).
    recogniser: Option<Arc<Recogniser>>,
    /// Item tooltips' names, settled against the item data.
    tooltips: TooltipReader,
    /// The player's switch: read a tooltip the mouse rests on.
    item_tooltips: bool,
    /// The item tooltips last read, shown until others are; whether they changed since asked.
    items: Vec<ItemTooltip>,
    items_changed: bool,
    /// Where tooltips stood at the last looks, and since when, so a held hover is read once.
    hover: Option<Hover>,
    title: TitleReader,
    identity: IdentityReader,
    /// The Quest Helper's titles and header.
    words: DarkText,
    /// The reference data handed over so far (maps, quests, items, monsters), and the atlas made
    /// of it that minimap and quest titles are settled against.
    dataset: Dataset,
    atlas: Option<Atlas>,
    /// The Quest Helper's quests held steady from one reading to the next, as the program does.
    quests: QuestTitles,
    /// The Pickup Log, read on every full read and looked at between them; the items it names
    /// are matched against the item data once the page hands it over.
    pickups: PickupReader,
    catalog: Option<Catalog>,
    /// The UI size the HUD was last drawn at, which the log's font follows.
    ui_scale: Option<f32>,
    /// The player's switch: count the Pickup Log.
    pickup_log: bool,
    tracker: SessionTracker,
    idle_after: f64,
    held: bool,
}

fn config(idle_after: f64) -> TrackerConfig {
    TrackerConfig { confirmations: CONFIRMATIONS, idle_after, ..TrackerConfig::default() }
}

impl LiveReader {
    /// `idle_after`: seconds without EXP before the clock pauses (the player's setting).
    pub fn new(idle_after: f64) -> Self {
        Self::with(SessionTracker::new(config(idle_after)), idle_after)
    }

    /// Carries on with a session saved by [`LiveReader::snapshot`]; `None` if it can't be read.
    pub fn restore(idle_after: f64, saved: Value) -> Option<Self> {
        let tracker = SessionTracker::restore(config(idle_after), saved)?;
        Some(Self::with(tracker, idle_after))
    }

    fn with(tracker: SessionTracker, idle_after: f64) -> Self {
        Self {
            ocr: Ocr::none(),
            text: None,
            choice: WindowChoice { character: true, bag: true, quest_log: true },
            windows: Windows::default(),
            recogniser: None,
            tooltips: TooltipReader::default(),
            item_tooltips: true,
            items: Vec::new(),
            items_changed: false,
            hover: None,
            title: TitleReader::default(),
            identity: IdentityReader::default(),
            words: DarkText::default(),
            dataset: Dataset::default(),
            atlas: None,
            quests: QuestTitles::default(),
            pickups: PickupReader::default(),
            catalog: None,
            ui_scale: None,
            pickup_log: true,
            tracker,
            idle_after,
            held: false,
        }
    }

    /// ocrs's text recognition model, for the minimap title and the HUD's name and job.
    pub fn set_text_model(&mut self, recognition: Vec<u8>) -> Result<(), String> {
        let recogniser = Arc::new(Recogniser::new(recognition)?);
        self.text = Some(Ocr::with(Box::new(OcrsEngine::new(recogniser.clone()))));
        self.recogniser = Some(recogniser);
        Ok(())
    }

    /// One of the reference data files (OSMS `maps.json`, `quests.json`, `items.json` or
    /// `monsters.json`), to settle titles against. Maps are needed for the minimap; quests (with
    /// items and monsters) steady the Quest Helper's titles.
    pub fn set_data(&mut self, file: &str, json: &[u8]) -> Result<(), String> {
        if !["maps.json", "quests.json", "items.json", "monsters.json"].contains(&file) {
            return Err(format!("not a data file the reader uses: {file}"));
        }
        match Records::parse(file, json).map_err(|e| e.to_string())? {
            Records::Maps(maps) => self.dataset.maps = maps,
            Records::Quests(quests) => self.dataset.quests = quests,
            Records::Items(items) => {
                self.catalog = Some(Catalog::new(Vec::new(), items.clone()));
                self.dataset.items = items;
            }
            Records::Monsters(monsters) => self.dataset.monsters = monsters,
            _ => {}
        }
        self.atlas = Some(Atlas::new(self.dataset.clone()));
        Ok(())
    }

    /// The tracker's whole state, to carry on after the page is closed or reloaded.
    pub fn snapshot(&self) -> Value {
        serde_json::to_value(&self.tracker).unwrap_or(Value::Null)
    }

    pub fn set_idle_after(&mut self, seconds: f64) {
        self.idle_after = seconds;
        self.tracker.set_idle_after(seconds);
    }

    /// Stops or restarts tracking by hand; frames are still read.
    pub fn set_held(&mut self, held: bool) {
        self.held = held;
        self.tracker.set_held(held);
    }

    /// Starts a new session.
    pub fn reset(&mut self) {
        self.quests = QuestTitles::default();
        self.pickups.reset();
        self.tracker = SessionTracker::new(config(self.idle_after));
        self.tracker.set_held(self.held);
    }

    pub fn summary(&self) -> SessionSummary {
        self.tracker.summary()
    }

    /// Reads `frame`, taken at `at` Unix seconds. `other` is set when it shows another character
    /// than the session's, which then starts afresh (the caller saves the old one first, from the
    /// summary it had).
    /// What the readers make of `frame`, without the session: the program's readers that need no
    /// OCR, then the parts read by ocrs. `None` when the frame can't be read at all.
    pub fn analyze(&mut self, frame: &Frame) -> Option<(Analysis, Option<Settled>)> {
        let mut analysis = pipeline::analyze_live(frame, &self.ocr).ok()?;
        if let Some(layout) = &analysis.hud_layout {
            self.ui_scale = Some(ui::hud_scale(frame, layout));
        }
        let Some(recogniser) = self.recogniser.as_deref() else { return Some((analysis, None)) };
        if let (Some(hud), Some(layout)) = (analysis.hud.as_mut(), analysis.hud_layout.as_ref()) {
            let identity = self.identity.read(recogniser, frame, layout);
            hud.job = identity.job;
            hud.name = identity.name;
        }
        if analysis.quest.is_none() {
            let words = &mut self.words;
            analysis.quest = pipeline::read_quests_drawn(frame, analysis.hud_layout.as_ref(), &mut |picture, area| {
                words.read(recogniser, picture, area)
            });
        }
        let mut settled = None;
        if analysis.map.is_none()
            && let Some(atlas) = self.atlas.as_ref()
            && let Some(raw) = self.title.read(recogniser, frame)
            && let Some((area, _)) = leafline_core::readers::map::title_area(frame)
        {
            settled = title::settle(atlas, &raw, area);
        }
        if let Some(found) = &settled {
            analysis.map = Some(found.reading.clone());
        }
        Some((analysis, settled))
    }

    /// Reads `frame`, taken at `at` Unix seconds. `other` is set when it shows another character
    /// than the session's, which then starts afresh (the caller saves the old one first, from the
    /// summary it had).
    pub fn read(&mut self, frame: &Frame, at: f64) -> Option<(LiveRead, bool)> {
        let started = leafline_core::clock::Instant::now();
        let (mut analysis, settled) = self.analyze(frame)?;
        if let Some(atlas) = self.atlas.as_ref().filter(|_| !self.dataset.quests.is_empty()) {
            self.quests.hold(atlas, &mut analysis.quest);
        }
        let mut sample = Sample::from_analysis(at, &analysis);
        if let Some(found) = settled {
            sample.town = found.town;
            sample.kill_exp = found.kill_exp;
        }
        let mut other = false;
        if !self.held {
            if self.tracker.is_other_character(&sample) {
                other = true;
                self.reset();
            }
            self.tracker.push(&sample);
        }
        self.look(frame, at);
        self.follow_windows(frame, &analysis);
        analysis.tooltips.clone_from(&self.items);
        let hud = analysis.hud.is_some();
        let ms = started.elapsed().as_secs_f32() * 1000.0;
        Some((LiveRead { analysis, session: self.tracker.summary(), hud, ms }, other))
    }
}

impl LiveReader {
    /// Looks at the Pickup Log alone in `frame`, taken at `at` Unix seconds, as the program does
    /// four times a second between full reads: a line stays readable for only a second or two.
    /// Returns whether a line arrived. The website has no OCR for the lines its font can't finish;
    /// their unknown glyphs stand as gaps that item names fill in.
    pub fn look(&mut self, frame: &Frame, at: f64) -> bool {
        self.hover(frame, at);
        if !self.pickup_log {
            return false;
        }
        let read = self.pickups.read(at, frame, self.ui_scale, &self.ocr);
        if read.layout.is_some() {
            self.tracker.watching_loot();
        }
        if read.arrivals.is_empty() {
            return false;
        }
        let lines: Vec<Pickup> = read.arrivals.iter().map(|a| as_pickup(self.catalog.as_ref(), a)).collect();
        self.tracker.push_pickups(&lines);
        true
    }

    /// The player's switch: count the Pickup Log.
    pub fn set_pickup_log(&mut self, on: bool) {
        self.pickup_log = on;
    }

    /// The player's switch: read a tooltip the mouse rests on. Off forgets the last items.
    pub fn set_item_tooltips(&mut self, on: bool) {
        self.item_tooltips = on;
        if !on {
            self.hover = None;
            self.items_changed = !self.items.is_empty();
            self.items.clear();
        }
    }

    /// The item tooltips last read, if they changed since this was last asked.
    pub fn take_items(&mut self) -> Option<Vec<ItemTooltip>> {
        std::mem::take(&mut self.items_changed).then(|| self.items.clone())
    }

    /// Reads the tooltips in `frame` once they have stood in the same place for [`HOLD`] seconds:
    /// the mouse resting on an item, not passing over it. Each resting place is read once, and
    /// what it names stays until another is read: a tooltip that doesn't settle on an item
    /// leaves the last card as it was. The names go out first; the stats, slower to read, at the
    /// next look while the tooltips are still there.
    fn hover(&mut self, frame: &Frame, at: f64) {
        if !self.item_tooltips || self.recogniser.is_none() || self.catalog.is_none() {
            return;
        }
        let found = tooltip::name_lines(frame);
        if found.is_empty() {
            self.hover = None;
            return;
        }
        let places: Vec<Rect> = found.iter().map(|n| n.bullet).collect();
        let (Some(recogniser), Some(catalog)) = (self.recogniser.as_deref(), self.catalog.as_ref()) else { return };
        let hover = match &mut self.hover {
            Some(hover) if same_places(&hover.places, &places) => hover,
            _ => {
                self.hover = Some(Hover { places, since: at, read: false, stats_for: Vec::new() });
                return;
            }
        };
        if !hover.read {
            if at - hover.since < HOLD {
                return;
            }
            hover.read = true;
            let named = self.tooltips.named(recogniser, catalog, frame, found);
            if named.is_empty() {
                return;
            }
            // A new copy of the same item may differ, so its card starts again from the name.
            let items: Vec<ItemTooltip> = named.iter().map(|(item, _)| item.clone()).collect();
            hover.stats_for = named.into_iter().filter(|(item, _)| !item.plain).map(|(_, drawn)| drawn).collect();
            if !same_items(&items, &self.items) {
                self.items = items;
                self.items_changed = true;
            }
            return;
        }
        if hover.stats_for.is_empty() {
            return;
        }
        let pending = std::mem::take(&mut hover.stats_for);
        // Each pending name with its item, as shown now (they are matched by where they stand).
        let at = |drawn: &NameLines| drawn.bullet.x - 4.0 * drawn.unit;
        let mut pairs: Vec<(ItemTooltip, NameLines)> = pending
            .into_iter()
            .filter_map(|drawn| Some((self.items.iter().find(|t| (t.panel.x - at(&drawn)).abs() < 1.0)?.clone(), drawn)))
            .collect();
        self.tooltips.fill_stats(recogniser, catalog, frame, &mut pairs);
        let mut items = self.items.clone();
        for (read, _) in pairs {
            if let Some(item) = items.iter_mut().find(|t| t.panel.x == read.panel.x) {
                *item = read;
            }
        }
        if !same_items(&items, &self.items) {
            self.items = items;
            self.items_changed = true;
        }
    }

    /// The item tooltips in `frame` whose names settle on one item in the data: nothing until
    /// the page has handed over ocrs's model and `items.json`.
    pub fn read_items(&mut self, frame: &Frame) -> Vec<ItemTooltip> {
        let (Some(recogniser), Some(catalog)) = (self.recogniser.as_deref(), self.catalog.as_ref()) else { return Vec::new() };
        self.tooltips.read(recogniser, catalog, frame)
    }

    /// The tooltips' names in `frame` as read, before settling (for measuring).
    pub fn read_item_names(&mut self, frame: &Frame) -> Vec<ReadName> {
        let Some(recogniser) = self.recogniser.as_deref() else { return Vec::new() };
        self.tooltips.read_names(recogniser, frame)
    }
}

impl LiveReader {
    /// The player's switches: update the build from the Character Stat and Skill windows; check
    /// the inventory tab that's open (sell or keep); keep the quest log from the Quest window.
    pub fn set_windows(&mut self, character: bool, bag: bool, quest_log: bool) {
        self.choice = WindowChoice { character, bag, quest_log };
        if !bag {
            self.windows.bag = inventory::TabWatch::default();
        }
    }

    /// What the windows gave since this was last asked; `None` with nothing new.
    pub fn take_windows(&mut self) -> Option<WindowReads> {
        let reads = std::mem::take(&mut self.windows.reads);
        (!reads.is_empty()).then_some(reads)
    }

    /// Follows the game's windows on a full read, as the program's live loop does: each is read
    /// once it has shown the same twice in a row, and again only when that changes.
    fn follow_windows(&mut self, frame: &Frame, analysis: &Analysis) {
        let hud = analysis.hud.as_ref();
        let name = hud.and_then(|h| h.name.clone());
        let level = hud.and_then(|h| h.level);
        let scale = analysis.hud_layout.as_ref().map(|layout| ui::hud_scale(frame, layout));
        if self.choice.character
            && let Some(layout) = &analysis.hud_layout
        {
            let glance = character::glance(frame, layout, level);
            // A window can't be matched to a build without the player's name.
            if self.windows.character.settled(glance)
                && name.is_some()
                && let Some(windows) = self.read_character_windows(frame, Some(layout))
            {
                self.windows.reads.character = Some(CharacterScan { name: name.clone(), level, windows });
            }
        }
        if self.choice.bag
            && let Some(seen) = self.windows.bag.settled(inventory::read_video(frame))
        {
            self.windows.reads.bag = Some(SeenBag::new(&seen, name.clone(), level));
            self.windows.bag.checked(seen);
        }
        if self.choice.quest_log && self.text.is_some() {
            let glance = questlog::glance(frame, scale);
            if glance.is_none() {
                self.windows.quest_closed += 1;
                if self.windows.quest_closed >= QUEST_WINDOW_CLOSED_AFTER {
                    self.windows.quest_fresh = true;
                }
            } else {
                self.windows.quest_closed = 0;
            }
            if self.windows.quest_log.settled(glance)
                && let Some(name) = name
                && let Some(reading) = self.read_quest_window(frame, scale)
            {
                let fresh = std::mem::take(&mut self.windows.quest_fresh);
                self.windows.reads.quest_log = Some(QuestLogSeen { name, reading, fresh });
            }
        }
    }

    /// The Character Stat and Skill windows in `frame`, skill names read by ocrs; `None` with
    /// neither open (or before the page has handed over ocrs's model).
    pub fn read_character_windows(&self, frame: &Frame, layout: Option<&leafline_core::readers::hud::HudLayout>) -> Option<character::CharacterReading> {
        let text = self.text.as_ref()?;
        let windows = character::read(frame, layout, text).ok()?;
        (windows.stats.is_some() || windows.skills.is_some() || windows.detail.is_some()).then_some(windows)
    }

    /// The Quest window in `frame`, read by ocrs; `None` with it closed (or before the page has
    /// handed over ocrs's model).
    pub fn read_quest_window(&self, frame: &Frame, scale: Option<f32>) -> Option<QuestLogReading> {
        questlog::read_list_only(frame, scale, self.text.as_ref()?).ok().flatten()
    }
}

/// Tooltips found at the last looks: where (their bullets), since when, and whether read.
struct Hover {
    places: Vec<Rect>,
    since: f64,
    read: bool,
    /// The equipment names read whose stats are still to be read.
    stats_for: Vec<NameLines>,
}

/// A settled log line as the session counts it, as the program's live loop takes it
/// (`src-tauri/src/app/live.rs`).
fn as_pickup(catalog: Option<&Catalog>, arrival: &pickup::Arrival) -> Pickup {
    let Some(line) = &arrival.line else { return Pickup::Unread };
    match &line.gain {
        pickup::Gain::Mesos { amount } => Pickup::Mesos(*amount),
        pickup::Gain::Exp { amount } => Pickup::Exp { amount: *amount, party: line.party },
        pickup::Gain::PartyBonus { amount } => Pickup::PartyBonus(*amount),
        pickup::Gain::Item { name, quantity, tab, cut_short } => match catalog.and_then(|c| name_item(c, name, *tab, *cut_short)) {
            Some(item) => Pickup::Item { id: Some(item.id), name: item.name.clone(), count: *quantity },
            None => Pickup::Item { id: None, name: name.clone(), count: *quantity },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use leafline_core::capturetest::pattern::{self, WindowInfo};

    #[test]
    fn a_frame_with_no_hud_reads_and_tracks_nothing() {
        let frame = pattern::draw(WindowInfo { width: 1280, height: 720, scale_milli: 1000 });
        let mut live = LiveReader::new(15.0);
        let (read, other) = live.read(&frame, 1_000.0).expect("read");
        assert!(!read.hud && !other);
        assert_eq!(read.session.exp_gained, 0);
    }

    #[test]
    fn a_snapshot_carries_the_session_on() {
        let live = LiveReader::new(15.0);
        let saved = live.snapshot();
        assert!(LiveReader::restore(20.0, saved).is_some());
        assert!(LiveReader::restore(20.0, Value::String("nonsense".into())).is_none());
    }
}
