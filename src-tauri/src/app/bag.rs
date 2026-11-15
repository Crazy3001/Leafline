//! Sell or keep, on a hotkey: one picture of the game, the Item Inventory's open tab read, each
//! item named by its picture and given what says to keep it. The window marks the rest to sell
//! and adds up what the bag is worth at NPC prices.
//!
//! Pictures come from OSMS through the picture cache, downloaded the first time and kept on this
//! PC. A check tries the items a character near this level most likely carries, plus every item
//! of the tab whose picture is kept already; "Look further" tries every item of the tab, which
//! may download many pictures, so the player asks for it.

use std::collections::{BTreeSet, HashSet};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use super::mapdata::MapDataState;
use super::pictures::PictureState;
use super::readings;
use super::records::now;
use super::settings::SettingsState;
use super::{builds, live, overlay, report};
use crate::error::{Error, Result};
use crate::mapdata::bag;
use crate::readers::inventory::{self, Inventory, Tab};

/// Emitted to every window with each check's result, and with `busy` while one runs.
pub const BAG_EVENT: &str = "bag";

#[derive(Default)]
pub struct BagState {
    busy: AtomicBool,
    /// The last tab read, kept in memory only, for "Look further".
    last: Mutex<Option<Read>>,
    result: Mutex<Option<BagCheck>>,
}

#[derive(Clone)]
struct Read {
    inventory: Inventory,
    character: Option<String>,
    level: Option<u32>,
}

/// What a check found.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BagCheck {
    /// A check is under way; the rest is from the one before.
    pub busy: bool,
    /// Unix seconds.
    pub at: i64,
    pub tab: Option<Tab>,
    pub character: Option<String>,
    pub items: Vec<BagItem>,
    /// Slots holding something no picture matched.
    pub unknown: Vec<u32>,
    /// Every item of the tab was tried.
    pub wide: bool,
    /// Pictures "Look further" would still download.
    pub more: usize,
    pub problem: Option<String>,
}

/// One slot's item.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BagItem {
    pub slot: u32,
    pub count: u32,
    /// Every item drawn the same, best first. More than one when the picture can't tell them apart.
    pub ids: Vec<u32>,
    pub name: String,
    /// What an NPC pays for one, from the item data, when the items drawn alike agree. What the
    /// data's price means in Classic isn't confirmed.
    pub price: Option<i64>,
    /// The lowest and highest when they don't agree.
    pub price_range: Option<(i64, i64)>,
    /// Can't be traded or dropped: an NPC may still buy it.
    pub untradable: bool,
    /// What says to keep it, from the data and the build.
    pub keep: Vec<Keep>,
    /// Quests that ask for it, for the window to check against the ones being done.
    pub wanted_by: Vec<QuestAsk>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestAsk {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Keep {
    /// A crafting recipe uses it.
    Recipe,
    /// Gear the build will wear.
    Plan,
    /// The data marks it a quest item.
    Quest,
    /// A scroll: worth more to players than to an NPC, as a rule.
    Scroll,
}

/// The hotkey: reads the open tab from one picture of the game. The Settings switch for live
/// reading covers this too.
pub fn check(app: &AppHandle) -> Result<()> {
    if !app.state::<SettingsState>().get().live.enabled {
        return super::settings::open(app);
    }
    start(app, None)
}

/// "Look further": tries every item of the tab last read.
#[tauri::command]
pub fn bag_look_further(app: AppHandle) -> Result<()> {
    let last = app.state::<BagState>().last.lock().expect("bag lock").clone();
    let Some(read) = last else {
        return Err(Error::Invalid("Check your bag first.".into()));
    };
    start(&app, Some(read))
}

/// The last check, for a window that opens after it.
#[tauri::command]
pub fn last_bag(state: State<BagState>) -> Option<BagCheck> {
    state.result.lock().expect("bag lock").clone()
}

fn start(app: &AppHandle, again: Option<Read>) -> Result<()> {
    let state = app.state::<BagState>();
    if state.busy.swap(true, Ordering::SeqCst) {
        return Ok(());
    }
    let busy = BagCheck { busy: true, ..state.result.lock().expect("bag lock").clone().unwrap_or_default() };
    let _ = app.emit(BAG_EVENT, &busy);
    let handle = app.clone();
    std::thread::Builder::new().name("bag-check".into()).spawn(move || {
        let wide = again.is_some();
        let result = (|| {
            let read = match again {
                Some(read) => read,
                None => read_bag(&handle)?,
            };
            *handle.state::<BagState>().last.lock().expect("bag lock") = Some(read.clone());
            name_items(&handle, &read, wide)
        })();
        let check = result.unwrap_or_else(|err| {
            report::note(format!("Couldn't check the bag: {err}"));
            BagCheck { at: now(), problem: Some(err.to_string()), ..BagCheck::default() }
        });
        let state = handle.state::<BagState>();
        *state.result.lock().expect("bag lock") = Some(check.clone());
        state.busy.store(false, Ordering::SeqCst);
        if let Err(err) = overlay::show_full(&handle) {
            report::note(err.to_string());
        }
        let _ = handle.emit(BAG_EVENT, &check);
    })?;
    Ok(())
}

/// Loads the pictures, downloading the missing ones a few at a time (the cache keeps to its own
/// limit). One that can't be had is left out.
fn load_pictures(pictures: &PictureState, wanted: Vec<(u32, String)>) -> Vec<(u32, image::RgbaImage)> {
    const WORKERS: usize = crate::mapdata::pictures::MAX_DOWNLOADS;
    let chunk = wanted.len().div_ceil(WORKERS).max(1);
    std::thread::scope(|scope| {
        let jobs: Vec<_> = wanted
            .chunks(chunk)
            .map(|part| {
                scope.spawn(move || part.iter().filter_map(|(id, url)| Some((*id, pictures.image(url).ok()?))).collect::<Vec<_>>())
            })
            .collect();
        jobs.into_iter().flat_map(|job| job.join().unwrap_or_default()).collect()
    })
}

fn read_bag(app: &AppHandle) -> Result<Read> {
    let frame = live::one_picture(app)?;
    let inventory = inventory::read(&frame)
        .ok_or_else(|| Error::Invalid("No Item Inventory in view. Open it on the tab to check, then press the hotkey.".into()))?;
    let analysis = readings::current_analysis(app);
    let hud = analysis.as_ref().and_then(|a| a.hud.as_ref());
    let session = readings::current_session(app);
    Ok(Read {
        inventory,
        character: hud.and_then(|h| h.name.clone()).or_else(|| session.as_ref().and_then(|s| s.name.clone())),
        level: hud.and_then(|h| h.level).or_else(|| session.as_ref().and_then(|s| s.level)),
    })
}

fn name_items(app: &AppHandle, read: &Read, wide: bool) -> Result<BagCheck> {
    if !app.state::<SettingsState>().get().show_pictures {
        return Err(Error::Invalid(
            "Sell or keep names items by their pictures. Turn on Show game pictures in Settings → Data.".into(),
        ));
    }
    let Some(tab) = read.inventory.tab else {
        return Err(Error::Invalid("Leafline couldn't tell which tab is open.".into()));
    };
    if bag::kinds(tab).is_empty() {
        return Err(Error::Invalid("Cash and Deco items aren't in the item data. Open Equip, Use, Set-up or Etc.".into()));
    }
    let data = app.state::<MapDataState>();
    let (Some(atlas), Some(catalog)) = (data.atlas(), data.catalog()) else {
        return Err(Error::Invalid("Download the map data first: items are named from it.".into()));
    };
    let index = data.pictures();
    let pictures = app.state::<PictureState>();
    let url = |id: u32| index.resolve(&format!("/item/{id}")).map(str::to_string);
    let planned = read.level.map_or_else(HashSet::new, |level| builds::planned_gear(app, read.character.as_deref(), level));
    let every = bag::every(&catalog, tab);
    let mut ids: BTreeSet<u32> = if wide {
        every.iter().copied().collect()
    } else {
        let extra: Vec<u32> = planned.iter().copied().collect();
        let dropped = data.dropped_pairs().unwrap_or_default();
        bag::likely(&atlas, &catalog, &dropped, read.level, tab, &extra).into_iter().collect()
    };
    // Pictures already on this PC cost nothing to try.
    ids.extend(every.iter().copied().filter(|&id| url(id).is_some_and(|u| pictures.has(&u))));
    let images = load_pictures(&pictures, ids.iter().filter_map(|&id| Some((id, url(id)?))).collect());
    let candidates: Vec<(u32, &image::RgbaImage)> = images.iter().map(|(id, image)| (*id, image)).collect();
    let more = every.iter().filter(|&&id| url(id).is_some_and(|u| !pictures.has(&u))).count();

    let facts = bag::KeepFacts::new(&atlas, &catalog, planned);
    let mut check = BagCheck { at: now(), tab: Some(tab), character: read.character.clone(), wide, more, ..BagCheck::default() };
    for slot in &read.inventory.slots {
        let Some(named) = bag::name(&slot.picture, &candidates) else {
            check.unknown.push(slot.index);
            continue;
        };
        let records: Vec<_> = named.ids.iter().filter_map(|&id| catalog.item(id)).collect();
        let Some(first) = records.first() else { continue };
        let prices: Vec<i64> = records.iter().filter_map(|r| r.price).collect();
        let (low, high) = (prices.iter().min().copied(), prices.iter().max().copied());
        let any = |test: &dyn Fn(u32) -> bool| named.ids.iter().any(|&id| test(id));
        let mut keep = Vec::new();
        if any(&|id| facts.in_recipes.contains(&id)) {
            keep.push(Keep::Recipe);
        }
        if any(&|id| facts.planned.contains(&id)) {
            keep.push(Keep::Plan);
        }
        if records.iter().any(|r| r.quest_item) {
            keep.push(Keep::Quest);
        }
        if records.iter().all(|r| r.kind == crate::mapdata::osms::ItemKind::Scroll) {
            keep.push(Keep::Scroll);
        }
        check.items.push(BagItem {
            slot: slot.index,
            count: slot.count.unwrap_or(1),
            ids: named.ids.clone(),
            name: first.name.clone(),
            price: (low == high).then_some(low).flatten(),
            price_range: low.zip(high).filter(|(l, h)| l != h),
            untradable: records.iter().all(|r| r.untradable),
            keep,
            wanted_by: named
                .ids
                .iter()
                .flat_map(|id| facts.wanted_by.get(id).into_iter().flatten())
                .map(|(id, name)| QuestAsk { id: *id, name: name.clone() })
                .collect(),
        });
    }
    Ok(check)
}
