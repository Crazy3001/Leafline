//! Global hotkeys. They work while the game has focus, without sending it any input.
//!
//! Ctrl+Alt combinations are used because the game binds single keys, Ctrl and Alt alone.

use serde::Serialize;
use tauri::plugin::TauriPlugin;
use tauri::{AppHandle, Wry};
use tauri_plugin_global_shortcut::{Builder, Shortcut, ShortcutState};

use super::{overlay, pins};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Action {
    ToggleOverlay,
    ToggleClickThrough,
    TogglePins,
    ToggleCompact,
    ShowWorkbench,
}

impl Action {
    pub const ALL: [Action; 5] = [
        Action::ToggleOverlay,
        Action::ToggleClickThrough,
        Action::TogglePins,
        Action::ToggleCompact,
        Action::ShowWorkbench,
    ];

    pub fn keys(self) -> &'static str {
        match self {
            Action::ToggleOverlay => "Ctrl+Alt+O",
            Action::ToggleClickThrough => "Ctrl+Alt+L",
            Action::TogglePins => "Ctrl+Alt+P",
            Action::ToggleCompact => "Ctrl+Alt+H",
            Action::ShowWorkbench => "Ctrl+Alt+W",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Action::ToggleOverlay => "Show or hide the overlay",
            Action::ToggleClickThrough => "Lock the overlay (click-through)",
            Action::TogglePins => "Show or hide pinned panels",
            Action::ToggleCompact => "Compact HUD: only EXP per hour and quest counters",
            Action::ShowWorkbench => "Open the workbench",
        }
    }

    fn shortcut(self) -> Shortcut {
        self.keys().parse().expect("built-in shortcut is valid")
    }

    pub fn run(self, app: &AppHandle) -> crate::error::Result<()> {
        match self {
            Action::ToggleOverlay => overlay::toggle_visibility(app),
            Action::ToggleClickThrough => overlay::toggle_click_through(app),
            Action::TogglePins => pins::toggle_visibility(app),
            Action::ToggleCompact => overlay::toggle_compact(app),
            Action::ShowWorkbench => overlay::show_workbench(app),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutInfo {
    pub action: Action,
    pub keys: &'static str,
    pub label: &'static str,
}

pub fn list() -> Vec<ShortcutInfo> {
    Action::ALL.iter().map(|&action| ShortcutInfo { action, keys: action.keys(), label: action.label() }).collect()
}

pub fn plugin() -> TauriPlugin<Wry> {
    Builder::new()
        .with_shortcuts(Action::ALL.map(Action::shortcut))
        .expect("built-in shortcuts register")
        .with_handler(|app, shortcut, event| {
            if event.state != ShortcutState::Pressed {
                return;
            }
            if let Some(action) = Action::ALL.into_iter().find(|a| a.shortcut() == *shortcut)
                && let Err(err) = action.run(app)
            {
                log_error(&err);
            }
        })
        .build()
}

pub fn log_error(err: &crate::error::Error) {
    eprintln!("leafline: {err}");
}
