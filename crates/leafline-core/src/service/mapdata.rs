//! Map data commands: downloading the cache, the map card, routes and drops.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use serde::Serialize;

use super::fighter::{self, Choice};
use super::files::{Files, write_json};
use super::host::{Host, emit};
use crate::mapdata::combat::{self, Fighter, Fit, Tuning};
use crate::mapdata::training::EstimateCheck;
use crate::build::Catalog;
use crate::build::catalog::item_group;
use crate::mapdata::osms::ItemKind;
use crate::error::{Error, Result};
use crate::mapdata::fetch::{self, Source};
pub use crate::clock::now;
use crate::mapdata::osms::Dataset;
use crate::mapdata::pictures::PictureIndex;
use crate::mapdata::layout::{self, Layouts};
use crate::mapdata::store::{DROPS_FAILED, LAYOUTS_FAILED, TURN_INS_FAILED};
use crate::mapdata::{
    Assumptions, Atlas, DataStatus, ItemOrigin, KillSpeed, MapCard, MapMatch, MapPage, MapQuery, MapRef, MeasuredRate,
    MonsterDrops, MonsterFilters, MonsterPage, MonsterQuery, MonsterSheet, NpcPage, NpcQuery, NpcSheet, QuestGuide, QuestPage, QuestQuery, QuestSearch,
    QuestSheet, RefreshReport, Route, Store, TrainingSpot, Travel, TravelFile, drops, shops, turnins,
};

pub const FILE_NAME: &str = "map-data.sqlite3";
/// The player's copy of the travel connections, which replaces the downloaded one when present.
pub const TRAVEL_FILE: &str = "travel.json";
/// The travel connections downloaded from mapleclassic.wiki.
pub const TRAVEL_CACHE: &str = "travel-wiki.json";

/// Emitted to every window when the cached map data changes.
pub const MAP_DATA_EVENT: &str = "map-data";
/// Emitted to every window to bring up a quest in the Quests tab.
pub const SHOW_QUEST_EVENT: &str = "show-quest";
/// Emitted to every window to bring up a monster in the Lookup tab.
pub const SHOW_MONSTER_EVENT: &str = "show-monster";
/// Emitted to every window to bring up a map or an NPC in the Lookup tab.
pub const SHOW_PLACE_EVENT: &str = "show-place";

/// The map data, and the skills and equipment that came with it.
fn split(mut data: Dataset) -> (Arc<Atlas>, Arc<Catalog>) {
    // The Atlas keeps the item names too, for the items quests ask to bring.
    let item_names = data.items.iter().map(|i| (i.id, i.name.clone())).collect();
    let genders: Vec<(u32, String, Option<String>)> =
        data.items.iter().map(|i| (i.id, i.name.clone(), i.gender.clone())).collect();
    let catalog = Catalog::new(std::mem::take(&mut data.skills), std::mem::take(&mut data.items))
        .with_recipes(std::mem::take(&mut data.recipes));
    let atlas = Atlas::new(data)
        .with_item_names(item_names)
        .with_item_genders(genders.iter().map(|(id, name, gender)| (*id, name.as_str(), gender.as_deref())));
    (Arc::new(atlas), Arc::new(catalog))
}

pub struct MapDataState {
    path: PathBuf,
    atlas: RwLock<Option<Arc<Atlas>>>,
    catalog: RwLock<Option<Arc<Catalog>>>,
    /// Where each picture the data names is published, for the picture protocol.
    pictures: RwLock<Arc<PictureIndex>>,
    /// Every map's layout, from Leafline's data repository. Empty until downloaded.
    layouts: RwLock<Arc<Layouts>>,
    downloading: AtomicBool,
    osms: Box<dyn Source + Send + Sync>,
    /// Shared so every wiki request (travel) keeps to its one-a-second pace.
    wiki: Box<dyn Source + Send + Sync>,
    /// One drop table download at a time; whoever waits then finds the table fresh.
    drops_refresh: Mutex<()>,
}

impl MapDataState {
    /// Opens the cache at `path`. Nothing is downloaded here. `osms` fetches OSMS and Leafline's
    /// own data files; `wiki` fetches mapleclassic.wiki.
    pub fn load(path: PathBuf, osms: Box<dyn Source + Send + Sync>, wiki: Box<dyn Source + Send + Sync>) -> Self {
        // A damaged cache must not stop the app; downloading again rebuilds it.
        let mut pictures = PictureIndex::default();
        let (atlas, catalog) = match Store::open(&path).and_then(|store| store.load()) {
            Ok(data) => {
                if let Some(data) = &data {
                    pictures = PictureIndex::new(data);
                }
                data.map(split).unzip()
            }
            Err(err) => {
                eprintln!("map data cache unreadable: {err}");
                (None, None)
            }
        };
        let layouts = Store::open(&path).and_then(|store| store.layouts()).unwrap_or_default();
        Self {
            path,
            atlas: RwLock::new(atlas),
            catalog: RwLock::new(catalog),
            pictures: RwLock::new(Arc::new(pictures)),
            layouts: RwLock::new(Arc::new(layouts)),
            downloading: AtomicBool::new(false),
            osms,
            wiki,
            drops_refresh: Mutex::new(()),
        }
    }

    /// The cache file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Whether the cache file is there yet. The website's storage has no paths to ask, and opening
    /// its cache costs nothing, so it always says yes.
    fn cache_exists(&self) -> bool {
        cfg!(target_arch = "wasm32") || self.path.exists()
    }

    pub fn layouts(&self) -> Arc<Layouts> {
        self.layouts.read().expect("layouts lock").clone()
    }

    /// Renews the map layouts, once a day at most, from Leafline's data repository.
    pub fn refresh_layouts(&self) -> Result<()> {
        let _turn = self.drops_refresh.lock().expect("drops refresh lock");
        let mut store = Store::open(&self.path)?;
        if layout::refresh(&mut store, &*self.osms, now())? {
            *self.layouts.write().expect("layouts lock") = Arc::new(store.layouts()?);
        }
        Ok(())
    }

    /// Every (monster, item) pair the drop list holds, by name.
    pub fn dropped_pairs(&self) -> Result<Vec<(String, String)>> {
        Store::open(&self.path)?.dropped_pairs()
    }

    pub fn atlas(&self) -> Option<Arc<Atlas>> {
        self.atlas.read().expect("atlas lock").clone()
    }

    /// A map's portals, NPCs and spawn points, read from the cache when asked.
    pub fn map_points(&self, id: u32) -> Result<Vec<crate::mapdata::osms::MapPoint>> {
        Store::open(&self.path)?.map_points(id)
    }

    pub fn pictures(&self) -> Arc<PictureIndex> {
        self.pictures.read().expect("pictures lock").clone()
    }

    pub fn catalog(&self) -> Option<Arc<Catalog>> {
        self.catalog.read().expect("catalog lock").clone()
    }

    pub fn refresh(&self, force: bool) -> Result<RefreshReport> {
        if self.downloading.swap(true, Ordering::SeqCst) {
            return Err(Error::Invalid("Map data is already being checked. Try again in a moment.".into()));
        }
        let result = (|| {
            let mut store = Store::open(&self.path)?;
            let report = fetch::refresh(&mut store, &*self.osms, force, now())?;
            if !report.downloaded.is_empty() || self.atlas().is_none() {
                let data = store.load()?;
                if let Some(data) = &data {
                    *self.pictures.write().expect("pictures lock") = Arc::new(PictureIndex::new(data));
                }
                let (atlas, catalog) = data.map(split).unzip();
                *self.atlas.write().expect("atlas lock") = atlas;
                *self.catalog.write().expect("catalog lock") = catalog;
            }
            Ok(report)
        })();
        self.downloading.store(false, Ordering::SeqCst);
        result
    }

    /// Renews the table of who takes each quest in, once a day at most, from Leafline's data
    /// repository on GitHub, and brings the atlas up to date with it.
    pub fn refresh_turn_ins(&self) -> Result<()> {
        let _turn = self.drops_refresh.lock().expect("drops refresh lock");
        let mut store = Store::open(&self.path)?;
        if turnins::refresh(&mut store, &*self.osms, now())? {
            let (atlas, catalog) = store.load()?.map(split).unzip();
            *self.atlas.write().expect("atlas lock") = atlas;
            *self.catalog.write().expect("catalog lock") = catalog;
        }
        Ok(())
    }

    /// Renews Leafline's shop file once a day, and brings the atlas up to date with it.
    pub fn refresh_shops(&self) -> Result<()> {
        let _turn = self.drops_refresh.lock().expect("drops refresh lock");
        let mut store = Store::open(&self.path)?;
        if shops::refresh(&mut store, &*self.osms, now())? {
            let (atlas, catalog) = store.load()?.map(split).unzip();
            *self.atlas.write().expect("atlas lock") = atlas;
            *self.catalog.write().expect("catalog lock") = catalog;
        }
        Ok(())
    }

    pub fn refresh_drops(&self) -> Result<()> {
        let _turn = self.drops_refresh.lock().expect("drops refresh lock");
        drops::refresh(&mut Store::open(&self.path)?, &*self.osms, now())
    }

    /// Forgets a failed download (see [`drops::RETRY_AFTER`]), so the next refresh asks at
    /// once. Only for the player's own request: automatic lookups keep waiting.
    pub fn retry(&self, failed: &str) -> Result<()> {
        Store::open(&self.path)?.set_failure(failed, None)
    }
}

/// Whether there is map data to keep up to date: downloaded already, or asked for.
pub fn wants_updates(state: &MapDataState) -> bool {
    let wanted = Store::open(&state.path).and_then(|s| s.wants_maps()).unwrap_or(false);
    state.atlas().is_some() || wanted
}

/// Checks OSMS for changes, at most once a day, and renews the turn-in table, layouts, shop file
/// and drop list once they are a day old. Returns the new status when map data changed. Failures
/// are logged: the cache keeps working, and the player can retry.
pub fn update_in_background(state: &MapDataState) -> Option<DataStatus> {
    let mut changed = None;
    match state.refresh(false) {
        Ok(report) if !report.downloaded.is_empty() => {
            changed = Store::open(&state.path).and_then(|s| s.status()).ok();
        }
        Ok(_) => {}
        Err(err) => eprintln!("map data update check failed: {err}"),
    }
    if let Err(err) = state.refresh_turn_ins() {
        eprintln!("turn-in table download failed: {err}");
    }
    // Estimates and routes work without layouts, just more roughly.
    if let Err(err) = state.refresh_layouts() {
        eprintln!("map layout download failed: {err}");
    }
    // The copy that came with the app keeps working when this fails.
    if let Err(err) = state.refresh_shops() {
        eprintln!("shop file download failed: {err}");
    }
    if let Err(err) = state.refresh_drops() {
        eprintln!("drop table update failed: {err}");
    }
    changed
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Downloaded {
    pub report: RefreshReport,
    pub status: DataStatus,
}

pub fn map_data_status(h: &dyn Host) -> Result<DataStatus> {
    Store::open(&h.map().path)?.status()
}

/// The explicit "Download map data" action: fetches whatever changed on OSMS. Without `force`,
/// does nothing if the last check was under a day ago.
pub fn download_map_data(h: &dyn Host, force: bool) -> Result<Downloaded> {
    let state = h.map();
    let report = state.refresh(force)?;
    // Quest help still works without it, from what the map data shows. The player asked, so
    // a recent failure is tried again.
    state.retry(TURN_INS_FAILED)?;
    if let Err(err) = state.refresh_turn_ins() {
        eprintln!("turn-in table download failed: {err}");
    }
    state.retry(LAYOUTS_FAILED)?;
    if let Err(err) = state.refresh_layouts() {
        eprintln!("map layout download failed: {err}");
    }
    if let Err(err) = state.refresh_shops() {
        eprintln!("shop file download failed: {err}");
    }
    let downloaded = Downloaded { report, status: Store::open(&h.map().path)?.status()? };
    emit(h, MAP_DATA_EVENT, &downloaded.status);
    Ok(downloaded)
}

/// The card for the map a minimap header names, matched fuzzily against the cached names.
pub fn find_map(
    h: &dyn Host,
    name: String,
    street: Option<String>,
    twin: Option<u32>,
) -> Option<MapMatch> {
    h.map().atlas()?.find_twin(&name, street.as_deref(), twin)
}

pub fn map_card(h: &dyn Host, id: u32) -> Option<MapCard> {
    h.map().atlas()?.card(id)
}

pub fn search_maps(h: &dyn Host, text: String) -> Vec<MapRef> {
    h.map().atlas().map(|atlas| atlas.search(&text, 8)).unwrap_or_default()
}

/// A route with roughly how long it takes on foot.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteInfo {
    #[serde(flatten)]
    route: Route,
    /// About how many seconds of walking, when every map on the way has a known layout.
    walk_seconds: Option<f64>,
}

/// The quickest way between two maps, through portals and any travel the player turned on,
/// timed by the walk between portals. Where the data doesn't place every portal on the way, the
/// map layouts give a rougher time.
pub fn map_route(h: &dyn Host, from: u32, to: u32) -> Option<RouteInfo> {
    let atlas = h.map().atlas()?;
    let route = atlas.route_with(from, to, &travel(h, &atlas))?;
    let walk_seconds = route.walk_seconds.or_else(|| layout::route_walk_seconds(&route, &h.map().layouts()));
    Some(RouteInfo { route, walk_seconds })
}

/// Where the travel connections in use come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TravelOrigin {
    /// The player's own copy.
    Custom,
    /// Downloaded from mapleclassic.wiki.
    Wiki,
    /// Nothing yet: the player hasn't downloaded or written any.
    None,
}

/// The travel connections in use: the player's own copy if there is one and it reads, otherwise
/// the copy downloaded from the wiki, if any. Also says why the player's copy could not be used.
pub fn travel_file(files: &dyn Files) -> (Option<TravelFile>, TravelOrigin, Option<String>) {
    let read = |name: &str| {
        files.read(name).map(|bytes| TravelFile::parse(&String::from_utf8_lossy(&bytes)))
    };
    let error = match read(TRAVEL_FILE) {
        Some(Ok(file)) => return (Some(file), TravelOrigin::Custom, None),
        Some(Err(err)) => Some(err),
        None => None,
    };
    match read(TRAVEL_CACHE) {
        Some(Ok(file)) => (Some(file), TravelOrigin::Wiki, error),
        _ => (None, TravelOrigin::None, error),
    }
}

/// The rides and scrolls the player turned on in Settings, found in the map data.
fn travel(h: &dyn Host, atlas: &Atlas) -> Travel {
    let choice = h.settings().travel;
    // NPC rides are always in: some maps can't be reached without one. Taxis and scrolls wait
    // for the player's switch, which `choice` checks for each ride.
    let file = travel_file(h.files()).0.unwrap_or_else(TravelFile::empty).with_known_rides();
    atlas.travel(&file, choice).0
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TravelInfo {
    file: Option<TravelFile>,
    origin: TravelOrigin,
    /// The player's own copy, whether or not it exists yet. `None` on the website.
    path: Option<String>,
    /// Why the player's copy could not be used, if it could not.
    error: Option<String>,
    /// Map names the map data has no map for; `None` before map data is downloaded.
    missing: Option<Vec<String>>,
}

/// The travel connections, for the Settings window.
pub fn travel_info(h: &dyn Host) -> TravelInfo {
    let (file, origin, error) = travel_file(h.files());
    let missing = file.as_ref().and_then(|file| Some(h.map().atlas()?.travel(file, Default::default()).1));
    let path = h.files().path(TRAVEL_FILE).map(|p| p.display().to_string());
    TravelInfo { file, origin, path, error, missing }
}

/// Downloads the taxis and return scrolls players recorded on mapleclassic.wiki and keeps them
/// beside the settings. Only on the player's request.
pub fn download_travel(h: &dyn Host) -> Result<TravelInfo> {
    let file = TravelFile::fetch(&*h.map().wiki, now())?;
    write_json(h.files(), TRAVEL_CACHE, &file)?;
    Ok(travel_info(h))
}

/// The player's copy of the travel connections, made first from the downloaded connections, or
/// empty if there are none. Returns where it is, for the program to open in an editor.
pub fn own_travel_file(h: &dyn Host) -> Result<Option<PathBuf>> {
    if h.files().read(TRAVEL_FILE).is_none() {
        let start = match travel_file(h.files()) {
            (Some(file), TravelOrigin::Wiki, _) => file,
            _ => TravelFile::empty(),
        };
        write_json(h.files(), TRAVEL_FILE, &start)?;
    }
    Ok(h.files().path(TRAVEL_FILE))
}

/// Player-observed drops for the named monsters, from the downloaded table. The first call
/// downloads the table; the names are never sent.
pub fn monster_drops(h: &dyn Host, monsters: Vec<String>) -> Result<Vec<MonsterDrops>> {
    let state = h.map();
    state.refresh_drops()?;
    let mut drops = Store::open(&h.map().path)?.drops(&monsters)?;
    // The drop list files everything as `etc`; the item data knows each item's kind, so the
    // lists can group equips, use items and the rest.
    if let Some(catalog) = h.map().catalog() {
        for drop in drops.iter_mut().flat_map(|m| m.drops.iter_mut()) {
            if let Some(item) = catalog.item_called(&drop.item) {
                drop.category = Some(item.kind.name().to_string());
                drop.item_id = Some(item.id);
                drop.item_type = item_group(item).map(str::to_string).or_else(|| match item.kind {
                    ItemKind::Scroll => Some("Scroll".into()),
                    _ => Some(item.weapon_type.clone().unwrap_or_else(|| item.slot.clone())),
                });
                drop.req_level = Some(item.req_level).filter(|&l| l > 0);
            }
        }
    }
    Ok(drops)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Guide {
    /// The map the guide counts portals from, when the minimap was read and matched.
    from: Option<MapRef>,
    quests: Vec<QuestGuide>,
}

/// Where to go for each quest the Quest Helper shows. Item objectives use the downloaded drop
/// table; nothing is fetched here. `None` until map data has been downloaded.
pub fn quest_guide(h: &dyn Host,
    quests: Vec<QuestQuery>,
    map_name: Option<String>,
    map_street: Option<String>,
    map_twin: Option<u32>,
    measured: Vec<MeasuredRate>,
) -> Result<Option<Guide>> {
    let Some(atlas) = h.map().atlas() else { return Ok(None) };
    let from = map_name.and_then(|name| atlas.find_twin(&name, map_street.as_deref(), map_twin)).map(|m| m.card.map);
    let objectives: Vec<String> = quests.iter().flat_map(|q| q.objectives.iter().cloned()).collect();
    let items = drops::sources_for(&Store::open(&h.map().path)?, &objectives)?;
    let quests = atlas.quest_guide(&quests, from.as_ref().map(|m| m.id), &items, &measured, &travel(h, &atlas));
    Ok(Some(Guide { from, quests }))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemWhere {
    /// The map portals are counted from, when the minimap was read and matched.
    from: Option<MapRef>,
    origin: ItemOrigin,
}

/// Who drops an item and which quests give it. The first call downloads the drop table; the
/// item's name is never sent. Without that table, the quests still show.
pub fn item_origin(h: &dyn Host,
    id: u32,
    map_name: Option<String>,
    map_street: Option<String>,
    map_twin: Option<u32>,
) -> Result<Option<ItemWhere>> {
    let state = h.map();
    let (Some(atlas), Some(catalog)) = (h.map().atlas(), h.map().catalog()) else { return Ok(None) };
    let Some(item) = catalog.item(id) else { return Ok(None) };
    if let Err(err) = state.refresh_drops() {
        eprintln!("drop table download failed: {err}");
    }
    let store = Store::open(&h.map().path)?;
    let sources = drops::sources_for(&store, std::slice::from_ref(&item.name))?;
    let from = map_name.and_then(|name| atlas.find_twin(&name, map_street.as_deref(), map_twin)).map(|m| m.card.map);
    let start = from.as_ref().map(|m| m.id);
    let origin = atlas.item_origin(&catalog.copies_of(id), &item.name, sources.first(), start, &travel(h, &atlas));
    Ok(Some(ItemWhere { from, origin }))
}

pub fn search_quests(h: &dyn Host, search: QuestSearch) -> QuestPage {
    h.map().atlas().map(|atlas| atlas.search_quests(&search)).unwrap_or_default()
}

pub fn quest_regions(h: &dyn Host) -> Vec<String> {
    h.map().atlas().map(|atlas| atlas.quest_regions()).unwrap_or_default()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestWhere {
    /// The map portals are counted from, when the minimap was read and matched.
    from: Option<MapRef>,
    sheet: QuestSheet,
}

/// All the data says about a quest. Without the drop table, its items show no droppers.
pub fn quest_sheet(h: &dyn Host,
    id: u32,
    map_name: Option<String>,
    map_street: Option<String>,
    map_twin: Option<u32>,
) -> Result<Option<QuestWhere>> {
    let state = h.map();
    let (Some(atlas), Some(catalog)) = (h.map().atlas(), h.map().catalog()) else { return Ok(None) };
    let item_name = |id: u32| catalog.item(id).map(|item| item.name.clone());
    if let Err(err) = state.refresh_drops() {
        eprintln!("drop table download failed: {err}");
    }
    let items = atlas.quest_items(id, item_name);
    let sources = drops::sources_for(&Store::open(&h.map().path)?, &items)?;
    let from = map_name.and_then(|name| atlas.find_twin(&name, map_street.as_deref(), map_twin)).map(|m| m.card.map);
    let start = from.as_ref().map(|m| m.id);
    let sheet = atlas.quest_sheet(id, item_name, &sources, start, &travel(h, &atlas));
    Ok(sheet.map(|sheet| QuestWhere { from, sheet }))
}

/// One page of the monster search. Drops are matched from the downloaded table, if there is one.
pub fn search_monsters(h: &dyn Host, query: MonsterQuery) -> Result<MonsterPage> {
    let Some(atlas) = h.map().atlas() else { return Ok(MonsterPage::default()) };
    Ok(atlas.search_monsters(&query, &Store::open(&h.map().path)?.dropped_pairs()?))
}

pub fn monster_filters(h: &dyn Host) -> MonsterFilters {
    h.map().atlas().map(|atlas| atlas.monster_filters()).unwrap_or_default()
}

/// The monster a name means.
pub fn find_monster(h: &dyn Host, name: String) -> Option<u32> {
    h.map().atlas()?.monster_like(&name)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonsterWhere {
    /// The map portals are counted from, when the minimap was read and matched.
    from: Option<MapRef>,
    sheet: MonsterSheet,
}

/// All the data says about a monster. The first call downloads the drop table; the monster's
/// name is never sent. Without that table, the rest still shows.
pub fn monster_sheet(h: &dyn Host,
    id: u32,
    map_name: Option<String>,
    map_street: Option<String>,
    map_twin: Option<u32>,
    measured: Vec<MeasuredRate>,
) -> Result<Option<MonsterWhere>> {
    let state = h.map();
    let Some(atlas) = h.map().atlas() else { return Ok(None) };
    let Some(name) = atlas.monster_name(id).map(str::to_string) else { return Ok(None) };
    if let Err(err) = state.refresh_drops() {
        eprintln!("drop table download failed: {err}");
    }
    let store = Store::open(&h.map().path)?;
    let drops = if store.drops_fetched_at()?.is_some() { store.drops(&[name])?.pop() } else { None };
    let from = map_name.and_then(|name| atlas.find_twin(&name, map_street.as_deref(), map_twin)).map(|m| m.card.map);
    let start = from.as_ref().map(|m| m.id);
    let sheet = atlas.monster_sheet(id, drops, start, &measured, &travel(h, &atlas));
    Ok(sheet.map(|sheet| MonsterWhere { from, sheet }))
}

/// One page of the map search.
pub fn browse_maps(h: &dyn Host, query: MapQuery) -> MapPage {
    let layouts = h.map().layouts();
    h.map()
        .atlas()
        .map(|atlas| atlas.browse_maps_with(&query, |id| layouts.get(&id).map(|l| l.walkable)))
        .unwrap_or_default()
}

pub fn map_regions(h: &dyn Host) -> Vec<String> {
    h.map().atlas().map(|atlas| atlas.map_regions()).unwrap_or_default()
}

/// The areas Lookup narrows maps, monsters and NPCs to.
pub fn map_areas(h: &dyn Host) -> Vec<String> {
    h.map().atlas().map(|atlas| atlas.map_areas()).unwrap_or_default()
}

/// One page of the NPC search.
pub fn search_npcs(h: &dyn Host, query: NpcQuery) -> NpcPage {
    h.map().atlas().map(|atlas| atlas.search_npcs(&query)).unwrap_or_default()
}

/// The NPC a name means.
pub fn find_npc(h: &dyn Host, name: String) -> Option<u32> {
    h.map().atlas()?.npc_like(&name)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcWhere {
    /// The map portals are counted from, when the minimap was read and matched.
    from: Option<MapRef>,
    sheet: NpcSheet,
}

/// All the data says about an NPC.
pub fn npc_sheet(h: &dyn Host,
    id: u32,
    map_name: Option<String>,
    map_street: Option<String>,
    map_twin: Option<u32>,
) -> Option<NpcWhere> {
    let atlas = h.map().atlas()?;
    let from = map_name.and_then(|name| atlas.find_twin(&name, map_street.as_deref(), map_twin)).map(|m| m.card.map);
    let sheet = atlas.npc_sheet(id, from.as_ref().map(|m| m.id), &travel(h, &atlas))?;
    Some(NpcWhere { from, sheet })
}

/// Downloads Leafline's drop list if there is none or it is over a day old.
/// `retry` is set when the player pressed a button for it, so a download that failed recently is
/// tried again; panels that ask on their own leave it unset and wait out the back-off.
pub fn download_drops(h: &dyn Host, retry: Option<bool>) -> Result<()> {
    let state = h.map();
    if retry == Some(true) {
        state.retry(DROPS_FAILED)?;
    }
    state.refresh_drops()
}

/// When the drop table was last downloaded, in Unix seconds, or `None` before the first time.
pub fn drops_fetched_at(h: &dyn Host) -> Result<Option<i64>> {
    if !h.map().cache_exists() {
        return Ok(None);
    }
    Store::open(&h.map().path)?.drops_fetched_at()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Spots {
    /// The map portals are counted from, when the minimap was read and matched.
    from: Option<MapRef>,
    /// How fast the player gets through monster HP, behind every estimate. None until enough
    /// solo play is saved.
    speed: Option<KillSpeed>,
    /// The character as the combat model sees them, and how its tuning fits their play. None
    /// until their attack range (or a spell) has been read or typed.
    fighter: Option<Fighter>,
    fit: Option<Fit>,
    spots: Vec<TrainingSpot>,
}

/// Every map with monsters to hunt, for the training spot finder, with the EXP per hour the
/// player measured on each and an estimate for the rest: from the combat model when the
/// character's damage is known, else from their kill speed. `level` is the character's;
/// `assumptions` what the player set for respawn time and EXP rate; `character` whose read
/// stats to use and `skill` the skill picked. `None` until map data has been downloaded.
#[allow(clippy::too_many_arguments)]
pub fn training_spots(h: &dyn Host,
    map_name: Option<String>,
    map_street: Option<String>,
    map_twin: Option<u32>,
    measured: Vec<MeasuredRate>,
    level: Option<u32>,
    assumptions: Option<Assumptions>,
    character: Option<String>,
    skill: Option<String>,
) -> Option<Spots> {
    let atlas = h.map().atlas()?;
    let from = map_name.and_then(|name| atlas.find_twin(&name, map_street.as_deref(), map_twin)).map(|m| m.card.map);
    let layouts = h.map().layouts();
    let assumptions = assumptions.unwrap_or_default();
    let estimator = atlas.estimator(&measured, level, assumptions, &layouts);
    let choice = Choice { assumptions: Some(&assumptions), skill: skill.as_deref() };
    let fighter = character.as_deref().filter(|n| !n.trim().is_empty()).and_then(|name| fighter::fighter(h, name, level, &choice));
    let model = atlas.model(&measured, fighter, assumptions, &layouts);
    let spots = atlas.training_spots(
        from.as_ref().map(|m| m.id),
        &measured,
        &travel(h, &atlas),
        estimator.as_ref(),
        model.as_ref(),
        &layouts,
    );
    Some(Spots { from, speed: estimator.map(|e| e.speed), fighter, fit: model.map(|m| m.fit), spots })
}

/// The lowest NPC shop price of each of these items that a shop sells, for working out what
/// crafting costs. Items no shop sells are left out.
pub fn shop_prices(h: &dyn Host, ids: Vec<u32>) -> Vec<(u32, u64)> {
    let Some(atlas) = h.map().atlas() else { return Vec::new() };
    ids.into_iter().filter_map(|id| Some((id, atlas.cheapest_price(id)?))).collect()
}

/// One monster against the player's character, from the combat model.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonsterFight {
    pub fighter: Fighter,
    pub fight: combat::Fight,
    /// Seconds of attacking a kill takes, at the fighter's attack speed, nothing else counted.
    pub attack_seconds_per_kill: f64,
    /// `K` in the defence divisor `1 + defence / K`, and whether it was fitted to the player's
    /// own play rather than the starting guess.
    pub defence_k: f64,
    pub fitted: bool,
}

/// How a fight with monster `id` goes for `character` at `level`: hit chance, hits to kill, what a
/// blow does. `None` until the character's attack has been read or typed, or with no map data.
#[allow(clippy::too_many_arguments)]
pub fn monster_fight(h: &dyn Host,
    id: u32,
    character: Option<String>,
    level: Option<u32>,
    measured: Vec<MeasuredRate>,
    assumptions: Option<Assumptions>,
    skill: Option<String>,
) -> Option<MonsterFight> {
    let atlas = h.map().atlas()?;
    let foe = atlas.mob_row(id)?;
    let assumptions = assumptions.unwrap_or_default();
    let choice = Choice { assumptions: Some(&assumptions), skill: skill.as_deref() };
    let name = character.as_deref().filter(|n| !n.trim().is_empty())?;
    let fighter = fighter::fighter(h, name, level, &choice)?;
    let model = atlas.model(&measured, Some(fighter), assumptions, &h.map().layouts());
    let (defence_k, fitted) = match &model {
        Some(m) if m.fit.stretches > 0 => (m.fit.tuning.defence, true),
        _ => (Tuning::default().defence, false),
    };
    let fight = combat::fight(&fighter, &foe, defence_k);
    Some(MonsterFight { attack_seconds_per_kill: fight.attacks * fighter.attack_seconds, fighter, fight, defence_k, fitted })
}

/// How close the estimates come on the player's own maps, each hidden in turn and estimated from
/// the rest. `None` until map data has been downloaded.
pub fn check_estimates(h: &dyn Host,
    measured: Vec<MeasuredRate>,
    level: Option<u32>,
    assumptions: Option<Assumptions>,
    character: Option<String>,
    skill: Option<String>,
) -> Option<EstimateCheck> {
    let atlas = h.map().atlas()?;
    let layouts = h.map().layouts();
    let assumptions = assumptions.unwrap_or_default();
    let choice = Choice { assumptions: Some(&assumptions), skill: skill.as_deref() };
    let fighter = character.as_deref().filter(|n| !n.trim().is_empty()).and_then(|name| fighter::fighter(h, name, level, &choice));
    Some(atlas.check_estimates(&measured, fighter, assumptions, &layouts))
}
