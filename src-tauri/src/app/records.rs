//! What the player records: the price log, saved sessions, notes, spots, potions and the drop
//! log. The logic is shared with the website in `leafline_core::service::records`; this adds what
//! only the program does: opening its data folder's file, the desk view's log tab, and saving
//! CSV and pictures where the player picked.

#[allow(unused_imports)]
use tauri::AppHandle;

#[allow(unused_imports)]
use super::host::AppHost;
pub use leafline_core::service::records::*;

use tauri::Manager;

use crate::error::{Error, Result};

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
