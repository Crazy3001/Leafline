//! Display settings for the overlay and pinned panels, and the Settings window.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};

use super::overlay::{self, OVERLAY};
use super::pins;
use super::shortcuts;
use crate::build::Rules;
use crate::capture::WindowChoice;
use crate::error::{Error, Result};
use crate::mapdata::TravelChoice;

const FILE_NAME: &str = "settings.json";
pub const SETTINGS: &str = "settings";
pub const SETUP: &str = "setup";

/// Emitted to every window when the settings change.
pub const SETTINGS_EVENT: &str = "settings";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// How opaque the overlay and pinned panels are, 0.4 to 1.
    pub opacity: f64,
    /// How large their text and controls are drawn, 0.6 to 2.
    pub scale: f64,
    /// Keep the overlay and pinned panels above the game.
    pub always_on_top: bool,
    /// Clicking the overlay or a pinned panel leaves the game focused, so play goes on without
    /// clicking back into it. Typing in a box still takes focus while the box is in use.
    pub keep_game_focus: bool,
    pub compact_shape: CompactShape,
    pub hotkeys: HotkeyChoice,
    /// Taxis and return scrolls in routes. Off until the player turns it on.
    pub travel: TravelChoice,
    pub live: LiveChoice,
    /// How points are earned, as the build planner counts them. The player can correct these.
    pub build_rules: Rules,
    /// Where saved game pictures go. None means the app's local folder, which never syncs.
    pub frames_folder: Option<String>,
    /// The first-launch setup has been finished. Until then it opens at every start.
    pub setup_done: bool,
    /// Ask GitHub for the latest version number at startup. Off until the player turns it on, in
    /// setup or Settings; sends nothing about the game or the player (see `update.rs`).
    pub update_check: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            opacity: 1.0,
            scale: 1.0,
            always_on_top: true,
            keep_game_focus: true,
            compact_shape: CompactShape::default(),
            hotkeys: HotkeyChoice::default(),
            travel: TravelChoice::default(),
            live: LiveChoice::default(),
            build_rules: Rules::default(),
            frames_folder: None,
            setup_done: false,
            update_check: false,
        }
    }
}

/// How the compact HUD was laid out. Since 2026-11-05 it has one layout and this is no longer
/// offered; it stays so settings files that name a shape still load.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompactShape {
    /// A narrow card, with the quest counters under the numbers.
    #[default]
    Card,
    /// Longer sideways, with a row of four numbers.
    Wide,
    /// One thin strip.
    Bar,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HotkeyChoice {
    pub enabled: bool,
    /// Keys the player changed, by action. Empty turns that hotkey off.
    pub keys: BTreeMap<String, String>,
}

impl Default for HotkeyChoice {
    fn default() -> Self {
        Self { enabled: true, keys: BTreeMap::new() }
    }
}

/// Live reading of the game window. On from the start (decided 2026-11-04); setup says in one line
/// that Leafline is unofficial, and the player can turn it off.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LiveChoice {
    pub enabled: bool,
    /// Seconds between readings, 1 to 10.
    pub interval: f64,
    /// Seconds without EXP before the clock pauses, 5 to 300.
    pub pause_after: f64,
    /// The window the player picked, for when Leafline can't tell which one is the game.
    pub window: Option<WindowChoice>,
}

impl Default for LiveChoice {
    fn default() -> Self {
        Self { enabled: true, interval: 2.0, pause_after: 15.0, window: None }
    }
}

impl Settings {
    fn clamped(self) -> Self {
        let finite = |v: f64, fallback: f64| if v.is_finite() { v } else { fallback };
        Self {
            opacity: finite(self.opacity, 1.0).clamp(0.4, 1.0),
            scale: finite(self.scale, 1.0).clamp(0.6, 2.0),
            live: LiveChoice {
                interval: finite(self.live.interval, 2.0).clamp(1.0, 10.0),
                pause_after: finite(self.live.pause_after, 15.0).clamp(5.0, 300.0),
                ..self.live
            },
            build_rules: self.build_rules.clamped(),
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
        self.current.lock().expect("settings lock").clone()
    }
}

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
/// the pinned panels, the desk view, Settings, setup, the build planner and the training log.
fn game_side_windows(app: &AppHandle) -> Vec<tauri::WebviewWindow> {
    app.get_webview_window(OVERLAY)
        .into_iter()
        .chain(pins::windows(app))
        .chain(
            [super::desk::DESK, SETTINGS, SETUP, super::builds::BUILD, super::records::TRAINING]
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
    matches!(label, OVERLAY | SETTINGS | SETUP | super::desk::DESK | super::builds::BUILD | super::records::TRAINING)
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
    let settings = settings.clamped();
    shortcuts::check(&settings.hotkeys)?;
    let old = std::mem::replace(&mut *state.current.lock().expect("settings lock"), settings.clone());
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
        .inner_size(600.0, 640.0)
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
