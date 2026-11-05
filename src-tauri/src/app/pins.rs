//! Panels pinned out of the overlay into small windows of their own. They stay on screen while
//! the overlay is hidden, follow its click-through lock, and come back at the next launch.

use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};
use tauri::{
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, PhysicalSize, State, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder, Window,
};

use super::overlay::{self, OVERLAY};
use super::settings::SettingsState;
use super::snap;
use crate::error::{Error, Result};

const FILE_NAME: &str = "pins.json";

/// Emitted to every window when panels are pinned, unpinned, shown or hidden.
pub const PINS_EVENT: &str = "pins";

/// Emitted to a pinned panel when the player resizes it, with its logical width and height.
const RESIZED_EVENT: &str = "pin-resized";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Panel {
    /// The character, this session and the build plan. Stats and Build were two tabs until
    /// 2026-11-04, so either old id still opens this one.
    #[serde(alias = "stats", alias = "build")]
    Character,
    Map,
    Train,
    Quests,
    Item,
}

impl Panel {
    const ALL: [Panel; 5] = [Panel::Character, Panel::Map, Panel::Train, Panel::Quests, Panel::Item];

    fn id(self) -> &'static str {
        match self {
            Panel::Character => "character",
            Panel::Map => "map",
            Panel::Train => "train",
            Panel::Quests => "quests",
            Panel::Item => "item",
        }
    }

    fn title(self) -> &'static str {
        match self {
            Panel::Character => "Character",
            Panel::Map => "Map",
            Panel::Train => "Train",
            Panel::Quests => "Quests",
            Panel::Item => "Lookup",
        }
    }

    fn label(self) -> String {
        format!("{PREFIX}{}", self.id())
    }
}

/// Logical width of a pinned panel, the overlay's own, so it fits as it does in a tab. The page
/// sets the height to fit what it shows, until the player resizes it by hand. Matches `PIN_WIDTH`
/// in src/lib/panels.ts.
const WIDTH: f64 = 340.0;

/// The smallest a pinned panel goes, before the size setting. Any narrower and the session
/// history in Character runs off the side.
const MIN_SIZE: (f64, f64) = (320.0, 60.0);

/// Space between the overlay and pinned panels placed beside it.
const GAP: f64 = 12.0;

/// Window labels of pinned panels start with this.
pub const PREFIX: &str = "pin-";

pub struct PinState {
    path: PathBuf,
    pinned: Mutex<Vec<Panel>>,
    hidden: AtomicBool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PinStatus {
    pinned: Vec<Panel>,
    /// Pinned panels are hidden for now (by hotkey or tray), without unpinning them.
    hidden: bool,
}

impl PinState {
    /// Loads which panels were pinned last time. A missing or unreadable file means none.
    pub fn load(app: &AppHandle) -> Result<Self> {
        let path = app.path().app_data_dir()?.join(FILE_NAME);
        let pinned = std::fs::read_to_string(&path).ok().map(|json| parse(&json)).unwrap_or_default();
        Ok(Self { path, pinned: Mutex::new(pinned), hidden: AtomicBool::new(false) })
    }

    fn status(&self) -> PinStatus {
        PinStatus {
            pinned: self.pinned.lock().expect("pins lock").clone(),
            hidden: self.hidden.load(Ordering::Relaxed),
        }
    }

    fn save(&self, pinned: &[Panel]) -> Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_string(pinned).map_err(|err| Error::Invalid(err.to_string()))?;
        std::fs::write(&self.path, json)?;
        Ok(())
    }
}

/// The pinned panels saved in `pins.json`, in order and each once. Unreadable means none. Older
/// files can name both Stats and Build, which are one panel now.
fn parse(json: &str) -> Vec<Panel> {
    let mut pinned: Vec<Panel> = serde_json::from_str(json).unwrap_or_default();
    let mut seen = Vec::new();
    pinned.retain(|p| {
        let first = !seen.contains(p);
        seen.push(*p);
        first
    });
    pinned
}

/// Whether a pinned Lookup panel, which shows items and monsters, is on screen.
pub fn shows_item(app: &AppHandle) -> bool {
    shows(app, Panel::Item)
}

pub fn shows_quests(app: &AppHandle) -> bool {
    shows(app, Panel::Quests)
}

fn shows(app: &AppHandle, panel: Panel) -> bool {
    let status = app.state::<PinState>().status();
    !status.hidden && status.pinned.contains(&panel)
}

/// Reopens the panels that were pinned when Leafline last closed.
pub fn restore(app: &AppHandle) -> Result<()> {
    let pinned = app.state::<PinState>().status().pinned;
    for (i, panel) in pinned.into_iter().enumerate() {
        open(app, panel, i)?;
    }
    Ok(())
}

/// The windows of every pinned panel.
pub fn windows(app: &AppHandle) -> impl Iterator<Item = WebviewWindow> {
    app.webview_windows().into_iter().filter(|(label, _)| label.starts_with(PREFIX)).map(|(_, window)| window)
}

fn open(app: &AppHandle, panel: Panel, slot: usize) -> Result<()> {
    if let Some(window) = app.get_webview_window(&panel.label()) {
        window.show()?;
        return Ok(());
    }
    let (x, y) = default_position(app, slot);
    let window = WebviewWindowBuilder::new(app, panel.label(), WebviewUrl::App(format!("pin/{}", panel.id()).into()))
        .title(format!("Leafline · {}", panel.title()))
        .inner_size(WIDTH, 120.0)
        .position(x, y)
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .always_on_top(app.state::<SettingsState>().get().always_on_top)
        .skip_taskbar(true)
        // Double-clicking the header would otherwise fill the screen.
        .maximizable(false)
        .focused(false)
        .focusable(!app.state::<SettingsState>().get().keep_game_focus)
        .visible(!app.state::<PinState>().hidden.load(Ordering::Relaxed))
        .additional_browser_args(super::BROWSER_ARGS)
        .build()?;
    zoom(&window, app.state::<SettingsState>().get().scale)?;
    snap::attach(&window)?;
    if overlay::status(app)?.click_through {
        window.set_ignore_cursor_events(true)?;
    }
    Ok(())
}

/// Applies the size setting to a pinned panel. The page fits the window to it.
pub fn zoom(window: &WebviewWindow, scale: f64) -> Result<()> {
    window.set_zoom(scale)?;
    window.set_min_size(Some(LogicalSize::new(MIN_SIZE.0 * scale, MIN_SIZE.1 * scale)))?;
    Ok(())
}

/// Tells a pinned panel when the player resizes it, so it keeps that size.
pub fn resized(window: &Window, size: PhysicalSize<u32>) {
    if !window.label().starts_with(PREFIX) || !snap::sizing(window) {
        return;
    }
    if let Ok(scale) = window.scale_factor() {
        let size: LogicalSize<f64> = size.to_logical(scale);
        let _ = window.emit_to(window.label(), RESIZED_EVENT, (size.width, size.height));
    }
}

/// In a row to the right of the overlay, in pin order. All start the same width, so a row never
/// overlaps. A position the user dragged it to is restored over this once the window is ready.
fn default_position(app: &AppHandle, slot: usize) -> (f64, f64) {
    let anchor = app.get_webview_window(OVERLAY).and_then(|w| {
        let scale = w.scale_factor().ok()?;
        let position: LogicalPosition<f64> = w.outer_position().ok()?.to_logical(scale);
        let size: LogicalSize<f64> = w.outer_size().ok()?.to_logical(scale);
        Some((position.x + size.width + GAP, position.y))
    });
    let (x, y) = anchor.unwrap_or((400.0, 120.0));
    (x + slot as f64 * (WIDTH + GAP), y)
}

fn changed(app: &AppHandle) -> Result<PinStatus> {
    let status = app.state::<PinState>().status();
    app.emit(PINS_EVENT, &status)?;
    Ok(status)
}

#[tauri::command]
pub fn pin_status(state: State<PinState>) -> PinStatus {
    state.status()
}

// Windows are created from async commands: a synchronous one would block the event loop the
// new window needs.
#[tauri::command]
pub async fn pin_panel(app: AppHandle, panel: Panel) -> Result<PinStatus> {
    let state = app.state::<PinState>();
    let slot = {
        let mut pinned = state.pinned.lock().expect("pins lock");
        if !pinned.contains(&panel) {
            pinned.push(panel);
            // Keep a stable order so restored panels stack the same way.
            pinned.sort_by_key(|p| Panel::ALL.iter().position(|a| a == p));
            state.save(&pinned)?;
        }
        pinned.iter().position(|p| *p == panel).unwrap_or(0)
    };
    state.hidden.store(false, Ordering::Relaxed);
    for window in windows(&app) {
        window.show()?;
    }
    open(&app, panel, slot)?;
    changed(&app)
}

#[tauri::command]
pub async fn unpin_panel(app: AppHandle, panel: Panel) -> Result<PinStatus> {
    let state = app.state::<PinState>();
    {
        let mut pinned = state.pinned.lock().expect("pins lock");
        pinned.retain(|p| *p != panel);
        state.save(&pinned)?;
    }
    if let Some(window) = app.get_webview_window(&panel.label()) {
        window.destroy()?;
    }
    changed(&app)
}

/// Hides every pinned panel, or shows them again, without unpinning any.
pub fn toggle_visibility(app: &AppHandle) -> Result<()> {
    let state = app.state::<PinState>();
    let hidden = !state.hidden.load(Ordering::Relaxed);
    state.hidden.store(hidden, Ordering::Relaxed);
    for window in windows(app) {
        if hidden { window.hide()? } else { window.show()? }
    }
    changed(app)?;
    Ok(())
}

#[tauri::command]
pub fn toggle_pins(app: AppHandle) -> Result<()> {
    toggle_visibility(&app)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_stats_and_build_pins_open_the_character_panel_once() {
        assert_eq!(parse(r#"["stats","map","build"]"#), vec![Panel::Character, Panel::Map]);
        assert_eq!(parse(r#"["build"]"#), vec![Panel::Character]);
        assert_eq!(parse(r#"["character","item"]"#), vec![Panel::Character, Panel::Item]);
        assert_eq!(parse("not json"), Vec::<Panel>::new());
    }

    #[test]
    fn panels_are_saved_under_their_new_id() {
        assert_eq!(serde_json::to_string(&[Panel::Character]).unwrap(), r#"["character"]"#);
    }
}
