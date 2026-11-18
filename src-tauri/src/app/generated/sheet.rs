//! Tauri commands for `leafline_core::service::sheet`. Written by tools/gen-commands.py;
//! change the service function, then run the script again.
#![allow(unused_imports)]

use tauri::AppHandle;

use leafline_core::service;
use crate::app::host::AppHost;
use leafline_core::service::sheet::*;
use serde::{Deserialize, Serialize};
use leafline_core::service::damage::PlayerDamage;
use leafline_core::service::host::Host;
use leafline_core::build::{Build, Catalog};
use leafline_core::mapdata::osms::{Effects, SkillRecord};
use leafline_core::readers::character::{DetailBonus, DetailWindow, StatWindow};

#[tauri::command]
pub fn character_stats(app: AppHandle, name: String) -> StatsView {
    service::sheet::character_stats(&AppHost(app), name)
}

#[tauri::command]
pub fn set_buff(app: AppHandle, name: String, skill: u32, on: bool) -> crate::error::Result<()> {
    service::sheet::set_buff(&AppHost(app), name, skill, on)
}
