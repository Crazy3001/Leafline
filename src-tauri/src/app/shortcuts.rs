//! Global hotkeys. They work while the game has focus, without sending it any input.
//!
//! None are bound until the player picks them at setup or in Settings. Ctrl and Alt reach the
//! game while held (attack and jump by default), so the suggestions use neither.
//!
//! From the game's Key Bindings window on a fresh character (2026-11-04): Pause/Break, PrtSc,
//! Tab, Caps Lock, Backspace, Enter and Win can't be bound in game, and it has no numpad. F1-F7
//! are emotes, F9 cash shop, F10 shortcut, F11 picture mode and Scroll Lock the screenshot by
//! default; F8 and F12 start empty. Windows keeps F12 for its debugger and refuses it as a
//! hotkey on every PC. On one test PC (2026-11-04) plain F8 never arrived, with nothing marked
//! as taken, while Win+F8 and Num 5 did: some program grabs keys below RegisterHotKey, where
//! Leafline can't see it. So the suggestions add Win, which the game can't bind, and setup has
//! the player press each key to prove it arrives.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::plugin::TauriPlugin;
use tauri::{AppHandle, Emitter, Manager, Wry};
use tauri_plugin_global_shortcut::{Builder, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use super::settings::HotkeyChoice;
use super::{live, overlay, pins, report};
use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Action {
    ToggleOverlay,
    ToggleClickThrough,
    TogglePins,
    ToggleCompact,
    ToggleLive,
    ReadItem,
    ReadCharacter,
    PauseTracking,
    NewSession,
    ToggleLogging,
    SaveFrame,
    ShowWorkbench,
}

impl Action {
    pub const ALL: [Action; 12] = [
        Action::ToggleOverlay,
        Action::ToggleClickThrough,
        Action::TogglePins,
        Action::ToggleCompact,
        Action::ToggleLive,
        Action::ReadItem,
        Action::ReadCharacter,
        Action::PauseTracking,
        Action::NewSession,
        Action::ToggleLogging,
        Action::SaveFrame,
        Action::ShowWorkbench,
    ];

    /// The key offered at setup, or "" for none. Few keys are free in game by default.
    pub fn suggested_keys(self) -> &'static str {
        match self {
            Action::ReadItem => "Super+F8",
            Action::ToggleOverlay => "Super+F9",
            Action::ToggleClickThrough => "Pause",
            _ => "",
        }
    }

    /// Whether setup asks about this action. The rest are in Settings.
    pub fn in_setup(self) -> bool {
        matches!(
            self,
            Action::ReadItem
                | Action::ToggleOverlay
                | Action::ToggleClickThrough
                | Action::ToggleCompact
                | Action::ReadCharacter
                | Action::SaveFrame
        )
    }

    pub fn label(self) -> &'static str {
        match self {
            Action::ToggleOverlay => "Show or hide the overlay",
            Action::ToggleClickThrough => "Lock the overlay (click-through)",
            Action::TogglePins => "Show or hide pinned panels",
            Action::ToggleCompact => "Compact HUD",
            Action::ToggleLive => "Start or stop live reading",
            Action::ReadItem => "Read the item you're hovering",
            Action::ReadCharacter => "Read your stat and skill windows, to update your build",
            Action::PauseTracking => "Pause or resume tracking",
            Action::NewSession => "Start a new session",
            Action::ToggleLogging => "Log this run or not",
            Action::SaveFrame => "Save the picture last read",
            Action::ShowWorkbench => "Open the workbench",
        }
    }

    /// Whether this build has this action. Only development builds have a workbench.
    pub fn available(self) -> bool {
        self != Action::ShowWorkbench || cfg!(debug_assertions)
    }

    /// The name settings know this action by.
    fn name(self) -> String {
        format!("{self:?}")
    }

    /// The keys set for this action; empty if it has none.
    fn keys(self, choice: &HotkeyChoice) -> String {
        choice.keys.get(&self.name()).cloned().unwrap_or_default()
    }

    pub fn run(self, app: &AppHandle) -> Result<()> {
        match self {
            Action::ToggleOverlay => overlay::toggle_visibility(app),
            Action::ToggleClickThrough => overlay::toggle_click_through(app),
            Action::TogglePins => pins::toggle_visibility(app),
            Action::ToggleCompact => overlay::toggle_compact(app),
            Action::ToggleLive => live::toggle(app),
            Action::ReadItem => live::read_item(app),
            Action::ReadCharacter => live::read_character(app),
            Action::PauseTracking => {
                live::toggle_held(app);
                Ok(())
            }
            Action::NewSession => {
                live::reset(app);
                Ok(())
            }
            Action::ToggleLogging => {
                live::toggle_logging(app);
                Ok(())
            }
            Action::SaveFrame => live::save_frame(app).map(|_| ()),
            Action::ShowWorkbench => overlay::show_workbench(app),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutInfo {
    pub action: String,
    /// Empty when this action has no hotkey.
    pub keys: String,
    pub suggested_keys: &'static str,
    pub label: &'static str,
    /// Setup asks about it.
    pub in_setup: bool,
    /// Its keys are set but another program on this PC holds them, so they do nothing.
    pub taken: bool,
}

pub fn list(app: &AppHandle, choice: &HotkeyChoice) -> Vec<ShortcutInfo> {
    let taken = app.state::<Bindings>().taken.lock().expect("hotkey lock").clone();
    Action::ALL
        .iter()
        .filter(|action| action.available())
        .map(|&action| ShortcutInfo {
            action: action.name(),
            keys: action.keys(choice),
            suggested_keys: action.suggested_keys(),
            label: action.label(),
            in_setup: action.in_setup(),
            taken: taken.contains(&action),
        })
        .collect()
}

/// Emitted with the action's name whenever one of its hotkeys arrives: setup ticks it, Settings
/// shows it was received, and a key picker takes it as the key pressed.
pub const HOTKEY_EVENT: &str = "hotkey";

/// How long one call to [`set_hotkeys_quiet`] keeps hotkeys quiet. The page renews it while it
/// needs them quiet, so a window that closes without saying so can't leave them dead.
const QUIET_FOR: Duration = Duration::from_secs(30);

/// The hotkeys in force, for the handler to look up, the ones another program holds, and how
/// often each arrived, for the bug report.
#[derive(Default)]
pub struct Bindings {
    held: Mutex<Vec<(Shortcut, Action)>>,
    taken: Mutex<Vec<Action>>,
    /// Until when a press only reports itself and runs nothing.
    quiet_until: Mutex<Option<Instant>>,
    pressed: Mutex<Vec<(Action, u32)>>,
}

/// While setup tests the keys or a key is being picked, a press only reports itself (the
/// `hotkey` event) and runs nothing. The hotkeys stay registered throughout, so nothing can be
/// left unregistered; quiet ends by itself after [`QUIET_FOR`] unless renewed.
#[tauri::command]
pub fn set_hotkeys_quiet(app: AppHandle, quiet: bool) {
    *app.state::<Bindings>().quiet_until.lock().expect("hotkey lock") = quiet.then(|| Instant::now() + QUIET_FOR);
}

/// How often each hotkey arrived since Leafline started, for the bug report.
pub fn pressed_summary(app: &AppHandle) -> String {
    let pressed = app.state::<Bindings>().pressed.lock().expect("hotkey lock").clone();
    if pressed.is_empty() {
        return "none".into();
    }
    pressed.iter().map(|(action, n)| format!("{} {n}", action.name())).collect::<Vec<_>>().join(", ")
}

/// The hotkeys set and held, for the bug report: which are in force and which another program has.
pub fn held_summary(app: &AppHandle) -> String {
    let bindings = app.state::<Bindings>();
    let held = bindings.held.lock().expect("hotkey lock").iter().map(|(_, a)| a.name()).collect::<Vec<_>>();
    let taken = bindings.taken.lock().expect("hotkey lock").iter().map(|a| a.name()).collect::<Vec<_>>();
    format!("held [{}], taken by another program [{}]", held.join(", "), taken.join(", "))
}

/// Keys that type nothing in chat, so they can be hotkeys on their own: F-keys, Pause, Scroll
/// Lock and the numpad. The game can't bind the numpad, but a player typing digits there in chat
/// would lose them; that is the player's choice.
fn types_nothing(key: &str) -> bool {
    matches!(key, "Pause" | "ScrollLock")
        || key.starts_with("Numpad")
        || key.strip_prefix('F').is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

/// Parses every hotkey. A key that types something needs Ctrl, Alt or Win, or it would stop
/// working in the game's chat (see `types_nothing`). Two actions can't share one.
fn parsed(choice: &HotkeyChoice) -> Result<Vec<(Shortcut, Action)>> {
    let mut bindings: Vec<(Shortcut, Action)> = Vec::new();
    for action in Action::ALL.into_iter().filter(|action| action.available()) {
        let keys = action.keys(choice);
        if keys.is_empty() {
            continue;
        }
        let shortcut: Shortcut =
            keys.parse().map_err(|_| Error::Invalid(format!("{keys} isn't a hotkey Leafline can use.")))?;
        let alone_ok = keys.rsplit('+').next().is_some_and(types_nothing);
        if !alone_ok && !shortcut.mods.intersects(Modifiers::CONTROL | Modifiers::ALT | Modifiers::SUPER) {
            return Err(Error::Invalid(format!(
                "{keys} would stop that key working in the game's chat. Use an F-key, Pause or a numpad key, or add Win."
            )));
        }
        if bindings.iter().any(|(other, _)| *other == shortcut) {
            return Err(Error::Invalid(format!("{keys} is already used by another hotkey.")));
        }
        bindings.push((shortcut, action));
    }
    Ok(bindings)
}

/// Whether these hotkeys can be used, before they are saved. F12 alone is refused here rather
/// than in `parsed`, so a settings file that already has it still loads its other hotkeys.
pub fn check(choice: &HotkeyChoice) -> Result<()> {
    if choice.keys.values().any(|keys| keys == "F12") {
        return Err(Error::Invalid("Windows keeps F12 for itself, so no program can use it as a hotkey. Pick another key.".into()));
    }
    parsed(choice).map(|_| ())
}

/// Puts these hotkeys in force in place of the ones before. One another program holds is
/// skipped, so the rest still work, and listed so Settings and setup can say so.
pub fn apply(app: &AppHandle, choice: &HotkeyChoice) -> Result<()> {
    let manager = app.global_shortcut();
    manager.unregister_all().map_err(|err| Error::Invalid(err.to_string()))?;
    let mut held = Vec::new();
    let mut taken = Vec::new();
    if choice.enabled {
        for (shortcut, action) in parsed(choice)? {
            match manager.register(shortcut) {
                Ok(()) => held.push((shortcut, action)),
                Err(err) => {
                    report::note(format!("Hotkey for \"{}\" isn't available: {err}", action.label()));
                    taken.push(action);
                }
            }
        }
    }
    let bindings = app.state::<Bindings>();
    *bindings.held.lock().expect("hotkey lock") = held;
    *bindings.taken.lock().expect("hotkey lock") = taken;
    Ok(())
}

pub fn plugin() -> TauriPlugin<Wry> {
    Builder::new()
        .with_handler(|app, shortcut, event| {
            if event.state != ShortcutState::Pressed {
                return;
            }
            let bindings = app.state::<Bindings>();
            let action = bindings.held.lock().expect("hotkey lock").iter().find(|(s, _)| s == shortcut).map(|(_, a)| *a);
            let Some(action) = action else { return };
            {
                let mut pressed = bindings.pressed.lock().expect("hotkey lock");
                match pressed.iter_mut().find(|(a, _)| *a == action) {
                    Some((_, n)) => *n += 1,
                    None => pressed.push((action, 1)),
                }
            }
            let _ = app.emit(HOTKEY_EVENT, action.name());
            let quiet = bindings.quiet_until.lock().expect("hotkey lock").is_some_and(|until| Instant::now() < until);
            if quiet {
                return;
            }
            if let Err(err) = action.run(app) {
                log_error(&err);
            }
        })
        .build()
}

pub fn log_error(err: &Error) {
    report::note(err.to_string());
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    fn with(action: Action, keys: &str) -> HotkeyChoice {
        HotkeyChoice { enabled: true, keys: BTreeMap::from([(action.name(), keys.to_string())]) }
    }

    fn suggested() -> HotkeyChoice {
        let keys = Action::ALL
            .iter()
            .filter(|a| !a.suggested_keys().is_empty())
            .map(|a| (a.name(), a.suggested_keys().to_string()))
            .collect();
        HotkeyChoice { enabled: true, keys }
    }

    #[test]
    fn nothing_is_bound_until_the_player_picks() {
        assert!(parsed(&HotkeyChoice::default()).unwrap().is_empty());
    }

    #[test]
    fn the_suggested_hotkeys_are_all_usable_and_use_no_ctrl_or_alt() {
        let bound = parsed(&suggested()).unwrap();
        assert_eq!(bound.len(), suggested().keys.len());
        assert!(bound.iter().all(|(s, _)| !s.mods.intersects(Modifiers::CONTROL | Modifiers::ALT)));
    }

    #[test]
    fn a_hotkey_can_be_set_or_turned_off() {
        assert_eq!(parsed(&with(Action::ToggleLive, "Ctrl+Shift+F5")).unwrap().len(), 1);
        assert!(parsed(&with(Action::ToggleLive, "")).unwrap().is_empty());
        assert!(check(&with(Action::ToggleLive, "F9")).is_ok());
        assert!(check(&with(Action::ToggleLive, "Pause")).is_ok());
        assert!(check(&with(Action::ToggleLive, "ScrollLock")).is_ok());
        assert!(check(&with(Action::ToggleLive, "Numpad5")).is_ok());
        assert!(check(&with(Action::ToggleLive, "NumpadAdd")).is_ok());
        assert!(check(&with(Action::ToggleLive, "Super+Shift+K")).is_ok());
    }

    #[test]
    fn keys_that_would_get_in_the_way_are_refused() {
        assert!(check(&with(Action::ToggleLive, "Shift+1")).is_err(), "it would stop ! working in chat");
        assert!(check(&with(Action::ToggleLive, "K")).is_err(), "it would stop k working in chat");
        let mut both = suggested();
        both.keys.insert(Action::ToggleLive.name(), "Super+F8".into());
        assert!(check(&both).is_err(), "taken by another action");
        assert!(check(&with(Action::ToggleLive, "Banana")).is_err());
        assert!(check(&with(Action::ToggleLive, "F12")).is_err(), "Windows keeps F12");
        assert!(parsed(&with(Action::ToggleLive, "F12")).is_ok(), "an old settings file with F12 still loads");
    }
}
