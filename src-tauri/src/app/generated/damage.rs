//! Tauri commands for `leafline_core::service::damage`. Written by tools/gen-commands.py;
//! change the service function, then run the script again.
#![allow(unused_imports)]

use tauri::AppHandle;

use leafline_core::service;
use crate::app::host::AppHost;
use leafline_core::service::damage::*;
use std::collections::HashMap;
use std::sync::Mutex;
use serde::{Deserialize, Serialize};
use leafline_core::service::files::Files;
use leafline_core::service::host::{Host, emit};
use leafline_core::service::records::now;
use leafline_core::service::sheet::{SkillSeen, StatSheet, buffs_of, seen_buffs, seen_on};
use leafline_core::build::Catalog;
use leafline_core::mapdata::osms::SkillRecord;
use leafline_core::error::{Error, Result};
use leafline_core::pipeline::CharacterScan;
use leafline_core::readers::character::SkillWindow;

#[tauri::command]
pub fn player_damage(app: AppHandle, name: String) -> Option<PlayerDamage> {
    service::damage::player_damage(&AppHost(app), name)
}
