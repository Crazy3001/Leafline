//! The website's live reading: a shared game window's frames read by the program's own readers
//! and fed to the program's own session tracker, as `src-tauri`'s live loop does. Only what reads
//! without Windows OCR runs here for now: level and EXP from the HUD's digits, and mesos from the
//! game's digit shapes. The map's name and the Quest Helper's titles need the game-font readers
//! still to come, and read nothing until then. Frames come in as RGBA bytes from the page's worker and never leave it.

use serde::Serialize;
use serde_json::Value;

use leafline_core::pipeline::{self, Analysis};
use leafline_core::session::{Sample, SessionSummary, SessionTracker, TrackerConfig};
use leafline_core::vision::{Frame, Ocr};

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
        Self { ocr: Ocr::none(), tracker: SessionTracker::new(config(idle_after)), idle_after, held: false }
    }

    /// Carries on with a session saved by [`LiveReader::snapshot`]; `None` if it can't be read.
    pub fn restore(idle_after: f64, saved: Value) -> Option<Self> {
        let tracker = SessionTracker::restore(config(idle_after), saved)?;
        Some(Self { ocr: Ocr::none(), tracker, idle_after, held: false })
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
        let analysis = pipeline::analyze_live(frame, &self.ocr).ok()?;
        let sample = Sample::from_analysis(at, &analysis);
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
