//! The latest readings, shared by the overlay and every pinned panel. Each window gets them as
//! events, and a window opened later asks for the current ones.

use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::pipeline::Analysis;
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

/// Shares a new analysis with every window. A window that is closed just misses it.
pub fn publish_analysis(app: &AppHandle, analysis: &Analysis) {
    *app.state::<Readings>().analysis.lock().expect("readings lock") = Some(analysis.clone());
    let _ = app.emit(ANALYSIS_EVENT, analysis);
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
