//! Tauri commands for `leafline_core::service::records`. Written by tools/gen-commands.py;
//! change the service function, then run the script again.
#![allow(unused_imports)]

use tauri::AppHandle;

use leafline_core::service;
use crate::app::host::AppHost;
use leafline_core::service::records::*;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};
use serde::{Deserialize, Serialize};
use leafline_core::service::bag::BagCheck;
use leafline_core::service::host::{Host, emit};
use leafline_core::error::{Error, Result};
use leafline_core::session::SessionSummary;
use leafline_core::userdata::{RecordsSummary, UserData};
use leafline_core::readers::charinfo::SeenLook;
use leafline_core::userdata::characters::{KnownCharacter, SeenCharacter};
use leafline_core::userdata::drops::{LoggedDrop, NewDrop};
use leafline_core::userdata::notes::{ItemNote, MapNote};
use leafline_core::userdata::potions::{PotionSlot, SessionPotion};
use leafline_core::userdata::sessions::{LevelExp, MapRate, SavedSession, SessionTimeline};
use leafline_core::userdata::spots::SavedSpot;
use leafline_core::clock::now;

#[tauri::command]
pub fn records_summary(app: AppHandle) -> Result<RecordsSummary> {
    service::records::records_summary(&AppHost(app))
}

#[tauri::command]
pub fn known_character(app: AppHandle, name: Option<String>) -> Result<Option<KnownCharacter>> {
    service::records::known_character(&AppHost(app), name)
}

#[tauri::command]
pub fn known_characters(app: AppHandle) -> Result<Vec<String>> {
    service::records::known_characters(&AppHost(app))
}

#[tauri::command]
pub fn known_bags(app: AppHandle, name: Option<String>) -> Result<Vec<BagCheck>> {
    service::records::known_bags(&AppHost(app), name)
}

#[tauri::command]
pub fn keep_look(app: AppHandle, look: SeenLook) -> Result<()> {
    service::records::keep_look(&AppHost(app), look)
}

#[tauri::command]
pub fn character_looks(app: AppHandle) -> Result<Vec<CharacterLook>> {
    service::records::character_looks(&AppHost(app))
}

#[tauri::command]
pub fn forget_looks(app: AppHandle) -> Result<()> {
    service::records::forget_looks(&AppHost(app))
}

#[tauri::command]
pub fn saved_spots(app: AppHandle) -> Result<Vec<SavedSpot>> {
    service::records::saved_spots(&AppHost(app))
}

#[tauri::command]
pub fn set_spot_saved(app: AppHandle, map_id: i64, name: String, saved: bool) -> Result<Vec<SavedSpot>> {
    service::records::set_spot_saved(&AppHost(app), map_id, name, saved)
}

#[tauri::command]
pub fn loot_prices(app: AppHandle, ids: Vec<u32>) -> Result<Vec<LootPrice>> {
    service::records::loot_prices(&AppHost(app), ids)
}

#[tauri::command]
pub fn potion_slots(app: AppHandle, character: Option<String>) -> Result<Vec<PotionSlot>> {
    service::records::potion_slots(&AppHost(app), character)
}

#[tauri::command]
pub fn set_potion_slots(app: AppHandle, character: Option<String>, slots: Vec<PotionSlot>) -> Result<Vec<PotionSlot>> {
    service::records::set_potion_slots(&AppHost(app), character, slots)
}

#[tauri::command]
pub fn session_potions(app: AppHandle, started_at: f64) -> Result<Vec<SessionPotion>> {
    service::records::session_potions(&AppHost(app), started_at)
}

#[tauri::command]
pub fn set_session_potion(app: AppHandle, character: Option<String>, started_at: f64, slot: u32, start: Option<i64>, now: Option<i64>) -> Result<Vec<SessionPotion>> {
    service::records::set_session_potion(&AppHost(app), character, started_at, slot, start, now)
}

#[tauri::command]
pub fn item_note(app: AppHandle, item: String) -> Result<Option<ItemNote>> {
    service::records::item_note(&AppHost(app), item)
}

#[tauri::command]
pub fn set_item_note(app: AppHandle, item: String, note: String) -> Result<Option<ItemNote>> {
    service::records::set_item_note(&AppHost(app), item, note)
}

#[tauri::command]
pub fn map_note(app: AppHandle, map_id: i64) -> Result<Option<MapNote>> {
    service::records::map_note(&AppHost(app), map_id)
}

#[tauri::command]
pub fn set_map_note(app: AppHandle, map_id: i64, map: String, note: String) -> Result<Option<MapNote>> {
    service::records::set_map_note(&AppHost(app), map_id, map, note)
}

#[tauri::command]
pub fn drop_log(app: AppHandle) -> Result<Vec<LoggedDrop>> {
    service::records::drop_log(&AppHost(app))
}

#[tauri::command]
pub fn log_drop(app: AppHandle, drop: NewDrop) -> Result<i64> {
    service::records::log_drop(&AppHost(app), drop)
}

#[tauri::command]
pub fn delete_drop(app: AppHandle, id: i64) -> Result<()> {
    service::records::delete_drop(&AppHost(app), id)
}

#[tauri::command]
pub fn session_history(app: AppHandle) -> Result<History> {
    service::records::session_history(&AppHost(app))
}

#[tauri::command]
pub fn level_exp(app: AppHandle) -> Result<Vec<LevelExp>> {
    service::records::level_exp(&AppHost(app))
}

#[tauri::command]
pub fn session_timeline(app: AppHandle, id: i64) -> Result<SessionTimeline> {
    service::records::session_timeline(&AppHost(app), id)
}

#[tauri::command]
pub fn delete_session(app: AppHandle, id: i64) -> Result<()> {
    service::records::delete_session(&AppHost(app), id)
}

#[tauri::command]
pub fn set_session_note(app: AppHandle, id: i64, note: String) -> Result<()> {
    service::records::set_session_note(&AppHost(app), id, note)
}

#[tauri::command]
pub fn rename_map(app: AppHandle, from: String, to: String) -> Result<()> {
    service::records::rename_map(&AppHost(app), from, to)
}
