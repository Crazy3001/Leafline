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
