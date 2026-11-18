//! Tauri commands for `leafline_core::service::builds`. Written by tools/gen-commands.py;
//! change the service function, then run the script again.
#![allow(unused_imports)]

use tauri::AppHandle;

use leafline_core::service;
use crate::app::host::AppHost;
use leafline_core::service::builds::*;
use serde::Serialize;
use leafline_core::service::host::{Host, emit};
use leafline_core::service::records::now;
use leafline_core::build::crafting::{CraftingPage, CraftingQuery, ItemCrafting};
use leafline_core::build::plan::{MAX_LEVEL, Piece};
use leafline_core::build::progress::{self, CatchUp};
use leafline_core::build::{
    Branch, Build, Catalog, Expansion, GearFor, GearFound, GearSuggestions, ItemFilters, ItemFit, ItemJob, ItemPage,
    ItemQuery, JobChoice, Outfit, Rules, Starter, Wearer, expand, fit, outfit, share, starters, suggest,
};
use leafline_core::error::{Error, Result};
use leafline_core::mapdata::osms::{ItemRecord, SkillRecord};
use leafline_core::pipeline::CharacterScan;
use leafline_core::readers::tooltip::Requirements;
use leafline_core::userdata::builds::BuildSummary;

#[tauri::command]
pub fn build_catalog(app: AppHandle) -> Option<CatalogView> {
    service::builds::build_catalog(&AppHost(app))
}

#[tauri::command]
pub fn starter_plans(app: AppHandle) -> Vec<Starter> {
    service::builds::starter_plans(&AppHost(app))
}

#[tauri::command]
pub fn search_gear(app: AppHandle, text: String, job: u32) -> Vec<ItemRecord> {
    service::builds::search_gear(&AppHost(app), text, job)
}

#[tauri::command]
pub fn item_crafting(app: AppHandle, id: u32) -> Option<ItemCrafting> {
    service::builds::item_crafting(&AppHost(app), id)
}

#[tauri::command]
pub fn crafting_recipes(app: AppHandle, query: CraftingQuery) -> CraftingPage {
    service::builds::crafting_recipes(&AppHost(app), query)
}

#[tauri::command]
pub fn search_items(app: AppHandle, query: ItemQuery, level: Option<u32>, character: Option<String>) -> Result<ItemPage> {
    service::builds::search_items(&AppHost(app), query, level, character)
}

#[tauri::command]
pub fn item_filters(app: AppHandle) -> ItemFilters {
    service::builds::item_filters(&AppHost(app))
}

#[tauri::command]
pub fn item_record(app: AppHandle, id: u32) -> Option<ItemRecord> {
    service::builds::item_record(&AppHost(app), id)
}

#[tauri::command]
pub fn find_item(app: AppHandle, name: String, level: Option<u32>) -> Option<ItemRecord> {
    service::builds::find_item(&AppHost(app), name, level)
}

#[tauri::command]
pub fn add_piece(app: AppHandle, piece: Piece, character: Option<String>) -> Result<()> {
    service::builds::add_piece(&AppHost(app), piece, character)
}

#[tauri::command]
pub fn wear_item(app: AppHandle, item: u32, character: Option<String>) -> Result<()> {
    service::builds::wear_item(&AppHost(app), item, character)
}

#[tauri::command]
pub fn update_piece(app: AppHandle, piece: Piece, character: Option<String>) -> Result<()> {
    service::builds::update_piece(&AppHost(app), piece, character)
}

#[tauri::command]
pub fn remove_piece(app: AppHandle, id: i64, character: Option<String>) -> Result<()> {
    service::builds::remove_piece(&AppHost(app), id, character)
}

#[tauri::command]
pub fn set_gear_goal(app: AppHandle, item: u32, wanted: bool, character: Option<String>) -> Result<()> {
    service::builds::set_gear_goal(&AppHost(app), item, wanted, character)
}

#[tauri::command]
pub fn build_outfit(app: AppHandle, id: i64, level: u32) -> Result<Option<Outfit>> {
    service::builds::build_outfit(&AppHost(app), id, level)
}

#[tauri::command]
pub fn fill_gear(app: AppHandle, id: i64, level: u32) -> Result<usize> {
    service::builds::fill_gear(&AppHost(app), id, level)
}

#[tauri::command]
pub fn gear_suggestions(app: AppHandle, id: i64, level: u32) -> Result<Option<GearSuggestions>> {
    service::builds::gear_suggestions(&AppHost(app), id, level)
}

#[tauri::command]
pub fn find_gear(app: AppHandle, who: GearFor, character: Option<String>, seen_job: Option<String>) -> Result<Option<GearFound>> {
    service::builds::find_gear(&AppHost(app), who, character, seen_job)
}

#[tauri::command]
pub fn list_builds(app: AppHandle) -> Result<Vec<BuildSummary>> {
    service::builds::list_builds(&AppHost(app))
}

#[tauri::command]
pub fn build_plan(app: AppHandle, id: i64) -> Result<Option<BuildPlan>> {
    service::builds::build_plan(&AppHost(app), id)
}

#[tauri::command]
pub fn active_build(app: AppHandle, character: Option<String>) -> Result<Option<BuildPlan>> {
    service::builds::active_build(&AppHost(app), character)
}

#[tauri::command]
pub fn save_build(app: AppHandle, build: Build) -> Result<BuildPlan> {
    service::builds::save_build(&AppHost(app), build)
}

#[tauri::command]
pub fn use_build(app: AppHandle, id: i64) -> Result<()> {
    service::builds::use_build(&AppHost(app), id)
}

#[tauri::command]
pub fn delete_build(app: AppHandle, id: i64) -> Result<()> {
    service::builds::delete_build(&AppHost(app), id)
}

#[tauri::command]
pub fn export_build(app: AppHandle, id: i64) -> Result<String> {
    service::builds::export_build(&AppHost(app), id)
}

#[tauri::command]
pub fn import_build(app: AppHandle, code: String, character: Option<String>) -> Result<BuildPlan> {
    service::builds::import_build(&AppHost(app), code, character)
}

#[tauri::command]
pub fn build_item_fit(app: AppHandle, requirements: Requirements, jobs: Vec<ItemJob>, item_type: Option<String>, level: Option<u32>, character: Option<String>) -> Result<Option<ItemFit>> {
    service::builds::build_item_fit(&AppHost(app), requirements, jobs, item_type, level, character)
}
