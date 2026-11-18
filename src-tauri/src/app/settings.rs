//! Display settings for the overlay and pinned panels, and the Settings window.

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};

use super::overlay::{self, OVERLAY};
use super::pins;
use super::shortcuts;
use super::host::AppHost;
use crate::error::Result;
pub use leafline_core::service::settings::*;

pub const SETTINGS: &str = "settings";
pub const SETUP: &str = "setup";

/// Draws the overlay at the saved scale; called once at startup.
pub fn apply(app: &AppHandle) -> Result<()> {
    let settings = app.state::<SettingsState>().get();
    if let Some(overlay) = app.get_webview_window(OVERLAY) {
        overlay.set_zoom(settings.scale)?;
        overlay.set_always_on_top(settings.always_on_top)?;
        overlay.set_focusable(!settings.keep_game_focus)?;
    }
    // A bad hand-edited hotkey must not stop the app from starting.
    if let Err(err) = shortcuts::apply(app, &settings.hotkeys) {
        shortcuts::log_error(&err);
    }
    Ok(())
}

/// The windows that leave the game focused when clicked, while the setting is on: the overlay,
/// the pinned panels, the desk view (which holds the training log and the build planner), Settings and setup.
fn game_side_windows(app: &AppHandle) -> Vec<tauri::WebviewWindow> {
    app.get_webview_window(OVERLAY)
        .into_iter()
        .chain(pins::windows(app))
        .chain(
            [super::desk::DESK, SETTINGS, SETUP]
                .into_iter()
                .filter_map(|label| app.get_webview_window(label)),
        )
        .collect()
}

/// Whether Leafline's windows leave the game focused when opened or clicked.
pub fn keeps_game_focus(app: &AppHandle) -> bool {
    app.state::<SettingsState>().get().keep_game_focus
}

/// Brings an open window forward: shown and unminimized, and focused too unless the game keeps
/// the focus, in which case it is only put above the game.
pub fn bring_forward(app: &AppHandle, window: &tauri::WebviewWindow) -> Result<()> {
    window.show()?;
    window.unminimize()?;
    if keeps_game_focus(app) {
        raise(window);
    } else {
        window.set_focus()?;
    }
    Ok(())
}

/// Puts a window above the others, the game included, without taking focus. A window opened
/// without focus otherwise lands under the game, which keeps the front. Done on the main thread,
/// which owns the window.
pub fn raise(window: &tauri::WebviewWindow) {
    let Ok(hwnd) = window.hwnd() else { return };
    let hwnd = hwnd.0 as isize;
    let _ = window.run_on_main_thread(move || foreground::raise(hwnd));
}

/// For a new window's `on_page_load`: once its page has loaded, it goes above the game. Moving it
/// in the stack any sooner, while WebView2 was still starting in it, left the window black.
pub fn raise_when_loaded(window: tauri::WebviewWindow, payload: tauri::webview::PageLoadPayload<'_>) {
    if payload.event() == tauri::webview::PageLoadEvent::Finished && keeps_game_focus(window.app_handle()) {
        raise(&window);
    }
}

/// A box is being typed in, in some Leafline window: that window has focus on purpose.
static TYPING: AtomicBool = AtomicBool::new(false);

/// Leafline's windows that leave the game focused, by label.
fn game_side(label: &str) -> bool {
    matches!(label, OVERLAY | SETTINGS | SETUP | super::desk::DESK)
        || label.starts_with(pins::PREFIX)
}

/// One of Leafline's windows got focus. Windows gives it focus even when it asks not to take
/// any, for one: clicking its taskbar button. Unless a box in it is being typed in, the focus goes
/// straight back to the game and the window stays in front of it.
pub fn focused(window: &tauri::Window) {
    let app = window.app_handle();
    if !game_side(window.label()) || !keeps_game_focus(app) || TYPING.load(Ordering::Relaxed) {
        return;
    }
    let choice = app.state::<SettingsState>().get().live.window;
    let Ok(Some(game)) = crate::capture::find(choice.as_ref()) else { return };
    let Ok(hwnd) = window.hwnd() else { return };
    foreground::hand_to(game.id);
    foreground::raise(hwnd.0 as isize);
}

/// While the player types in a box on the overlay or a pinned panel, that window takes focus so
/// the keys reach it. When they leave the box, the window that was in front before (the game,
/// usually) is put back in front, so play goes on without clicking it. That is a window-manager
/// call only: nothing is sent to the game.
#[tauri::command]
pub fn set_typing(app: AppHandle, window: tauri::WebviewWindow, typing: bool) -> Result<()> {
    if !app.state::<SettingsState>().get().keep_game_focus {
        return Ok(());
    }
    TYPING.store(typing, Ordering::Relaxed);
    if typing {
        foreground::remember();
    }
    window.set_focusable(typing)?;
    if typing {
        window.set_focus()?;
    } else {
        foreground::give_back();
        // The game comes back in front; the window stays above it, as it was.
        raise(&window);
    }
    Ok(())
}

/// The window in front before Leafline took focus for typing.
#[cfg(windows)]
mod foreground {
    use std::sync::atomic::{AtomicIsize, Ordering};

    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        GWL_EXSTYLE, GetForegroundWindow, GetWindowLongPtrW, GetWindowThreadProcessId, HWND_NOTOPMOST, HWND_TOPMOST,
        IsWindow, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetForegroundWindow, SetWindowPos, WS_EX_TOPMOST,
    };

    static BEFORE: AtomicIsize = AtomicIsize::new(0);

    /// Puts a window at the top of the ordinary ones without activating it: briefly topmost, then
    /// not, which leaves it above every other ordinary window. One that is always on top already
    /// stays as it is.
    pub fn raise(hwnd: isize) {
        let hwnd = HWND(hwnd as *mut _);
        let topmost = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } as u32 & WS_EX_TOPMOST.0 != 0;
        if topmost {
            return;
        }
        let flags = SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE;
        unsafe {
            let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, flags);
            let _ = SetWindowPos(hwnd, Some(HWND_NOTOPMOST), 0, 0, 0, 0, flags);
        }
    }

    /// Gives the game focus back, while Leafline has the front (only then may it pass it on).
    pub fn hand_to(game: isize) {
        let game = HWND(game as *mut _);
        if ours(unsafe { GetForegroundWindow() }) && unsafe { IsWindow(Some(game)) }.as_bool() {
            let _ = unsafe { SetForegroundWindow(game) };
        }
    }

    fn ours(hwnd: HWND) -> bool {
        let mut pid = 0;
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
        pid == std::process::id()
    }

    /// Notes the window in front, unless it is one of Leafline's (moving from one box to another
    /// across windows keeps the first one noted).
    pub fn remember() {
        let hwnd = unsafe { GetForegroundWindow() };
        if !hwnd.is_invalid() && !ours(hwnd) {
            BEFORE.store(hwnd.0 as isize, Ordering::Relaxed);
        }
    }

    /// Puts the noted window back in front, if Leafline still has the front.
    pub fn give_back() {
        let before = HWND(BEFORE.load(Ordering::Relaxed) as *mut _);
        let now = unsafe { GetForegroundWindow() };
        if before.is_invalid() || !ours(now) || !unsafe { IsWindow(Some(before)) }.as_bool() {
            return;
        }
        let _ = unsafe { SetForegroundWindow(before) };
    }
}

#[cfg(not(windows))]
mod foreground {
    pub fn remember() {}
    pub fn give_back() {}
    pub fn raise(_hwnd: isize) {}
    pub fn hand_to(_game: isize) {}
}

#[tauri::command]
pub fn get_settings(state: State<SettingsState>) -> Settings {
    state.get()
}

#[tauri::command]
pub fn update_settings(app: AppHandle, state: State<SettingsState>, settings: Settings) -> Result<Settings> {
    shortcuts::check(&settings.clone().clamped().hotkeys)?;
    let (old, settings) = state.replace(AppHost::files_of(&app), settings)?;

    if old.scale != settings.scale {
        // The overlay grows with its contents so the same amount stays in view. Pinned panels
        // fit themselves to their contents once redrawn.
        if let Some(overlay) = app.get_webview_window(OVERLAY) {
            overlay.set_zoom(settings.scale)?;
            overlay::rescale(&app, settings.scale / old.scale)?;
        }
        for window in pins::windows(&app) {
            pins::zoom(&window, settings.scale)?;
        }
    }
    if old.always_on_top != settings.always_on_top {
        if let Some(overlay) = app.get_webview_window(OVERLAY) {
            overlay.set_always_on_top(settings.always_on_top)?;
        }
        for window in pins::windows(&app) {
            window.set_always_on_top(settings.always_on_top)?;
        }
    }
    if old.keep_game_focus != settings.keep_game_focus {
        for window in game_side_windows(&app) {
            window.set_focusable(!settings.keep_game_focus)?;
        }
    }
    if old.hotkeys != settings.hotkeys {
        shortcuts::apply(&app, &settings.hotkeys)?;
    }
    if old.live.enabled && !settings.live.enabled {
        super::live::stop(&app);
    }
    if old.update_check != settings.update_check {
        if settings.update_check { super::update::start(&app) } else { super::update::turn_off(&app) }
    }
    app.emit(SETTINGS_EVENT, &settings)?;
    Ok(settings)
}

/// Opens the Settings window, or brings it forward.
#[tauri::command]
pub async fn show_settings(app: AppHandle) -> Result<()> {
    open(&app)
}

/// Opens the first-launch setup, or brings it forward.
pub fn open_setup(app: &AppHandle) -> Result<()> {
    if let Some(window) = app.get_webview_window(SETUP) {
        return bring_forward(app, &window);
    }
    let keep = keeps_game_focus(app);
    WebviewWindowBuilder::new(app, SETUP, WebviewUrl::App("setup".into()))
        .title("Leafline — Setup")
        // Wide enough for the steps' rail beside the step; narrower, they run across the top.
        .inner_size(820.0, 640.0)
        .min_inner_size(520.0, 520.0)
        .center()
        .focusable(!keep)
        .focused(!keep)
        .background_color(tauri::window::Color(20, 20, 18, 255))
        .additional_browser_args(super::BROWSER_ARGS)
        .on_page_load(raise_when_loaded)
        .build()?;
    Ok(())
}

/// Opens the first-launch setup again, from Settings.
#[tauri::command]
pub async fn show_setup(app: AppHandle) -> Result<()> {
    open_setup(&app)
}

/// Finishes setup: closes its window and starts live reading if it is on.
#[tauri::command]
pub fn finish_setup(app: AppHandle) -> Result<()> {
    if let Some(window) = app.get_webview_window(SETUP) {
        window.close()?;
    }
    if app.state::<SettingsState>().get().live.enabled && !super::live::running(&app) {
        super::live::start(&app)?;
    }
    Ok(())
}

pub fn open(app: &AppHandle) -> Result<()> {
    if let Some(window) = app.get_webview_window(SETTINGS) {
        return bring_forward(app, &window);
    }
    let keep = keeps_game_focus(app);
    WebviewWindowBuilder::new(app, SETTINGS, WebviewUrl::App("settings".into()))
        .title("Leafline — Settings")
        .inner_size(640.0, 720.0)
        .min_inner_size(520.0, 480.0)
        .center()
        .focusable(!keep)
        .focused(!keep)
        .background_color(tauri::window::Color(13, 16, 23, 255))
        .additional_browser_args(super::BROWSER_ARGS)
        .on_page_load(raise_when_loaded)
        .build()?;
    Ok(())
}
