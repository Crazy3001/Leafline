//! Tauri commands for `leafline_core::service::mapdata`. Written by tools/gen-commands.py;
//! change the service function, then run the script again.
#![allow(unused_imports)]

use tauri::AppHandle;

use leafline_core::service;
use crate::app::host::AppHost;
use leafline_core::service::mapdata::*;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use serde::Serialize;
use leafline_core::service::fighter::{self, Choice};
use leafline_core::service::files::{Files, write_json};
use leafline_core::service::host::{Host, emit};
use leafline_core::mapdata::combat::{self, Fighter, Fit, Tuning};
use leafline_core::mapdata::training::EstimateCheck;
use leafline_core::build::Catalog;
use leafline_core::build::catalog::item_group;
use leafline_core::mapdata::osms::ItemKind;
use leafline_core::error::{Error, Result};
use leafline_core::mapdata::fetch::{self, Source};
use leafline_core::clock::now;
use leafline_core::mapdata::osms::Dataset;
use leafline_core::mapdata::pictures::PictureIndex;
use leafline_core::mapdata::layout::{self, Layouts};
use leafline_core::mapdata::store::{DROPS_FAILED, LAYOUTS_FAILED, TURN_INS_FAILED};
use leafline_core::mapdata::{
    Assumptions, Atlas, DataStatus, ItemOrigin, KillSpeed, MapCard, MapMatch, MapPage, MapQuery, MapRef, MeasuredRate,
    MonsterDrops, MonsterFilters, MonsterPage, MonsterQuery, MonsterSheet, NpcPage, NpcQuery, NpcSheet, QuestGuide, QuestPage, QuestQuery, QuestSearch,
    QuestSheet, RefreshReport, Route, Store, TrainingSpot, Travel, TravelFile, drops, shops, turnins,
};

#[tauri::command]
pub fn map_data_status(app: AppHandle) -> Result<DataStatus> {
    service::mapdata::map_data_status(&AppHost(app))
}

#[tauri::command]
pub async fn download_map_data(app: AppHandle, force: bool) -> Result<Downloaded> {
    tauri::async_runtime::spawn_blocking(move || service::mapdata::download_map_data(&AppHost(app), force))
        .await
        .map_err(|err| leafline_core::error::Error::Invalid(err.to_string()))?
}

#[tauri::command]
pub fn find_map(app: AppHandle, name: String, street: Option<String>, twin: Option<u32>) -> Option<MapMatch> {
    service::mapdata::find_map(&AppHost(app), name, street, twin)
}

#[tauri::command]
pub fn map_card(app: AppHandle, id: u32) -> Option<MapCard> {
    service::mapdata::map_card(&AppHost(app), id)
}

#[tauri::command]
pub fn search_maps(app: AppHandle, text: String) -> Vec<MapRef> {
    service::mapdata::search_maps(&AppHost(app), text)
}

#[tauri::command]
pub fn map_route(app: AppHandle, from: u32, to: u32) -> Option<RouteInfo> {
    service::mapdata::map_route(&AppHost(app), from, to)
}

#[tauri::command]
pub fn travel_info(app: AppHandle) -> TravelInfo {
    service::mapdata::travel_info(&AppHost(app))
}

#[tauri::command]
pub async fn download_travel(app: AppHandle) -> Result<TravelInfo> {
    tauri::async_runtime::spawn_blocking(move || service::mapdata::download_travel(&AppHost(app)))
        .await
        .map_err(|err| leafline_core::error::Error::Invalid(err.to_string()))?
}

#[tauri::command]
pub async fn monster_drops(app: AppHandle, monsters: Vec<String>) -> Result<Vec<MonsterDrops>> {
    tauri::async_runtime::spawn_blocking(move || service::mapdata::monster_drops(&AppHost(app), monsters))
        .await
        .map_err(|err| leafline_core::error::Error::Invalid(err.to_string()))?
}

#[tauri::command]
pub fn quest_guide(app: AppHandle, quests: Vec<QuestQuery>, map_name: Option<String>, map_street: Option<String>, map_twin: Option<u32>, measured: Vec<MeasuredRate>) -> Result<Option<Guide>> {
    service::mapdata::quest_guide(&AppHost(app), quests, map_name, map_street, map_twin, measured)
}

#[tauri::command]
pub async fn item_origin(app: AppHandle, id: u32, map_name: Option<String>, map_street: Option<String>, map_twin: Option<u32>) -> Result<Option<ItemWhere>> {
    tauri::async_runtime::spawn_blocking(move || service::mapdata::item_origin(&AppHost(app), id, map_name, map_street, map_twin))
        .await
        .map_err(|err| leafline_core::error::Error::Invalid(err.to_string()))?
}

#[tauri::command]
pub fn search_quests(app: AppHandle, search: QuestSearch) -> QuestPage {
    service::mapdata::search_quests(&AppHost(app), search)
}

#[tauri::command]
pub fn quest_regions(app: AppHandle) -> Vec<String> {
    service::mapdata::quest_regions(&AppHost(app))
}

#[tauri::command]
pub async fn quest_sheet(app: AppHandle, id: u32, map_name: Option<String>, map_street: Option<String>, map_twin: Option<u32>) -> Result<Option<QuestWhere>> {
    tauri::async_runtime::spawn_blocking(move || service::mapdata::quest_sheet(&AppHost(app), id, map_name, map_street, map_twin))
        .await
        .map_err(|err| leafline_core::error::Error::Invalid(err.to_string()))?
}

#[tauri::command]
pub fn search_monsters(app: AppHandle, query: MonsterQuery) -> Result<MonsterPage> {
    service::mapdata::search_monsters(&AppHost(app), query)
}

#[tauri::command]
pub fn monster_filters(app: AppHandle) -> MonsterFilters {
    service::mapdata::monster_filters(&AppHost(app))
}

#[tauri::command]
pub fn find_monster(app: AppHandle, name: String) -> Option<u32> {
    service::mapdata::find_monster(&AppHost(app), name)
}

#[tauri::command]
pub async fn monster_sheet(app: AppHandle, id: u32, map_name: Option<String>, map_street: Option<String>, map_twin: Option<u32>, measured: Vec<MeasuredRate>) -> Result<Option<MonsterWhere>> {
    tauri::async_runtime::spawn_blocking(move || service::mapdata::monster_sheet(&AppHost(app), id, map_name, map_street, map_twin, measured))
        .await
        .map_err(|err| leafline_core::error::Error::Invalid(err.to_string()))?
}

#[tauri::command]
pub fn browse_maps(app: AppHandle, query: MapQuery) -> MapPage {
    service::mapdata::browse_maps(&AppHost(app), query)
}

#[tauri::command]
pub fn map_regions(app: AppHandle) -> Vec<String> {
    service::mapdata::map_regions(&AppHost(app))
}

#[tauri::command]
pub fn map_areas(app: AppHandle) -> Vec<String> {
    service::mapdata::map_areas(&AppHost(app))
}

#[tauri::command]
pub fn search_npcs(app: AppHandle, query: NpcQuery) -> NpcPage {
    service::mapdata::search_npcs(&AppHost(app), query)
}

#[tauri::command]
pub fn find_npc(app: AppHandle, name: String) -> Option<u32> {
    service::mapdata::find_npc(&AppHost(app), name)
}

#[tauri::command]
pub fn npc_sheet(app: AppHandle, id: u32, map_name: Option<String>, map_street: Option<String>, map_twin: Option<u32>) -> Option<NpcWhere> {
    service::mapdata::npc_sheet(&AppHost(app), id, map_name, map_street, map_twin)
}

#[tauri::command]
pub async fn download_drops(app: AppHandle, retry: Option<bool>) -> Result<()> {
    tauri::async_runtime::spawn_blocking(move || service::mapdata::download_drops(&AppHost(app), retry))
        .await
        .map_err(|err| leafline_core::error::Error::Invalid(err.to_string()))?
}

#[tauri::command]
pub fn drops_fetched_at(app: AppHandle) -> Result<Option<i64>> {
    service::mapdata::drops_fetched_at(&AppHost(app))
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn training_spots(app: AppHandle, map_name: Option<String>, map_street: Option<String>, map_twin: Option<u32>, measured: Vec<MeasuredRate>, level: Option<u32>, assumptions: Option<Assumptions>, character: Option<String>, skill: Option<String>) -> Option<Spots> {
    service::mapdata::training_spots(&AppHost(app), map_name, map_street, map_twin, measured, level, assumptions, character, skill)
}

#[tauri::command]
pub fn shop_prices(app: AppHandle, ids: Vec<u32>) -> Vec<(u32, u64)> {
    service::mapdata::shop_prices(&AppHost(app), ids)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn monster_fight(app: AppHandle, id: u32, character: Option<String>, level: Option<u32>, measured: Vec<MeasuredRate>, assumptions: Option<Assumptions>, skill: Option<String>) -> Option<MonsterFight> {
    service::mapdata::monster_fight(&AppHost(app), id, character, level, measured, assumptions, skill)
}

#[tauri::command]
pub fn check_estimates(app: AppHandle, measured: Vec<MeasuredRate>, level: Option<u32>, assumptions: Option<Assumptions>, character: Option<String>, skill: Option<String>) -> Option<EstimateCheck> {
    service::mapdata::check_estimates(&AppHost(app), measured, level, assumptions, character, skill)
}
