//! Global hotkeys. They work while the game has focus, without sending it any input.
//!
//! Ctrl+Alt combinations are the default because the game binds single keys, Ctrl and Alt alone.
//! The player can change them, turn one off, or turn them all off.

use std::sync::Mutex;

use serde::Serialize;
use tauri::plugin::TauriPlugin;
use tauri::{AppHandle, Manager, Wry};
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
    PauseTracking,
    NewSession,
    ToggleLogging,
    SaveFrame,
    ShowWorkbench,
}

impl Action {
    pub const ALL: [Action; 11] = [
        Action::ToggleOverlay,
        Action::ToggleClickThrough,
        Action::TogglePins,
        Action::ToggleCompact,
        Action::ToggleLive,
        Action::ReadItem,
        Action::PauseTracking,
        Action::NewSession,
        Action::ToggleLogging,
        Action::SaveFrame,
        Action::ShowWorkbench,
    ];

    pub fn default_keys(self) -> &'static str {
        match self {
            Action::ToggleOverlay => "Ctrl+Alt+O",
            Action::ToggleClickThrough => "Ctrl+Alt+L",
            Action::TogglePins => "Ctrl+Alt+P",
            Action::ToggleCompact => "Ctrl+Alt+H",
            Action::ToggleLive => "Ctrl+Alt+R",
            Action::ReadItem => "Ctrl+Alt+I",
            Action::PauseTracking => "Ctrl+Alt+K",
            Action::NewSession => "Ctrl+Alt+N",
            Action::ToggleLogging => "Ctrl+Alt+J",
            Action::SaveFrame => "Ctrl+Alt+F",
            Action::ShowWorkbench => "Ctrl+Alt+W",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Action::ToggleOverlay => "Show or hide the overlay",
            Action::ToggleClickThrough => "Lock the overlay (click-through)",
            Action::TogglePins => "Show or hide pinned panels",
            Action::ToggleCompact => "Compact HUD",
            Action::ToggleLive => "Start or stop live reading",
            Action::ReadItem => "Read the item you're hovering, for a price check",
            Action::PauseTracking => "Pause or resume tracking",
            Action::NewSession => "Start a new session",
            Action::ToggleLogging => "Log this run or not",
            Action::SaveFrame => "Save the picture last read, to Pictures\\Leafline",
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

    /// The keys set for this action; empty if it is turned off.
    fn keys(self, choice: &HotkeyChoice) -> String {
        choice.keys.get(&self.name()).cloned().unwrap_or_else(|| self.default_keys().to_string())
    }

    pub fn run(self, app: &AppHandle) -> Result<()> {
        match self {
            Action::ToggleOverlay => overlay::toggle_visibility(app),
            Action::ToggleClickThrough => overlay::toggle_click_through(app),
            Action::TogglePins => pins::toggle_visibility(app),
            Action::ToggleCompact => overlay::toggle_compact(app),
            Action::ToggleLive => live::toggle(app),
            Action::ReadItem => live::read_item(app),
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
    /// Empty when this hotkey is turned off.
    pub keys: String,
    pub default_keys: &'static str,
    pub label: &'static str,
}

pub fn list(choice: &HotkeyChoice) -> Vec<ShortcutInfo> {
    Action::ALL
        .iter()
        .filter(|action| action.available())
        .map(|&action| ShortcutInfo {
            action: action.name(),
            keys: action.keys(choice),
            default_keys: action.default_keys(),
            label: action.label(),
        })
        .collect()
}

/// The hotkeys in force, for the handler to look up.
#[derive(Default)]
pub struct Bindings(Mutex<Vec<(Shortcut, Action)>>);

/// Parses every hotkey. A key without Ctrl, Alt or Win would stop working in the game's chat,
/// and two actions can't share one.
fn parsed(choice: &HotkeyChoice) -> Result<Vec<(Shortcut, Action)>> {
    let mut bindings: Vec<(Shortcut, Action)> = Vec::new();
    for action in Action::ALL.into_iter().filter(|action| action.available()) {
        let keys = action.keys(choice);
        if keys.is_empty() {
            continue;
        }
        let shortcut: Shortcut =
            keys.parse().map_err(|_| Error::Invalid(format!("{keys} isn't a hotkey Leafline can use.")))?;
        let function_key = keys.rsplit('+').next().is_some_and(|key| {
            key.strip_prefix('F').is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
        });
        if !function_key && !shortcut.mods.intersects(Modifiers::CONTROL | Modifiers::ALT | Modifiers::SUPER) {
            return Err(Error::Invalid(format!("{keys} would stop that key working in the game. Add Ctrl or Alt.")));
        }
        if bindings.iter().any(|(other, _)| *other == shortcut) {
            return Err(Error::Invalid(format!("{keys} is already used by another hotkey.")));
        }
        bindings.push((shortcut, action));
    }
    Ok(bindings)
}

/// Whether these hotkeys can be used, before they are saved.
pub fn check(choice: &HotkeyChoice) -> Result<()> {
    parsed(choice).map(|_| ())
}

/// Puts these hotkeys in force in place of the ones before. One another app holds is noted and
/// skipped, so the rest still work.
pub fn apply(app: &AppHandle, choice: &HotkeyChoice) -> Result<()> {
    let manager = app.global_shortcut();
    manager.unregister_all().map_err(|err| Error::Invalid(err.to_string()))?;
    let mut held = Vec::new();
    if choice.enabled {
        for (shortcut, action) in parsed(choice)? {
            match manager.register(shortcut) {
                Ok(()) => held.push((shortcut, action)),
                Err(err) => report::note(format!("Hotkey for \"{}\" isn't available: {err}", action.label())),
            }
        }
    }
    *app.state::<Bindings>().0.lock().expect("hotkey lock") = held;
    Ok(())
}

pub fn plugin() -> TauriPlugin<Wry> {
    Builder::new()
        .with_handler(|app, shortcut, event| {
            if event.state != ShortcutState::Pressed {
                return;
            }
            let bindings = app.state::<Bindings>();
            let action = bindings.0.lock().expect("hotkey lock").iter().find(|(s, _)| s == shortcut).map(|(_, a)| *a);
            if let Some(action) = action
                && let Err(err) = action.run(app)
            {
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

    #[test]
    fn the_default_hotkeys_are_all_usable() {
        assert_eq!(parsed(&HotkeyChoice::default()).unwrap().len(), Action::ALL.len());
    }

    #[test]
    fn a_hotkey_can_be_changed_or_turned_off() {
        let changed = parsed(&with(Action::ToggleLive, "Ctrl+Shift+F5")).unwrap();
        assert_eq!(changed.len(), Action::ALL.len());
        assert_eq!(parsed(&with(Action::ToggleLive, "")).unwrap().len(), Action::ALL.len() - 1);
        assert!(check(&with(Action::ToggleLive, "F9")).is_ok());
    }

    #[test]
    fn keys_that_would_get_in_the_way_are_refused() {
        assert!(check(&with(Action::ToggleLive, "Shift+1")).is_err(), "it would stop ! working in chat");
        assert!(check(&with(Action::ToggleLive, "Ctrl+Alt+O")).is_err(), "taken by another action");
        assert!(check(&with(Action::ToggleLive, "Banana")).is_err());
    }
}
