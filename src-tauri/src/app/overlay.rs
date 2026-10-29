//! Overlay window behaviour: visibility, click-through, and the workbench window.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;
use tauri::menu::CheckMenuItem;
use tauri::{AppHandle, Emitter, Manager, PhysicalSize, WebviewWindow, Wry};

use crate::error::{Error, Result};

pub const OVERLAY: &str = "overlay";
pub const WORKBENCH: &str = "workbench";

/// Emitted to all windows whenever the overlay's status changes.
pub const STATUS_EVENT: &str = "overlay-status";

#[derive(Default)]
pub struct OverlayState {
    click_through: AtomicBool,
    /// The tray's check item, kept in sync when click-through is toggled by hotkey.
    pub click_through_item: OnceLock<CheckMenuItem<Wry>>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayStatus {
    pub visible: bool,
    pub click_through: bool,
}

pub fn status(app: &AppHandle) -> Result<OverlayStatus> {
    Ok(OverlayStatus {
        visible: window(app, OVERLAY)?.is_visible()?,
        click_through: app.state::<OverlayState>().click_through.load(Ordering::Relaxed),
    })
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

/// When on, mouse input passes through the overlay to the game underneath. It can only be
/// turned off again by hotkey or from the tray.
pub fn set_click_through(app: &AppHandle, enabled: bool) -> Result<()> {
    window(app, OVERLAY)?.set_ignore_cursor_events(enabled)?;
    let state = app.state::<OverlayState>();
    state.click_through.store(enabled, Ordering::Relaxed);
    if let Some(item) = state.click_through_item.get() {
        item.set_checked(enabled)?;
    }
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
