//! The desk view: the panels side by side in one ordinary window, for a second screen. Unlike the
//! overlay and pins it is not meant to sit over the game, so it has a frame and a taskbar entry
//! and stays off the top unless the player asks. It opens again at launch if it was open at quit.

use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder, Window};

use super::settings::SettingsState;
use crate::error::{Error, Result};

const FILE_NAME: &str = "desk.json";

/// The desk view's window label.
pub const DESK: &str = "desk";

/// What is remembered about the desk view between runs. Its place and size are kept by the
/// window-state plugin, like every other window's; the layout is the page's own.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DeskStatus {
    /// Open when Leafline last quit, so it opens again.
    pub open: bool,
    /// Kept above other windows. Off by default: on a screen of its own it covers nothing.
    pub on_top: bool,
}

pub struct DeskState {
    path: PathBuf,
    status: Mutex<DeskStatus>,
}

impl DeskState {
    /// A missing or unreadable file means closed and not on top.
    pub fn load(app: &AppHandle) -> Result<Self> {
        let path = app.path().app_data_dir()?.join(FILE_NAME);
        let status = std::fs::read_to_string(&path).ok().map(|json| parse(&json)).unwrap_or_default();
        Ok(Self { path, status: Mutex::new(status) })
    }

    fn get(&self) -> DeskStatus {
        *self.status.lock().expect("desk lock")
    }

    fn update(&self, change: impl FnOnce(&mut DeskStatus)) -> Result<DeskStatus> {
        let mut status = self.status.lock().expect("desk lock");
        change(&mut status);
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_string(&*status).map_err(|err| Error::Invalid(err.to_string()))?;
        std::fs::write(&self.path, json)?;
        Ok(*status)
    }
}

fn parse(json: &str) -> DeskStatus {
    serde_json::from_str(json).unwrap_or_default()
}

/// Whether the desk view is on screen. Links and item checks then show there, so the overlay
/// doesn't have to come up for them.
pub fn showing(app: &AppHandle) -> bool {
    app.get_webview_window(DESK).is_some_and(|w| w.is_visible().unwrap_or(false) && !w.is_minimized().unwrap_or(false))
}

/// Opens the desk view if it was open when Leafline last quit.
pub fn restore(app: &AppHandle) -> Result<()> {
    if app.state::<DeskState>().get().open { open(app) } else { Ok(()) }
}

fn open(app: &AppHandle) -> Result<()> {
    if let Some(window) = app.get_webview_window(DESK) {
        window.show()?;
        window.unminimize()?;
        window.set_focus()?;
        return Ok(());
    }
    let settings = app.state::<SettingsState>().get();
    WebviewWindowBuilder::new(app, DESK, WebviewUrl::App("desk".into()))
        .title("Leafline — Desk view")
        .inner_size(1600.0, 900.0)
        .min_inner_size(640.0, 420.0)
        .center()
        .background_color(tauri::window::Color(20, 20, 18, 255))
        .always_on_top(app.state::<DeskState>().get().on_top)
        // Like the overlay, a click leaves the game focused unless the player turned that off;
        // typing in a box takes focus while it lasts (see settings::set_typing).
        .focusable(!settings.keep_game_focus)
        .focused(!settings.keep_game_focus)
        .additional_browser_args(super::BROWSER_ARGS)
        .build()?;
    Ok(())
}

/// The player closed the desk view with its window's own button: it stays closed next launch.
/// Quitting Leafline closes it without this, so it comes back.
pub fn closed(window: &Window) {
    if window.label() != DESK {
        return;
    }
    if let Err(err) = window.app_handle().state::<DeskState>().update(|s| s.open = false) {
        eprintln!("could not save the desk view: {err}");
    }
}

#[tauri::command]
pub fn desk_status(state: State<DeskState>) -> DeskStatus {
    state.get()
}

// From an async command: a synchronous one would block the event loop the new window needs.
#[tauri::command]
pub async fn show_desk(app: AppHandle) -> Result<DeskStatus> {
    let status = app.state::<DeskState>().update(|s| s.open = true)?;
    open(&app)?;
    Ok(status)
}

#[tauri::command]
pub fn set_desk_on_top(app: AppHandle, on_top: bool) -> Result<DeskStatus> {
    let status = app.state::<DeskState>().update(|s| s.on_top = on_top)?;
    if let Some(window) = app.get_webview_window(DESK) {
        window.set_always_on_top(on_top)?;
    }
    Ok(status)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_or_broken_file_means_closed_and_not_on_top() {
        assert_eq!(parse("not json"), DeskStatus::default());
        assert_eq!(parse("{}"), DeskStatus::default());
        assert_eq!(parse(r#"{"open":true}"#), DeskStatus { open: true, on_top: false });
        assert_eq!(
            serde_json::to_string(&DeskStatus { open: true, on_top: true }).unwrap(),
            r#"{"open":true,"onTop":true}"#
        );
    }
}
