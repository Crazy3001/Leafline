//! The game's Quest window, read from live pictures while the player has it open, kept as each
//! character's quest log. Nothing is read with the window shut, and nothing read leaves the PC.

use tauri::AppHandle;

use super::host::AppHost;
use super::report;
use crate::pipeline::Analysis;
use crate::readers::questlog::{self, Watch};
use crate::readers::ui;
use crate::vision::{Frame, Ocr};
use leafline_core::service::questwindow::{QuestWindow, Strictness};

/// Pictures in a row without the window before it counts as closed, and the next opening as a
/// fresh look. One missed picture doesn't close it.
const CLOSED_AFTER: u32 = 3;

/// Follows the Quest window across live pictures: reads it once a change has settled, and puts
/// each tab together from the views the player scrolls through.
pub struct QuestLogFollow {
    watch: Watch,
    window: QuestWindow,
    closed: u32,
}

impl Default for QuestLogFollow {
    fn default() -> Self {
        Self { watch: Watch::default(), window: QuestWindow::new(Strictness::AsRead), closed: 0 }
    }
}

impl QuestLogFollow {
    pub fn follow(&mut self, app: &AppHandle, frame: &Frame, analysis: &Analysis, ocr: &Ocr) {
        let scale = analysis.hud_layout.as_ref().map(|layout| ui::hud_scale(frame, layout));
        let glance = questlog::glance(frame, scale);
        if glance.is_none() {
            self.closed += 1;
            if self.closed >= CLOSED_AFTER {
                self.window.closed();
            }
        } else {
            self.closed = 0;
        }
        if !self.watch.settled(glance) {
            return;
        }
        // The log is kept per character, so the name has to be known.
        let Some(name) = analysis.hud.as_ref().and_then(|hud| hud.name.clone()) else { return };
        match questlog::read(frame, scale, ocr) {
            Ok(Some(reading)) => {
                super::live::see(app, crate::sight::Reader::QuestWindow);
                if let Err(err) = self.window.keep(&AppHost::new(app), &name, &reading) {
                    report::note(format!("Couldn't keep the quest log: {err}"));
                }
            }
            Ok(None) => {}
            Err(err) => report::note(format!("Couldn't read the Quest window: {err}")),
        }
    }
}
