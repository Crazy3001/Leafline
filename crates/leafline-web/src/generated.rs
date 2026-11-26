//! The website's command dispatcher. Written by tools/gen-commands.py; change the service
//! function, then run the script again.
#![allow(unused_imports, clippy::too_many_lines)]

use leafline_core::service::host::Host;

use crate::reply::Reply;

/// Runs a shared command by the name the program's frontend invokes it by. `None` when the
/// name isn't one of them.
pub fn dispatch(host: &dyn Host, name: &str, args: serde_json::Value) -> Option<Reply> {
    None        .or_else(|| builds::dispatch(host, name, args.clone()))
        .or_else(|| damage::dispatch(host, name, args.clone()))
        .or_else(|| mapdata::dispatch(host, name, args.clone()))
        .or_else(|| quests::dispatch(host, name, args.clone()))
        .or_else(|| records::dispatch(host, name, args.clone()))
        .or_else(|| sheet::dispatch(host, name, args.clone()))
        .or_else(|| typed::dispatch(host, name, args.clone()))
}

mod builds {
    use leafline_core::service;
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

    use crate::reply::{Reply, parse, reply, reply_result};

    pub fn dispatch(host: &dyn Host, name: &str, args: serde_json::Value) -> Option<Reply> {
        Some(match name {
            "build_catalog" => reply(service::builds::build_catalog(host)),
            "starter_plans" => reply(service::builds::starter_plans(host)),
            "search_gear" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                text: String,
                job: u32,
                }
                let Args { text, job } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply(service::builds::search_gear(host, text, job))
            }
            "item_crafting" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                id: u32,
                }
                let Args { id } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply(service::builds::item_crafting(host, id))
            }
            "crafting_recipes" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                query: CraftingQuery,
                }
                let Args { query } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply(service::builds::crafting_recipes(host, query))
            }
            "search_items" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                query: ItemQuery,
                level: Option<u32>,
                character: Option<String>,
                }
                let Args { query, level, character } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::builds::search_items(host, query, level, character))
            }
            "item_filters" => reply(service::builds::item_filters(host)),
            "item_record" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                id: u32,
                }
                let Args { id } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply(service::builds::item_record(host, id))
            }
            "find_item" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                name: String,
                level: Option<u32>,
                }
                let Args { name, level } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply(service::builds::find_item(host, name, level))
            }
            "add_piece" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                piece: Piece,
                character: Option<String>,
                }
                let Args { piece, character } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::builds::add_piece(host, piece, character))
            }
            "wear_item" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                item: u32,
                character: Option<String>,
                }
                let Args { item, character } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::builds::wear_item(host, item, character))
            }
            "update_piece" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                piece: Piece,
                character: Option<String>,
                }
                let Args { piece, character } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::builds::update_piece(host, piece, character))
            }
            "remove_piece" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                id: i64,
                character: Option<String>,
                }
                let Args { id, character } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::builds::remove_piece(host, id, character))
            }
            "set_gear_goal" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                item: u32,
                wanted: bool,
                character: Option<String>,
                }
                let Args { item, wanted, character } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::builds::set_gear_goal(host, item, wanted, character))
            }
            "build_outfit" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                id: i64,
                level: u32,
                }
                let Args { id, level } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::builds::build_outfit(host, id, level))
            }
            "fill_gear" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                id: i64,
                level: u32,
                }
                let Args { id, level } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::builds::fill_gear(host, id, level))
            }
            "gear_suggestions" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                id: i64,
                level: u32,
                }
                let Args { id, level } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::builds::gear_suggestions(host, id, level))
            }
            "find_gear" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                who: GearFor,
                character: Option<String>,
                seen_job: Option<String>,
                }
                let Args { who, character, seen_job } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::builds::find_gear(host, who, character, seen_job))
            }
            "list_builds" => reply_result(service::builds::list_builds(host)),
            "build_plan" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                id: i64,
                }
                let Args { id } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::builds::build_plan(host, id))
            }
            "active_build" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                character: Option<String>,
                }
                let Args { character } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::builds::active_build(host, character))
            }
            "save_build" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                build: Build,
                }
                let Args { build } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::builds::save_build(host, build))
            }
            "use_build" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                id: i64,
                }
                let Args { id } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::builds::use_build(host, id))
            }
            "delete_build" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                id: i64,
                }
                let Args { id } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::builds::delete_build(host, id))
            }
            "export_build" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                id: i64,
                }
                let Args { id } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::builds::export_build(host, id))
            }
            "import_build" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                code: String,
                character: Option<String>,
                }
                let Args { code, character } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::builds::import_build(host, code, character))
            }
            "build_item_fit" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                requirements: Requirements,
                jobs: Vec<ItemJob>,
                item_type: Option<String>,
                level: Option<u32>,
                character: Option<String>,
                }
                let Args { requirements, jobs, item_type, level, character } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::builds::build_item_fit(host, requirements, jobs, item_type, level, character))
            }
            _ => return None,
        })
    }
}

mod damage {
    use leafline_core::service;
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

    use crate::reply::{Reply, parse, reply, reply_result};

    pub fn dispatch(host: &dyn Host, name: &str, args: serde_json::Value) -> Option<Reply> {
        Some(match name {
            "player_damage" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                name: String,
                }
                let Args { name } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply(service::damage::player_damage(host, name))
            }
            _ => return None,
        })
    }
}

mod mapdata {
    use leafline_core::service;
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

    use crate::reply::{Reply, parse, reply, reply_result};

    pub fn dispatch(host: &dyn Host, name: &str, args: serde_json::Value) -> Option<Reply> {
        Some(match name {
            "map_data_status" => reply_result(service::mapdata::map_data_status(host)),
            "download_map_data" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                force: bool,
                }
                let Args { force } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::mapdata::download_map_data(host, force))
            }
            "find_map" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                name: String,
                street: Option<String>,
                twin: Option<u32>,
                }
                let Args { name, street, twin } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply(service::mapdata::find_map(host, name, street, twin))
            }
            "map_card" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                id: u32,
                }
                let Args { id } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply(service::mapdata::map_card(host, id))
            }
            "search_maps" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                text: String,
                }
                let Args { text } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply(service::mapdata::search_maps(host, text))
            }
            "map_route" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                from: u32,
                to: u32,
                }
                let Args { from, to } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply(service::mapdata::map_route(host, from, to))
            }
            "travel_info" => reply(service::mapdata::travel_info(host)),
            "download_travel" => reply_result(service::mapdata::download_travel(host)),
            "monster_drops" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                monsters: Vec<String>,
                }
                let Args { monsters } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::mapdata::monster_drops(host, monsters))
            }
            "quest_guide" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                quests: Vec<QuestQuery>,
                map_name: Option<String>,
                map_street: Option<String>,
                map_twin: Option<u32>,
                measured: Vec<MeasuredRate>,
                }
                let Args { quests, map_name, map_street, map_twin, measured } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::mapdata::quest_guide(host, quests, map_name, map_street, map_twin, measured))
            }
            "item_origin" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                id: u32,
                map_name: Option<String>,
                map_street: Option<String>,
                map_twin: Option<u32>,
                }
                let Args { id, map_name, map_street, map_twin } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::mapdata::item_origin(host, id, map_name, map_street, map_twin))
            }
            "search_quests" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                search: QuestSearch,
                }
                let Args { search } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply(service::mapdata::search_quests(host, search))
            }
            "quest_regions" => reply(service::mapdata::quest_regions(host)),
            "quest_sheet" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                id: u32,
                map_name: Option<String>,
                map_street: Option<String>,
                map_twin: Option<u32>,
                }
                let Args { id, map_name, map_street, map_twin } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::mapdata::quest_sheet(host, id, map_name, map_street, map_twin))
            }
            "search_monsters" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                query: MonsterQuery,
                }
                let Args { query } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::mapdata::search_monsters(host, query))
            }
            "monster_filters" => reply(service::mapdata::monster_filters(host)),
            "find_monster" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                name: String,
                }
                let Args { name } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply(service::mapdata::find_monster(host, name))
            }
            "monster_sheet" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                id: u32,
                map_name: Option<String>,
                map_street: Option<String>,
                map_twin: Option<u32>,
                measured: Vec<MeasuredRate>,
                }
                let Args { id, map_name, map_street, map_twin, measured } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::mapdata::monster_sheet(host, id, map_name, map_street, map_twin, measured))
            }
            "browse_maps" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                query: MapQuery,
                }
                let Args { query } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply(service::mapdata::browse_maps(host, query))
            }
            "map_regions" => reply(service::mapdata::map_regions(host)),
            "map_areas" => reply(service::mapdata::map_areas(host)),
            "search_npcs" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                query: NpcQuery,
                }
                let Args { query } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply(service::mapdata::search_npcs(host, query))
            }
            "find_npc" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                name: String,
                }
                let Args { name } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply(service::mapdata::find_npc(host, name))
            }
            "npc_sheet" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                id: u32,
                map_name: Option<String>,
                map_street: Option<String>,
                map_twin: Option<u32>,
                }
                let Args { id, map_name, map_street, map_twin } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply(service::mapdata::npc_sheet(host, id, map_name, map_street, map_twin))
            }
            "download_drops" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                retry: Option<bool>,
                }
                let Args { retry } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::mapdata::download_drops(host, retry))
            }
            "drops_fetched_at" => reply_result(service::mapdata::drops_fetched_at(host)),
            "training_spots" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                map_name: Option<String>,
                map_street: Option<String>,
                map_twin: Option<u32>,
                measured: Vec<MeasuredRate>,
                level: Option<u32>,
                assumptions: Option<Assumptions>,
                character: Option<String>,
                skill: Option<String>,
                }
                let Args { map_name, map_street, map_twin, measured, level, assumptions, character, skill } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply(service::mapdata::training_spots(host, map_name, map_street, map_twin, measured, level, assumptions, character, skill))
            }
            "shop_prices" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                ids: Vec<u32>,
                }
                let Args { ids } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply(service::mapdata::shop_prices(host, ids))
            }
            "monster_fight" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                id: u32,
                character: Option<String>,
                level: Option<u32>,
                measured: Vec<MeasuredRate>,
                assumptions: Option<Assumptions>,
                skill: Option<String>,
                }
                let Args { id, character, level, measured, assumptions, skill } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply(service::mapdata::monster_fight(host, id, character, level, measured, assumptions, skill))
            }
            "check_estimates" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                measured: Vec<MeasuredRate>,
                level: Option<u32>,
                assumptions: Option<Assumptions>,
                character: Option<String>,
                skill: Option<String>,
                }
                let Args { measured, level, assumptions, character, skill } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply(service::mapdata::check_estimates(host, measured, level, assumptions, character, skill))
            }
            _ => return None,
        })
    }
}

mod quests {
    use leafline_core::service;
    use leafline_core::service::quests::*;
    use leafline_core::service::host::{Host, emit};
    use leafline_core::error::Result;
    use leafline_core::mapdata::questlog::RegionTotal;
    use leafline_core::mapdata::wholelog::WholeLog;
    use leafline_core::userdata::questlog::QuestLog;

    use crate::reply::{Reply, parse, reply, reply_result};

    pub fn dispatch(host: &dyn Host, name: &str, args: serde_json::Value) -> Option<Reply> {
        Some(match name {
            "quest_log" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                name: Option<String>,
                }
                let Args { name } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::quests::quest_log(host, name))
            }
            "forget_quest_log" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                name: String,
                }
                let Args { name } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::quests::forget_quest_log(host, name))
            }
            "quest_region_totals" => reply(service::quests::quest_region_totals(host)),
            "whole_quest_log" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                name: Option<String>,
                level: Option<u32>,
                helper: Vec<String>,
                }
                let Args { name, level, helper } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::quests::whole_quest_log(host, name, level, helper))
            }
            _ => return None,
        })
    }
}

mod records {
    use leafline_core::service;
    use leafline_core::service::records::*;
    use std::path::Path;
    use std::sync::{Mutex, MutexGuard};
    use serde::{Deserialize, Serialize};
    use leafline_core::service::bag::BagCheck;
    use leafline_core::service::host::{Host, emit};
    use leafline_core::error::{Error, Result};
    use leafline_core::session::SessionSummary;
    use leafline_core::userdata::{RecordsSummary, UserData};
    use leafline_core::userdata::characters::{KnownCharacter, SeenCharacter};
    use leafline_core::userdata::drops::{LoggedDrop, NewDrop};
    use leafline_core::userdata::notes::{ItemNote, MapNote};
    use leafline_core::userdata::potions::{PotionSlot, SessionPotion};
    use leafline_core::userdata::sessions::{LevelExp, MapRate, SavedSession, SessionTimeline};
    use leafline_core::userdata::spots::SavedSpot;
    use leafline_core::clock::now;

    use crate::reply::{Reply, parse, reply, reply_result};

    pub fn dispatch(host: &dyn Host, name: &str, args: serde_json::Value) -> Option<Reply> {
        Some(match name {
            "known_character" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                name: Option<String>,
                }
                let Args { name } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::records::known_character(host, name))
            }
            "known_characters" => reply_result(service::records::known_characters(host)),
            "known_bags" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                name: Option<String>,
                }
                let Args { name } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::records::known_bags(host, name))
            }
            "saved_spots" => reply_result(service::records::saved_spots(host)),
            "set_spot_saved" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                map_id: i64,
                name: String,
                saved: bool,
                }
                let Args { map_id, name, saved } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::records::set_spot_saved(host, map_id, name, saved))
            }
            "loot_prices" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                ids: Vec<u32>,
                }
                let Args { ids } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::records::loot_prices(host, ids))
            }
            "potion_slots" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                character: Option<String>,
                }
                let Args { character } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::records::potion_slots(host, character))
            }
            "set_potion_slots" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                character: Option<String>,
                slots: Vec<PotionSlot>,
                }
                let Args { character, slots } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::records::set_potion_slots(host, character, slots))
            }
            "session_potions" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                started_at: f64,
                }
                let Args { started_at } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::records::session_potions(host, started_at))
            }
            "set_session_potion" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                character: Option<String>,
                started_at: f64,
                slot: u32,
                start: Option<i64>,
                now: Option<i64>,
                }
                let Args { character, started_at, slot, start, now } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::records::set_session_potion(host, character, started_at, slot, start, now))
            }
            "item_note" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                item: String,
                }
                let Args { item } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::records::item_note(host, item))
            }
            "set_item_note" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                item: String,
                note: String,
                }
                let Args { item, note } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::records::set_item_note(host, item, note))
            }
            "map_note" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                map_id: i64,
                }
                let Args { map_id } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::records::map_note(host, map_id))
            }
            "set_map_note" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                map_id: i64,
                map: String,
                note: String,
                }
                let Args { map_id, map, note } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::records::set_map_note(host, map_id, map, note))
            }
            "drop_log" => reply_result(service::records::drop_log(host)),
            "log_drop" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                drop: NewDrop,
                }
                let Args { drop } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::records::log_drop(host, drop))
            }
            "delete_drop" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                id: i64,
                }
                let Args { id } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::records::delete_drop(host, id))
            }
            "session_history" => reply_result(service::records::session_history(host)),
            "level_exp" => reply_result(service::records::level_exp(host)),
            "session_timeline" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                id: i64,
                }
                let Args { id } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::records::session_timeline(host, id))
            }
            "delete_session" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                id: i64,
                }
                let Args { id } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::records::delete_session(host, id))
            }
            "set_session_note" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                id: i64,
                note: String,
                }
                let Args { id, note } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::records::set_session_note(host, id, note))
            }
            "rename_map" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                from: String,
                to: String,
                }
                let Args { from, to } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::records::rename_map(host, from, to))
            }
            _ => return None,
        })
    }
}

mod sheet {
    use leafline_core::service;
    use leafline_core::service::sheet::*;
    use serde::{Deserialize, Serialize};
    use leafline_core::service::damage::PlayerDamage;
    use leafline_core::service::host::Host;
    use leafline_core::build::{Build, Catalog};
    use leafline_core::mapdata::osms::{Effects, SkillRecord};
    use leafline_core::readers::character::{DetailBonus, DetailWindow, StatWindow};

    use crate::reply::{Reply, parse, reply, reply_result};

    pub fn dispatch(host: &dyn Host, name: &str, args: serde_json::Value) -> Option<Reply> {
        Some(match name {
            "character_stats" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                name: String,
                }
                let Args { name } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply(service::sheet::character_stats(host, name))
            }
            "set_buff" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                name: String,
                skill: u32,
                on: bool,
                }
                let Args { name, skill, on } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::sheet::set_buff(host, name, skill, on))
            }
            _ => return None,
        })
    }
}

mod typed {
    use leafline_core::service;
    use leafline_core::service::typed::*;
    use serde::{Deserialize, Serialize};
    use leafline_core::service::host::{Host, emit};
    use leafline_core::service::records::HISTORY_EVENT;
    use leafline_core::clock::now_f64;
    use leafline_core::error::{Error, Result};
    use leafline_core::session::{MapSummary, SessionLine, SessionSummary};

    use crate::reply::{Reply, parse, reply, reply_result};

    pub fn dispatch(host: &dyn Host, name: &str, args: serde_json::Value) -> Option<Reply> {
        Some(match name {
            "typed_session_outcome" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                session: TypedSession,
                }
                let Args { session } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::typed::typed_session_outcome(host, session))
            }
            "save_typed_session" => {
                #[derive(serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Args {
                session: TypedSession,
                }
                let Args { session } = match parse(args) {
                    Ok(args) => args,
                    Err(err) => return Some(Err(err)),
                };
                reply_result(service::typed::save_typed_session(host, session))
            }
            _ => return None,
        })
    }
}
