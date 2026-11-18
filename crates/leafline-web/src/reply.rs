//! What a command sends back to the page: its value as JSON, or a plain error message, as the
//! program's Tauri commands do.

use serde::Serialize;
use serde::de::DeserializeOwned;

pub type Reply = Result<serde_json::Value, String>;

/// The command's arguments, from the object the page passed.
pub fn parse<T: DeserializeOwned>(args: serde_json::Value) -> Result<T, String> {
    serde_json::from_value(args).map_err(|err| format!("bad arguments: {err}"))
}

pub fn reply<T: Serialize>(value: T) -> Reply {
    serde_json::to_value(value).map_err(|err| err.to_string())
}

pub fn reply_result<T: Serialize>(value: leafline_core::error::Result<T>) -> Reply {
    value.map_err(|err| err.to_string()).and_then(reply)
}
