//! Keeps the live tracker's state on disk beside the saved session, so a crash, a closed app or
//! a restart picks the session up where it was instead of starting a new one. The log already
//! holds the session's totals; this holds what the tracker needs to carry on adding to them.
//! A snapshot older than the break that ends a session is not used.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};
use tauri::{AppHandle, Manager};

use super::records::UserDataState;
use crate::session::{SessionTracker, TrackerConfig};

const FILE_NAME: &str = "session-recovery.json";
/// Bumped when the tracker's saved shape changes in a way an old snapshot can't be read with.
const VERSION: u32 = 1;

fn path(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_data_dir().ok().map(|dir| dir.join(FILE_NAME))
}

fn unix_now() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

/// Writes the tracker's state, with the log id of the session it belongs to (`None` before the
/// session is long enough to be kept in the log). A failure is not worth stopping for.
pub fn write(app: &AppHandle, id: Option<i64>, tracker: &SessionTracker) {
    let Some(path) = path(app) else { return };
    let Ok(tracker) = serde_json::to_value(tracker) else { return };
    let text = json!({ "version": VERSION, "savedAt": unix_now(), "id": id, "tracker": tracker }).to_string();
    // Written beside and renamed, so a crash mid-write leaves the last snapshot whole.
    let temp = path.with_extension("tmp");
    if std::fs::write(&temp, text).is_ok() {
        let _ = std::fs::rename(&temp, &path);
    }
}

/// Forgets the snapshot: the session ended, or the player started a new one.
pub fn clear(app: &AppHandle) {
    if let Some(path) = path(app) {
        let _ = std::fs::remove_file(path);
    }
}

/// The tracker and log id to carry on with, if a snapshot was left less than `max_age` seconds
/// ago and its session is still in the log.
pub fn load(app: &AppHandle, config: TrackerConfig, max_age: f64) -> Option<(SessionTracker, Option<i64>)> {
    let text = std::fs::read_to_string(path(app)?).ok()?;
    let saved: Value = serde_json::from_str(&text).ok()?;
    if saved.get("version")?.as_u64()? != u64::from(VERSION) {
        return None;
    }
    if unix_now() - saved.get("savedAt")?.as_f64()? > max_age {
        return None;
    }
    let id = saved.get("id").and_then(Value::as_i64);
    if let Some(id) = id {
        // The player may have deleted it from the log since.
        let kept = app.state::<UserDataState>().with(|db| db.sessions()).ok()?.iter().any(|s| s.id == id);
        if !kept {
            return None;
        }
    }
    let tracker = SessionTracker::restore(config, saved.get("tracker")?.clone())?;
    Some((tracker, id))
}
