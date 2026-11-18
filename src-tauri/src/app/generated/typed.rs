//! Tauri commands for `leafline_core::service::typed`. Written by tools/gen-commands.py;
//! change the service function, then run the script again.
#![allow(unused_imports)]

use tauri::AppHandle;

use leafline_core::service;
use crate::app::host::AppHost;
use leafline_core::service::typed::*;
use serde::{Deserialize, Serialize};
use leafline_core::service::host::{Host, emit};
use leafline_core::service::records::HISTORY_EVENT;
use leafline_core::clock::now_f64;
use leafline_core::error::{Error, Result};
use leafline_core::session::{MapSummary, SessionLine, SessionSummary};

#[tauri::command]
pub fn typed_session_outcome(app: AppHandle, session: TypedSession) -> Result<TypedOutcome> {
    service::typed::typed_session_outcome(&AppHost(app), session)
}

#[tauri::command]
pub fn save_typed_session(app: AppHandle, session: TypedSession) -> Result<i64> {
    service::typed::save_typed_session(&AppHost(app), session)
}
