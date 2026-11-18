//! Tauri commands for `leafline_core::service::quests`. Written by tools/gen-commands.py;
//! change the service function, then run the script again.
#![allow(unused_imports)]

use tauri::AppHandle;

use leafline_core::service;
use crate::app::host::AppHost;
use leafline_core::service::quests::*;
use leafline_core::service::host::{Host, emit};
use leafline_core::error::Result;
use leafline_core::mapdata::questlog::RegionTotal;
use leafline_core::userdata::questlog::QuestLog;

#[tauri::command]
pub fn quest_log(app: AppHandle, name: Option<String>) -> Result<QuestLog> {
    service::quests::quest_log(&AppHost(app), name)
}

#[tauri::command]
pub fn forget_quest_log(app: AppHandle, name: String) -> Result<()> {
    service::quests::forget_quest_log(&AppHost(app), name)
}

#[tauri::command]
pub fn quest_region_totals(app: AppHandle) -> Vec<RegionTotal> {
    service::quests::quest_region_totals(&AppHost(app))
}
