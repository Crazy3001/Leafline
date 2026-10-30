//! Overlay window behaviour: visibility, click-through, the compact HUD, and the workbench
//! window.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};
use tauri::menu::CheckMenuItem;
use tauri::{AppHandle, Emitter, LogicalSize, Manager, PhysicalSize, WebviewWindow, Wry};

use super::pins;
use crate::error::{Error, Result};

pub const OVERLAY: &str = "overlay";
pub const WORKBENCH: &str = "workbench";

/// Emitted to all windows whenever the overlay's status changes.
pub const STATUS_EVENT: &str = "overlay-status";

/// Whether the overlay is compact, with the full size to go back to, and whether it is locked,
/// kept across restarts.
const MODE_FILE: &str = "overlay.json";
/// The full overlay's size and smallest size, as in tauri.conf.json.
const FULL_SIZE: (f64, f64) = (340.0, 560.0);
const MIN_SIZE: (f64, f64) = (280.0, 120.0);

#[derive(Default)]
pub struct OverlayState {
    click_through: AtomicBool,
    compact: AtomicBool,
    /// The full overlay's logical size while it is compact.
    full_size: Mutex<Option<(f64, f64)>>,
    /// The tray's check item, kept in sync when click-through is toggled by hotkey.
    pub click_through_item: OnceLock<CheckMenuItem<Wry>>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayStatus {
    pub visible: bool,
    pub click_through: bool,
    /// Showing only EXP per hour and the quest counters, in a small window.
    pub compact: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct SavedMode {
    compact: bool,
    full_size: Option<(f64, f64)>,
    click_through: bool,
}

impl OverlayState {
    /// Whether clicks pass through the overlay and pinned panels.
    pub fn click_through(&self) -> bool {
        self.click_through.load(Ordering::Relaxed)
    }
}

pub fn status(app: &AppHandle) -> Result<OverlayStatus> {
    let state = app.state::<OverlayState>();
    Ok(OverlayStatus {
        visible: window(app, OVERLAY)?.is_visible()?,
        click_through: state.click_through.load(Ordering::Relaxed),
        compact: state.compact.load(Ordering::Relaxed),
    })
}

/// Starts compact or locked if the overlay was when the app last closed. Called once at startup,
/// before pinned panels reopen, since they take the lock from here.
pub fn restore_mode(app: &AppHandle) -> Result<()> {
    let path = app.path().app_data_dir()?.join(MODE_FILE);
    let saved: SavedMode =
        std::fs::read_to_string(path).ok().and_then(|json| serde_json::from_str(&json).ok()).unwrap_or_default();
    if saved.click_through {
        app.state::<OverlayState>().click_through.store(true, Ordering::Relaxed);
        window(app, OVERLAY)?.set_ignore_cursor_events(true)?;
    }
    if saved.compact {
        let state = app.state::<OverlayState>();
        state.compact.store(true, Ordering::Relaxed);
        *state.full_size.lock().expect("overlay lock") = saved.full_size;
        window(app, OVERLAY)?.set_min_size(None::<LogicalSize<f64>>)?;
    }
    Ok(())
}

fn save_mode(app: &AppHandle) -> Result<()> {
    let state = app.state::<OverlayState>();
    let saved = SavedMode {
        compact: state.compact.load(Ordering::Relaxed),
        full_size: *state.full_size.lock().expect("overlay lock"),
        click_through: state.click_through.load(Ordering::Relaxed),
    };
    let path = app.path().app_data_dir()?.join(MODE_FILE);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let json = serde_json::to_string_pretty(&saved).map_err(|err| Error::Invalid(err.to_string()))?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Switches between the full overlay and the compact HUD. The compact view sizes the window to
/// fit what it shows; going back restores the full overlay's size.
pub fn set_compact(app: &AppHandle, enabled: bool) -> Result<()> {
    let state = app.state::<OverlayState>();
    if state.compact.swap(enabled, Ordering::Relaxed) != enabled {
        let overlay = window(app, OVERLAY)?;
        if enabled {
            let size: LogicalSize<f64> = overlay.inner_size()?.to_logical(overlay.scale_factor()?);
            *state.full_size.lock().expect("overlay lock") = Some((size.width, size.height));
            overlay.set_min_size(None::<LogicalSize<f64>>)?;
        } else {
            let (width, height) = state.full_size.lock().expect("overlay lock").take().unwrap_or(FULL_SIZE);
            overlay.set_min_size(Some(LogicalSize::new(MIN_SIZE.0, MIN_SIZE.1)))?;
            overlay.set_size(LogicalSize::new(width, height))?;
        }
        save_mode(app)?;
    }
    broadcast(app)
}

/// The hotkey: toggles the compact HUD, showing the overlay if it was hidden.
pub fn toggle_compact(app: &AppHandle) -> Result<()> {
    let overlay = window(app, OVERLAY)?;
    if !overlay.is_visible()? {
        overlay.show()?;
        if app.state::<OverlayState>().compact.load(Ordering::Relaxed) {
            return broadcast(app);
        }
    }
    let enabled = app.state::<OverlayState>().compact.load(Ordering::Relaxed);
    set_compact(app, !enabled)
}

/// Grows or shrinks the overlay with a new size setting, so the same amount stays in view. A
/// compact overlay fits itself; its saved full size is scaled instead.
pub fn rescale(app: &AppHandle, ratio: f64) -> Result<()> {
    let state = app.state::<OverlayState>();
    if state.compact.load(Ordering::Relaxed) {
        if let Some((width, height)) = state.full_size.lock().expect("overlay lock").as_mut() {
            *width *= ratio;
            *height *= ratio;
        }
        return save_mode(app);
    }
    let overlay = window(app, OVERLAY)?;
    let size: LogicalSize<f64> = overlay.inner_size()?.to_logical(overlay.scale_factor()?);
    overlay.set_size(LogicalSize::new(size.width * ratio, size.height * ratio))?;
    Ok(())
}

pub fn toggle_visibility(app: &AppHandle) -> Result<()> {
    let overlay = window(app, OVERLAY)?;
    if overlay.is_visible()? {
        overlay.hide()?
    } else {
        overlay.show()?
    }
    broadcast(app)
}

pub fn hide(app: &AppHandle) -> Result<()> {
    window(app, OVERLAY)?.hide()?;
    broadcast(app)
}

/// When on, mouse input passes through the overlay and pinned panels to the game underneath.
/// It can only be turned off again by hotkey or from the tray.
pub fn set_click_through(app: &AppHandle, enabled: bool) -> Result<()> {
    window(app, OVERLAY)?.set_ignore_cursor_events(enabled)?;
    for pinned in pins::windows(app) {
        pinned.set_ignore_cursor_events(enabled)?;
    }
    let state = app.state::<OverlayState>();
    state.click_through.store(enabled, Ordering::Relaxed);
    if let Some(item) = state.click_through_item.get() {
        item.set_checked(enabled)?;
    }
    save_mode(app)?;
    broadcast(app)
}

pub fn toggle_click_through(app: &AppHandle) -> Result<()> {
    let enabled = app.state::<OverlayState>().click_through.load(Ordering::Relaxed);
    set_click_through(app, !enabled)
}

pub fn show_workbench(app: &AppHandle) -> Result<()> {
    let workbench = window(app, WORKBENCH)?;
    workbench.show()?;
    workbench.unminimize()?;
    workbench.set_focus()?;
    Ok(())
}

/// Shrinks the workbench to fit its monitor's work area when the default size is too big
/// (small or high-DPI screens). A size the user chose is restored before this runs.
pub fn fit_workbench(app: &AppHandle) -> Result<()> {
    let workbench = window(app, WORKBENCH)?;
    let Some(monitor) = workbench.current_monitor()? else { return Ok(()) };
    let area = monitor.work_area().size;
    let size = workbench.outer_size()?;
    if size.width > area.width || size.height > area.height {
        let fitted = PhysicalSize::new(size.width.min(area.width * 9 / 10), size.height.min(area.height * 9 / 10));
        workbench.set_size(fitted)?;
        workbench.center()?;
    }
    Ok(())
}

fn broadcast(app: &AppHandle) -> Result<()> {
    app.emit(STATUS_EVENT, status(app)?)?;
    Ok(())
}

fn window(app: &AppHandle, label: &str) -> Result<WebviewWindow> {
    app.get_webview_window(label).ok_or_else(|| Error::Invalid(format!("the {label} window is missing")))
}
