//! Commands the frontend can call.

use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

use super::overlay::{self, OverlayStatus};
use super::settings::SettingsState;
use super::shortcuts::{self, ShortcutInfo};
use crate::error::{Error, Result};

#[tauri::command]
pub fn overlay_status(app: AppHandle) -> Result<OverlayStatus> {
    overlay::status(&app)
}

#[tauri::command]
pub fn set_click_through(app: AppHandle, enabled: bool) -> Result<()> {
    overlay::set_click_through(&app, enabled)
}

#[tauri::command]
pub fn set_compact(app: AppHandle, enabled: bool) -> Result<()> {
    overlay::set_compact(&app, enabled)
}

#[tauri::command]
pub fn hide_overlay(app: AppHandle) -> Result<()> {
    overlay::hide(&app)
}

#[tauri::command]
pub fn toggle_overlay(app: AppHandle) -> Result<()> {
    overlay::toggle_visibility(&app)
}

#[tauri::command]
pub fn shortcut_list(app: AppHandle) -> Vec<ShortcutInfo> {
    shortcuts::list(&app, &app.state::<SettingsState>().get().hotkeys)
}

/// Opens the folder that holds the player's records, settings and backups.
#[tauri::command]
pub fn open_data_folder(app: AppHandle) -> Result<()> {
    let folder = app.path().app_data_dir()?;
    app.opener().open_path(folder.to_string_lossy(), None::<&str>).map_err(|err| Error::Invalid(err.to_string()))
}

/// The folder saved game pictures go to, as the Settings window shows it.
#[tauri::command]
pub fn frames_folder(app: AppHandle) -> Result<String> {
    Ok(super::live::frames_folder(&app)?.display().to_string())
}

/// Opens the folder saved game pictures go to, making it first if needed.
#[tauri::command]
pub fn open_frames_folder(app: AppHandle) -> Result<()> {
    let folder = super::live::frames_folder(&app)?;
    std::fs::create_dir_all(&folder)?;
    app.opener().open_path(folder.to_string_lossy(), None::<&str>).map_err(|err| Error::Invalid(err.to_string()))
}
