//! What a command needs from whoever runs Leafline: the program (over Tauri) or the website's
//! worker. Each holds the same state and tells its windows or tabs about changes.

use serde::Serialize;

use super::damage::DamageState;
use super::files::Files;
use super::mapdata::MapDataState;
use super::records::UserDataState;
use super::settings::Settings;

pub trait Host {
    fn map(&self) -> &MapDataState;
    fn records(&self) -> &UserDataState;
    fn damage(&self) -> &DamageState;
    fn settings(&self) -> Settings;
    /// Where the small JSON files live.
    fn files(&self) -> &dyn Files;
    /// Tells every window (or tab) about a change, as the program's Tauri events do.
    fn emit_json(&self, event: &str, payload: serde_json::Value);
}

/// Sends an event with any payload that serializes.
pub fn emit<T: Serialize + ?Sized>(host: &dyn Host, event: &str, payload: &T) {
    host.emit_json(event, serde_json::to_value(payload).unwrap_or(serde_json::Value::Null));
}
