//! What the player records: the price log, saved sessions, notes, spots, potions and the drop
//! log. The logic is shared with the website in `leafline_core::service::records`; this adds what
//! only the program does: opening its data folder's file, the desk view's log tab, and saving
//! CSV and pictures where the player picked.

#[allow(unused_imports)]
use tauri::AppHandle;

#[allow(unused_imports)]
use super::host::AppHost;
pub use leafline_core::service::records::*;

use std::path::{Path, PathBuf};

use tauri::{Emitter, Manager, State};

use crate::error::{Error, Result};
use crate::userdata::{RecordsSummary, UserData};

/// A records file the player picked, copied into the data folder to be checked and brought in.
const INCOMING: &str = "import.leafline";
/// The records as they were before a records file replaced them.
const BEFORE_IMPORT: &str = "leafline-before-import.sqlite3";

fn data_dir(app: &AppHandle) -> Result<PathBuf> {
    Ok(app.path().app_data_dir()?)
}

fn remove(path: &Path) -> Result<()> {
    match std::fs::remove_file(path) {
        Err(err) if err.kind() != std::io::ErrorKind::NotFound => Err(err.into()),
        _ => Ok(()),
    }
}

/// Saves every record to `path`, a `.leafline` file the player picked in a save dialog.
#[tauri::command]
pub fn export_records(state: State<UserDataState>, path: PathBuf) -> Result<RecordsSummary> {
    // The save dialog has already asked about replacing a file of that name.
    remove(&path)?;
    state.export_to(&path)
}

/// Says what the records file at `path` holds, changing nothing: it is copied first, since
/// checking brings an older file up to date.
#[tauri::command]
pub fn check_records(app: AppHandle, path: PathBuf) -> Result<RecordsSummary> {
    let incoming = data_dir(&app)?.join(INCOMING);
    remove(&incoming)?;
    std::fs::copy(&path, &incoming)?;
    UserData::open_file(&incoming)?.summary()
}

/// Replaces the records with the file `check_records` took in, keeping the ones before aside,
/// and tells every window to start afresh.
#[tauri::command]
pub fn import_records(app: AppHandle, state: State<UserDataState>) -> Result<RecordsSummary> {
    let dir = data_dir(&app)?;
    let summary = state.replace_from(&dir.join(FILE_NAME), &dir.join(INCOMING), &dir.join(BEFORE_IMPORT), &remove)?;
    let _ = remove(&dir.join(INCOMING));
    app.emit(RECORDS_REPLACED_EVENT, &summary)?;
    Ok(summary)
}

/// One of the records' daily copies, or the records kept aside before the last restore or import.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Backup {
    /// The file's name, which `check_backup` takes.
    pub name: String,
    /// The day it was taken, `2026-11-24`; none for the records kept aside.
    pub day: Option<String>,
    /// The records as they were before the last restore or import.
    pub before_restore: bool,
    /// When the file was last written, Unix seconds.
    pub saved_at: Option<i64>,
    pub summary: Option<RecordsSummary>,
    /// Why it couldn't be read, when it couldn't.
    pub problem: Option<String>,
}

/// A copy used to read a backup's summary, which brings an older file up to date.
const PEEK: &str = "backup-peek.sqlite3";

/// Where a backup named `name` is, if it is one: a daily copy in the backups folder, or the
/// records kept aside before the last restore. Nothing else can be named.
fn backup_path(dir: &Path, name: &str) -> Option<PathBuf> {
    if name == BEFORE_IMPORT {
        return Some(dir.join(BEFORE_IMPORT));
    }
    let day = name.strip_prefix("leafline-")?.strip_suffix(".sqlite3")?;
    let ok = day.len() == 10 && day.chars().all(|c| c.is_ascii_digit() || c == '-');
    ok.then(|| dir.join(BACKUP_FOLDER).join(name))
}

/// What a records file holds, read from a copy so the file itself is left as it is.
fn peek(dir: &Path, path: &Path) -> Result<RecordsSummary> {
    let copy = dir.join(PEEK);
    remove(&copy)?;
    std::fs::copy(path, &copy)?;
    let summary = UserData::open_file(&copy).and_then(|db| db.summary());
    let _ = remove(&copy);
    summary
}

/// The daily copies of the records, the newest first, then the records kept aside before the last
/// restore or import, each with what it holds.
#[tauri::command]
pub fn list_backups(app: AppHandle) -> Result<Vec<Backup>> {
    let dir = data_dir(&app)?;
    let mut names: Vec<String> = std::fs::read_dir(dir.join(BACKUP_FOLDER))
        .map(|entries| entries.filter_map(|e| Some(e.ok()?.file_name().to_string_lossy().into_owned())).collect())
        .unwrap_or_default();
    names.retain(|n| backup_path(&dir, n).is_some());
    names.sort_by(|a, b| b.cmp(a));
    if dir.join(BEFORE_IMPORT).exists() {
        names.push(BEFORE_IMPORT.to_string());
    }
    Ok(names
        .into_iter()
        .filter_map(|name| {
            let path = backup_path(&dir, &name)?;
            let saved_at = std::fs::metadata(&path)
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64);
            let (summary, problem) = match peek(&dir, &path) {
                Ok(summary) => (Some(summary), None),
                Err(err) => (None, Some(err.to_string())),
            };
            let before_restore = name == BEFORE_IMPORT;
            let day = (!before_restore).then(|| name["leafline-".len()..name.len() - ".sqlite3".len()].to_string());
            Some(Backup { name, day, before_restore, saved_at, summary, problem })
        })
        .collect())
}

/// Takes in a backup to restore, as `check_records` takes in a file: `import_records` then puts
/// it in place, keeping the records as they are now aside first.
#[tauri::command]
pub fn check_backup(app: AppHandle, name: String) -> Result<RecordsSummary> {
    let dir = data_dir(&app)?;
    let path = backup_path(&dir, &name).filter(|p| p.exists()).ok_or_else(|| Error::Invalid("That backup isn't there any more.".into()))?;
    let incoming = dir.join(INCOMING);
    remove(&incoming)?;
    std::fs::copy(&path, &incoming)?;
    UserData::open_file(&incoming)?.summary()
}

/// Opens the player's records in the app's data folder, with a copy a day in its backups folder.
pub fn load(app: &AppHandle) -> Result<UserDataState> {
    let dir = app.path().app_data_dir()?;
    Ok(UserDataState::load(&dir.join(FILE_NAME), Some(&dir.join(BACKUP_FOLDER))))
}

/// Opens the training log: a tab of the desk view.
#[tauri::command]
pub async fn show_training(app: AppHandle) -> Result<()> {
    super::desk::open_tab(&app, "log")
}

/// Writes the price log, the session history or the drop log as CSV to `path`, which the user
/// picked in a save dialog. Returns how many rows were written.
#[tauri::command]
pub fn export_csv(app: AppHandle, what: Export, path: std::path::PathBuf) -> Result<usize> {
    let (text, rows) = csv(&AppHost::new(&app), what)?;
    std::fs::write(path, text)?;
    Ok(rows)
}

/// The largest picture `save_image` writes: a share card is well under 1 MB.
const MAX_IMAGE: usize = 8 * 1024 * 1024;

/// Writes a picture the app drew, such as a session's share card, to `path`, which the player
/// picked in a save dialog. Only PNG files.
#[tauri::command]
pub fn save_image(path: std::path::PathBuf, png: Vec<u8>) -> Result<()> {
    if !path.extension().is_some_and(|e| e.eq_ignore_ascii_case("png")) {
        return Err(Error::Invalid("Save the picture as a .png file.".into()));
    }
    if png.len() > MAX_IMAGE || !png.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err(Error::Invalid("That isn't a picture Leafline drew.".into()));
    }
    std::fs::write(path, png)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_daily_copies_and_the_records_kept_aside_can_be_named() {
        let dir = Path::new("data");
        assert_eq!(backup_path(dir, "leafline-2026-11-24.sqlite3"), Some(dir.join("backups").join("leafline-2026-11-24.sqlite3")));
        assert_eq!(backup_path(dir, BEFORE_IMPORT), Some(dir.join(BEFORE_IMPORT)));
        for name in ["leafline.sqlite3", "leafline-../../x.sqlite3", "leafline-2026-11-24.sqlite3x", "other.sqlite3", r"..\leafline-2026-11-24.sqlite3"] {
            assert_eq!(backup_path(dir, name), None, "{name}");
        }
    }
}
