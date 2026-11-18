//! Part C's readers: the ones the website could run on a shared window today, timed frame by
//! frame. They are the program's own, with no text recognition: the website has no OCR yet, so
//! the HUD's digits, the Pickup Log's glyphs, the quick slots and the shop check read, and what
//! needs OCR (names, mesos, the map's name) finds nothing. The times are what reading costs a
//! browser tab, not how well it reads.

use serde::Serialize;

use crate::clock::Instant;
use crate::pipeline::{self, Timing};
use crate::readers::pickup::PickupReader;
use crate::readers::ui;
use crate::vision::{Frame, Ocr};

pub struct Readers {
    ocr: Ocr,
    pickup: PickupReader,
    /// The scale the HUD was last drawn at, for the Pickup Log.
    ui_scale: Option<f32>,
    /// When everything was last read, in seconds.
    last_full: Option<f64>,
}

impl Default for Readers {
    fn default() -> Self {
        Self { ocr: Ocr::none(), pickup: PickupReader::default(), ui_scale: None, last_full: None }
    }
}

/// What one frame cost and showed.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Read {
    /// Everything was read, not just the Pickup Log.
    pub full: bool,
    pub ms: f32,
    pub timings: Vec<Timing>,
    /// The HUD was found, and what it showed.
    pub hud: bool,
    pub level: Option<u32>,
    pub exp_percent: Option<f32>,
    /// The Pickup Log was found, and how many lines settled from this frame.
    pub pickup_log: bool,
    pub pickup_lines: usize,
}

impl Readers {
    /// Reads `frame`, taken at `at` seconds: the Pickup Log every time (the program looks four
    /// times a second), everything else once every `full_every` seconds.
    pub fn read(&mut self, frame: &Frame, at: f64, full_every: f64) -> Read {
        let started = Instant::now();
        let full = self.last_full.is_none_or(|t| at - t >= full_every || at < t);
        let mut read = Read {
            full,
            ms: 0.0,
            timings: Vec::new(),
            hud: false,
            level: None,
            exp_percent: None,
            pickup_log: false,
            pickup_lines: 0,
        };
        if full {
            self.last_full = Some(at);
            if let Ok(analysis) = pipeline::analyze_live(frame, &self.ocr) {
                if let Some(layout) = &analysis.hud_layout {
                    self.ui_scale = Some(ui::hud_scale(frame, layout));
                }
                read.hud = analysis.hud.is_some();
                read.level = analysis.hud.as_ref().and_then(|h| h.level);
                read.exp_percent = analysis.hud.as_ref().and_then(|h| h.exp.map(|e| e.percent).or(h.exp_percent));
                read.timings = analysis.timings;
            }
        }
        let looked = Instant::now();
        let pickups = self.pickup.read(at, frame, self.ui_scale, &self.ocr);
        read.timings.push(Timing { stage: "Pickup Log", ms: looked.elapsed().as_secs_f32() * 1000.0 });
        read.pickup_log = pickups.layout.is_some();
        read.pickup_lines = pickups.arrivals.iter().filter(|a| a.line.is_some()).count();
        read.ms = started.elapsed().as_secs_f32() * 1000.0;
        read
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capturetest::pattern::{self, WindowInfo};

    #[test]
    fn reads_a_frame_without_ocr_and_times_it() {
        let frame = pattern::draw(WindowInfo { width: 1280, height: 720, scale_milli: 1000 });
        let mut readers = Readers::default();
        let first = readers.read(&frame, 0.0, 1.0);
        assert!(first.full && !first.hud);
        assert!(first.timings.iter().any(|t| t.stage == "Pickup Log"));
        let second = readers.read(&frame, 0.25, 1.0);
        assert!(!second.full, "only the Pickup Log between full reads");
        assert!(readers.read(&frame, 1.1, 1.0).full);
    }
}
