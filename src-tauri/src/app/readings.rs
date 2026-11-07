//! The latest readings, shared by the overlay and every pinned panel. Each window gets them as
//! events, and a window opened later asks for the current ones.

use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::pipeline::Analysis;
use crate::readers::tooltip::ItemTooltip;
use crate::session::SessionSummary;

/// Emitted to every window with each new analysis.
pub const ANALYSIS_EVENT: &str = "analysis";
/// Emitted to every window with each new session summary.
pub const SESSION_EVENT: &str = "session";

#[derive(Default)]
pub struct Readings {
    analysis: Mutex<Option<Analysis>>,
    session: Mutex<Option<SessionSummary>>,
}

#[derive(Serialize)]
pub struct Current {
    analysis: Option<Analysis>,
    session: Option<SessionSummary>,
}

/// Shares a live analysis with every window; a window that is closed just misses it. Those read
/// no items, so the last item read stays on show.
pub fn publish_live_analysis(app: &AppHandle, mut analysis: Analysis) {
    let state = app.state::<Readings>();
    let mut current = state.analysis.lock().expect("readings lock");
    if let Some(previous) = current.as_mut() {
        analysis.tooltips = std::mem::take(&mut previous.tooltips);
    }
    let _ = app.emit(ANALYSIS_EVENT, &analysis);
    *current = Some(analysis);
}

/// Shows the items an item check read, in place of the ones before.
pub fn publish_items(app: &AppHandle, width: u32, height: u32, items: Vec<ItemTooltip>) {
    let state = app.state::<Readings>();
    let mut current = state.analysis.lock().expect("readings lock");
    let analysis = match current.as_mut() {
        Some(analysis) => {
            analysis.tooltips = items;
            analysis
        }
        None => current.insert(Analysis::of_items(width, height, items)),
    };
    let _ = app.emit(ANALYSIS_EVENT, &*analysis);
}

/// The latest analysis, if any.
pub fn current_analysis(app: &AppHandle) -> Option<Analysis> {
    app.state::<Readings>().analysis.lock().expect("readings lock").clone()
}

/// The session the overlay is showing, if any.
pub fn current_session(app: &AppHandle) -> Option<SessionSummary> {
    app.state::<Readings>().session.lock().expect("readings lock").clone()
}

pub fn publish_session(app: &AppHandle, session: &SessionSummary) {
    *app.state::<Readings>().session.lock().expect("readings lock") = Some(session.clone());
    let _ = app.emit(SESSION_EVENT, session);
}

#[tauri::command]
pub fn current_readings(state: State<Readings>) -> Current {
    Current {
        analysis: state.analysis.lock().expect("readings lock").clone(),
        session: state.session.lock().expect("readings lock").clone(),
    }
}
