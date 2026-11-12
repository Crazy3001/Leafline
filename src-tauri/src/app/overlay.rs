//! Overlay window behaviour: visibility, click-through and the compact HUD.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};
use tauri::menu::CheckMenuItem;
use tauri::{AppHandle, Emitter, LogicalSize, Manager, WebviewWindow, Wry};

use super::pins;
use crate::error::{Error, Result};

pub const OVERLAY: &str = "overlay";

/// Emitted to all windows whenever the overlay's status changes.
pub const STATUS_EVENT: &str = "overlay-status";

/// Whether the overlay is compact, with the full size to go back to, and whether it is locked,
/// kept across restarts.
const MODE_FILE: &str = "overlay.json";
/// The full overlay's size and smallest size, as in tauri.conf.json.
/// The Train tab's four ranking tabs and its tables need about this much width, so the figures at
/// the right of each row aren't cut off.
const FULL_SIZE: (f64, f64) = (420.0, 660.0);
const MIN_SIZE: (f64, f64) = (380.0, 120.0);

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
        // The full size it goes back to is widened too, if it was saved narrower than it needs.
        *state.full_size.lock().expect("overlay lock") =
            saved.full_size.map(|(width, height)| if width < MIN_SIZE.0 { (FULL_SIZE.0, height.max(FULL_SIZE.1)) } else { (width, height) });
        window(app, OVERLAY)?.set_min_size(None::<LogicalSize<f64>>)?;
    } else {
        // An overlay remembered narrower than the Train tab now needs (the old 340x560 default) is
        // given the new default size once, since a restored size ignores the new minimum. Dragged narrower later, it stops at the minimum.
        let overlay = window(app, OVERLAY)?;
        let size: LogicalSize<f64> = overlay.inner_size()?.to_logical(overlay.scale_factor()?);
        if size.width < MIN_SIZE.0 {
            overlay.set_size(LogicalSize::new(FULL_SIZE.0, size.height.max(FULL_SIZE.1)))?;
        }
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

/// Brings up the full overlay, for something the compact HUD has no room for.
pub fn show_full(app: &AppHandle) -> Result<()> {
    window(app, OVERLAY)?.show()?;
    set_compact(app, false)
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

/// Shows the overlay, as when Leafline is started again while it runs.
pub fn show(app: &AppHandle) -> Result<()> {
    window(app, OVERLAY)?.show()?;
    broadcast(app)
}

/// Below this many logical pixels of screen height, the overlay starts smaller.
const SMALL_SCREEN_HEIGHT: f64 = 900.0;
/// The overlay's first size on such a screen: narrower, and no taller than this share of it.
const SMALL_SCREEN_WIDTH: f64 = 380.0;
const SMALL_SCREEN_SHARE: f64 = 0.5;

/// On a new install, starts the overlay smaller on a short screen, such as a 1366x768 laptop,
/// where its usual 420x660 would cover most of a game window's height. Only before setup is
/// done, so a size the player chose stays.
pub fn fit_first_launch(app: &AppHandle) -> Result<()> {
    let overlay = window(app, OVERLAY)?;
    let Some(monitor) = overlay.current_monitor()? else { return Ok(()) };
    let scale = monitor.scale_factor();
    let area = monitor.work_area();
    let height = f64::from(area.size.height) / scale;
    if height >= SMALL_SCREEN_HEIGHT {
        return Ok(());
    }
    let size: LogicalSize<f64> = overlay.inner_size()?.to_logical(scale);
    let fitted = LogicalSize::new(
        size.width.min(SMALL_SCREEN_WIDTH).max(MIN_SIZE.0),
        size.height.min(height * SMALL_SCREEN_SHARE).max(MIN_SIZE.1),
    );
    overlay.set_size(fitted)?;
    Ok(())
}

fn broadcast(app: &AppHandle) -> Result<()> {
    app.emit(STATUS_EVENT, status(app)?)?;
    Ok(())
}

fn window(app: &AppHandle, label: &str) -> Result<WebviewWindow> {
    app.get_webview_window(label).ok_or_else(|| Error::Invalid(format!("the {label} window is missing")))
}
