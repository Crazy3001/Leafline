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

use crate::identity::IdentityReader;
use crate::ocr::Recogniser;
use crate::title::{self, Settled, TitleReader};
use crate::words::DarkText;

/// Live frames are plentiful, so a reading must agree twice before it counts, as in the program.
const CONFIRMATIONS: usize = 2;

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

pub struct LiveReader {
    ocr: Ocr,
    /// ocrs, once the page has handed over its model; until then no text is read (the map, the
    /// character's name and job).
    recogniser: Option<Recogniser>,
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
            recogniser: None,
            title: TitleReader::default(),
            identity: IdentityReader::default(),
            words: DarkText::default(),
            dataset: Dataset::default(),
            atlas: None,
            quests: QuestTitles::default(),
            pickups: PickupReader::default(),
            catalog: None,
            ui_scale: None,
            tracker,
            idle_after,
            held: false,
        }
    }

    /// ocrs's text recognition model, for the minimap title and the HUD's name and job.
    pub fn set_text_model(&mut self, recognition: Vec<u8>) -> Result<(), String> {
        self.recogniser = Some(Recogniser::new(recognition)?);
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
        let Some(recogniser) = self.recogniser.as_ref() else { return Some((analysis, None)) };
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
