//! Commands the frontend can call.

use std::path::PathBuf;

use tauri::ipc::Response;
use tauri::{AppHandle, Emitter, Manager};

use super::calibration::HudFontState;
use super::overlay::{self, OVERLAY, OverlayStatus};
use super::shortcuts::{self, ShortcutInfo};
use crate::error::{Error, Result};
use crate::pipeline::{self, Analysis};
use crate::vision::{self, Ocr};

/// Emitted to the overlay with each new analysis.
pub const ANALYSIS_EVENT: &str = "analysis";

/// Runs the pipeline on a saved screenshot (offline test mode) and shows the result on the
/// overlay as it would appear live.
#[tauri::command]
pub async fn analyze_image(app: AppHandle, path: PathBuf) -> Result<Analysis> {
    let hud_font = app.state::<HudFontState>().current();
    let analysis = tauri::async_runtime::spawn_blocking(move || -> Result<Analysis> {
        let frame = vision::load(&path)?;
        pipeline::analyze(&frame, &Ocr::new()?, &hud_font)
    })
    .await
    .map_err(|err| Error::Invalid(err.to_string()))??;
    app.emit_to(OVERLAY, ANALYSIS_EVENT, &analysis)?;
    Ok(analysis)
}

/// Supported images in `folder`, sorted by name.
#[tauri::command]
pub fn list_images(folder: PathBuf) -> Result<Vec<PathBuf>> {
    let mut images: Vec<PathBuf> = std::fs::read_dir(folder)?
        .filter_map(|entry| Some(entry.ok()?.path()))
        .filter(|path| vision::is_supported(path))
        .collect();
    images.sort();
    Ok(images)
}

/// The raw bytes of a screenshot, for display in the workbench.
#[tauri::command]
pub fn read_image(path: PathBuf) -> Result<Response> {
    if !vision::is_supported(&path) {
        return Err(Error::Invalid(format!("{} is not a supported image", path.display())));
    }
    Ok(Response::new(std::fs::read(path)?))
}

#[tauri::command]
pub fn overlay_status(app: AppHandle) -> Result<OverlayStatus> {
    overlay::status(&app)
}

#[tauri::command]
pub fn set_click_through(app: AppHandle, enabled: bool) -> Result<()> {
    overlay::set_click_through(&app, enabled)
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
pub fn show_workbench(app: AppHandle) -> Result<()> {
    overlay::show_workbench(&app)
}

#[tauri::command]
pub fn shortcut_list() -> Vec<ShortcutInfo> {
    shortcuts::list()
}
