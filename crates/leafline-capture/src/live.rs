//! The website's live reading: a shared game window's frames read by the program's own readers
//! and fed to the program's own session tracker, as `src-tauri`'s live loop does. Only what reads
//! without Windows OCR runs here: level and EXP from the HUD's digits, mesos from the game's digit
//! shapes, and the map from the minimap title by ocrs once the page hands over its model and the
//! map list (see `title`). The Quest Helper's titles read nothing yet. Frames come in as RGBA bytes from the page's worker and never leave it.

use serde::Serialize;
use serde_json::Value;

use leafline_core::mapdata::atlas::Atlas;
use leafline_core::mapdata::osms::{Dataset, Records};
use leafline_core::pipeline::{self, Analysis};
use leafline_core::session::{Sample, SessionSummary, SessionTracker, TrackerConfig};
use leafline_core::vision::{Frame, Ocr};

use crate::title::{self, TitleReader};

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
    /// The minimap title's reader and the known maps it is settled against, once the page has
    /// handed over the model and the map data; until then no map is read.
    title: Option<TitleReader>,
    atlas: Option<Atlas>,
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
        Self { ocr: Ocr::none(), title: None, atlas: None, tracker: SessionTracker::new(config(idle_after)), idle_after, held: false }
    }

    /// Carries on with a session saved by [`LiveReader::snapshot`]; `None` if it can't be read.
    pub fn restore(idle_after: f64, saved: Value) -> Option<Self> {
        let tracker = SessionTracker::restore(config(idle_after), saved)?;
        Some(Self { ocr: Ocr::none(), title: None, atlas: None, tracker, idle_after, held: false })
    }

    /// ocrs's text recognition model, for the minimap title.
    pub fn set_title_model(&mut self, recognition: Vec<u8>) -> Result<(), String> {
        self.title = Some(TitleReader::new(recognition)?);
        Ok(())
    }

    /// The known maps (OSMS `maps.json`), to settle titles against.
    pub fn set_maps(&mut self, maps_json: &[u8]) -> Result<(), String> {
        let Records::Maps(maps) = Records::parse("maps.json", maps_json).map_err(|e| e.to_string())? else {
            return Err("not a map list".into());
        };
        self.atlas = Some(Atlas::new(Dataset { maps, ..Dataset::default() }));
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
        self.tracker = SessionTracker::new(config(self.idle_after));
        self.tracker.set_held(self.held);
    }

    pub fn summary(&self) -> SessionSummary {
        self.tracker.summary()
    }

    /// Reads `frame`, taken at `at` Unix seconds. `other` is set when it shows another character
    /// than the session's, which then starts afresh (the caller saves the old one first, from the
    /// summary it had).
    pub fn read(&mut self, frame: &Frame, at: f64) -> Option<(LiveRead, bool)> {
        let started = leafline_core::clock::Instant::now();
        let mut analysis = pipeline::analyze_live(frame, &self.ocr).ok()?;
        let mut settled = None;
        if analysis.map.is_none()
            && let (Some(reader), Some(atlas)) = (self.title.as_mut(), self.atlas.as_ref())
            && let Some(raw) = reader.read(frame)
            && let Some((area, _)) = leafline_core::readers::map::title_area(frame)
        {
            settled = title::settle(atlas, &raw, area);
        }
        if let Some(found) = &settled {
            analysis.map = Some(found.reading.clone());
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
        let hud = analysis.hud.is_some();
        let ms = started.elapsed().as_secs_f32() * 1000.0;
        Some((LiveRead { analysis, session: self.tracker.summary(), hud, ms }, other))
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
