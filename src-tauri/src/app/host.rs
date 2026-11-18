//! The program as a [`Host`] for the shared commands: its managed state, its data folder, and
//! Tauri events to every window.

use tauri::{AppHandle, Emitter, Manager};

use super::damage::DamageState;
use super::mapdata::MapDataState;
use super::records::UserDataState;
use super::settings::{Settings, SettingsState};
use leafline_core::service::files::{Files, Folder};
use leafline_core::service::host::Host;

/// The app data folder, where settings, Go here and the read damage are kept.
pub struct DataFolder(pub Folder);

/// The program, for the shared commands. Owns a handle so a command can run off the UI thread.
pub struct AppHost(pub AppHandle);

impl AppHost {
    pub fn new(app: &AppHandle) -> Self {
        Self(app.clone())
    }

    pub fn files_of(app: &AppHandle) -> &dyn Files {
        &app.state::<DataFolder>().inner().0
    }
}

impl Host for AppHost {
    fn map(&self) -> &MapDataState {
        self.0.state::<MapDataState>().inner()
    }

    fn records(&self) -> &UserDataState {
        self.0.state::<UserDataState>().inner()
    }

    fn damage(&self) -> &DamageState {
        self.0.state::<DamageState>().inner()
    }

    fn settings(&self) -> Settings {
        self.0.state::<SettingsState>().get()
    }

    fn files(&self) -> &dyn Files {
        Self::files_of(&self.0)
    }

    fn emit_json(&self, event: &str, payload: serde_json::Value) {
        if let Err(err) = self.0.emit(event, payload) {
            eprintln!("could not tell the windows about {event}: {err}");
        }
    }
}
