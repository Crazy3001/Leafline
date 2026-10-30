//! Display settings for the overlay and pinned panels, and the Settings window.

use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};

use super::overlay::{self, OVERLAY};
use super::pins;
use crate::error::{Error, Result};
use crate::mapdata::TravelChoice;

const FILE_NAME: &str = "settings.json";
pub const SETTINGS: &str = "settings";

/// Emitted to every window when the settings change.
pub const SETTINGS_EVENT: &str = "settings";

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// How opaque the overlay and pinned panels are, 0.4 to 1.
    pub opacity: f64,
    /// How large their text and controls are drawn, 0.85 to 1.3.
    pub scale: f64,
    /// Taxis and return scrolls in routes. Off until the player turns it on.
    pub travel: TravelChoice,
}

impl Default for Settings {
    fn default() -> Self {
        Self { opacity: 1.0, scale: 1.0, travel: TravelChoice::default() }
    }
}

impl Settings {
    fn clamped(self) -> Self {
        let finite = |v: f64, fallback: f64| if v.is_finite() { v } else { fallback };
        Self {
            opacity: finite(self.opacity, 1.0).clamp(0.4, 1.0),
            scale: finite(self.scale, 1.0).clamp(0.85, 1.3),
            ..self
        }
    }
}

pub struct SettingsState {
    path: PathBuf,
    current: Mutex<Settings>,
}

impl SettingsState {
    /// Loads the saved settings. A missing or unreadable file means the defaults.
    pub fn load(app: &AppHandle) -> Result<Self> {
        let path = app.path().app_data_dir()?.join(FILE_NAME);
        let current = std::fs::read_to_string(&path)
            .ok()
            .and_then(|json| serde_json::from_str::<Settings>(&json).ok())
            .unwrap_or_default()
            .clamped();
        Ok(Self { path, current: Mutex::new(current) })
    }

    pub fn get(&self) -> Settings {
        *self.current.lock().expect("settings lock")
    }
}

/// Draws the overlay at the saved scale; called once at startup.
pub fn apply(app: &AppHandle) -> Result<()> {
    let scale = app.state::<SettingsState>().get().scale;
    if let Some(overlay) = app.get_webview_window(OVERLAY) {
        overlay.set_zoom(scale)?;
    }
    Ok(())
}

#[tauri::command]
pub fn get_settings(state: State<SettingsState>) -> Settings {
    state.get()
}

#[tauri::command]
pub fn update_settings(app: AppHandle, state: State<SettingsState>, settings: Settings) -> Result<Settings> {
    let settings = settings.clamped();
    let old = std::mem::replace(&mut *state.current.lock().expect("settings lock"), settings);
    if let Some(dir) = state.path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let json = serde_json::to_string_pretty(&settings).map_err(|err| Error::Invalid(err.to_string()))?;
    std::fs::write(&state.path, json)?;

    if old.scale != settings.scale {
        // The overlay grows with its contents so the same amount stays in view. Pinned panels
        // fit themselves to their contents once redrawn.
        if let Some(overlay) = app.get_webview_window(OVERLAY) {
            overlay.set_zoom(settings.scale)?;
            overlay::rescale(&app, settings.scale / old.scale)?;
        }
        for window in pins::windows(&app) {
            window.set_zoom(settings.scale)?;
        }
    }
    app.emit(SETTINGS_EVENT, settings)?;
    Ok(settings)
}

/// Opens the Settings window, or brings it forward.
#[tauri::command]
pub async fn show_settings(app: AppHandle) -> Result<()> {
    open(&app)
}

pub fn open(app: &AppHandle) -> Result<()> {
    if let Some(window) = app.get_webview_window(SETTINGS) {
        window.show()?;
        window.unminimize()?;
        window.set_focus()?;
        return Ok(());
    }
    WebviewWindowBuilder::new(app, SETTINGS, WebviewUrl::App("settings".into()))
        .title("Leafline — Settings")
        .inner_size(640.0, 720.0)
        .min_inner_size(520.0, 480.0)
        .center()
        .background_color(tauri::window::Color(13, 16, 23, 255))
        .build()?;
    Ok(())
}
