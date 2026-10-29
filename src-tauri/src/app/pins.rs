//! Panels pinned out of the overlay into small windows of their own. They stay on screen while
//! the overlay is hidden, follow its click-through lock, and come back at the next launch.

use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};
use tauri::{
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};

use super::overlay::{self, OVERLAY};
use crate::error::{Error, Result};

const FILE_NAME: &str = "pins.json";

/// Emitted to every window when panels are pinned, unpinned, shown or hidden.
pub const PINS_EVENT: &str = "pins";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Panel {
    Stats,
    Map,
    Quests,
    Item,
}

impl Panel {
    const ALL: [Panel; 4] = [Panel::Stats, Panel::Map, Panel::Quests, Panel::Item];

    fn id(self) -> &'static str {
        match self {
            Panel::Stats => "stats",
            Panel::Map => "map",
            Panel::Quests => "quests",
            Panel::Item => "item",
        }
    }

    fn title(self) -> &'static str {
        match self {
            Panel::Stats => "Stats",
            Panel::Map => "Map",
            Panel::Quests => "Quests",
            Panel::Item => "Item",
        }
    }

    fn label(self) -> String {
        format!("{PREFIX}{}", self.id())
    }

    /// Logical width. The page sets the height to fit what it shows.
    fn width(self) -> f64 {
        match self {
            Panel::Stats => 264.0,
            Panel::Map | Panel::Item => 300.0,
            Panel::Quests => 280.0,
        }
    }
}

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
        let pinned =
            std::fs::read_to_string(&path).ok().and_then(|json| serde_json::from_str(&json).ok()).unwrap_or_default();
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
        .inner_size(panel.width(), 120.0)
        .position(x, y)
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .focused(false)
        .visible(!app.state::<PinState>().hidden.load(Ordering::Relaxed))
        .build()?;
    if overlay::status(app)?.click_through {
        window.set_ignore_cursor_events(true)?;
    }
    Ok(())
}

/// In a row to the right of the overlay, in pin order. Widths are fixed and heights are not,
/// so a row never overlaps. A position the user dragged it to is restored over this once the
/// window is ready.
fn default_position(app: &AppHandle, slot: usize) -> (f64, f64) {
    let anchor = app.get_webview_window(OVERLAY).and_then(|w| {
        let scale = w.scale_factor().ok()?;
        let position: LogicalPosition<f64> = w.outer_position().ok()?.to_logical(scale);
        let size: LogicalSize<f64> = w.outer_size().ok()?.to_logical(scale);
        Some((position.x + size.width + GAP, position.y))
    });
    let (x, y) = anchor.unwrap_or((400.0, 120.0));
    let pinned = app.state::<PinState>().status().pinned;
    let before: f64 = pinned.iter().take(slot).map(|p| p.width() + GAP).sum();
    (x + before, y)
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
