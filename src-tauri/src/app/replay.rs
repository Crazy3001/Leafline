//! Replaying a folder of screenshots as a session, for the workbench's offline mode.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use super::calibration::HudFontState;
use super::commands::ANALYSIS_EVENT;
use super::overlay::{OVERLAY, WORKBENCH};
use crate::error::{Error, Result};
use crate::pipeline::Analysis;
use crate::replay::{self, Order};
use crate::session::SessionSummary;
use crate::vision::Ocr;

/// Emitted to the workbench after each replayed frame.
pub const PROGRESS_EVENT: &str = "replay-progress";
/// Emitted to the overlay with each new session summary.
pub const SESSION_EVENT: &str = "session";

#[derive(Default)]
pub struct ReplayState {
    running: AtomicBool,
    cancel: AtomicBool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Progress<'a> {
    done: usize,
    total: usize,
    path: &'a Path,
    at: f64,
    analysis: &'a Analysis,
    session: &'a SessionSummary,
}

/// Runs every screenshot in `folder` through the pipeline in `order` and tracks them as one
/// session, showing each frame on the overlay as it goes.
#[tauri::command]
pub async fn replay_session(app: AppHandle, folder: PathBuf, order: Order) -> Result<SessionSummary> {
    let state = app.state::<ReplayState>();
    if state.running.swap(true, Ordering::SeqCst) {
        return Err(Error::Invalid("A replay is already running.".into()));
    }
    state.cancel.store(false, Ordering::SeqCst);
    let hud_font = app.state::<HudFontState>().current();
    let handle = app.clone();
    let result = tauri::async_runtime::spawn_blocking(move || -> Result<SessionSummary> {
        let frames = replay::frames(&folder, order)?;
        let total = frames.len();
        let ocr = Ocr::new()?;
        replay::run(&frames, order.tracker_config(), &ocr, &hud_font, |i, frame, analysis, session| {
            let progress = Progress { done: i + 1, total, path: &frame.path, at: frame.at, analysis, session };
            // A closed window just misses the update; the replay carries on.
            let _ = handle.emit_to(WORKBENCH, PROGRESS_EVENT, &progress);
            let _ = handle.emit_to(OVERLAY, ANALYSIS_EVENT, analysis);
            let _ = handle.emit_to(OVERLAY, SESSION_EVENT, session);
            !handle.state::<ReplayState>().cancel.load(Ordering::SeqCst)
        })
    })
    .await
    .map_err(|err| Error::Invalid(err.to_string()));
    state.running.store(false, Ordering::SeqCst);
    result?
}

/// Stops a running replay after the frame it is on.
#[tauri::command]
pub fn cancel_replay(app: AppHandle) {
    app.state::<ReplayState>().cancel.store(true, Ordering::SeqCst);
}
