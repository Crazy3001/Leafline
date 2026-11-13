//! Map data commands: downloading the cache, the map card, routes and drops.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State, Url};
use tauri_plugin_opener::OpenerExt;

use super::settings::SettingsState;
use super::{overlay, pins};
use crate::build::Catalog;
use crate::error::{Error, Result};
use crate::mapdata::fetch::{self, Http};
use crate::mapdata::osms::Dataset;
use crate::mapdata::layout::{self, Layouts};
use crate::mapdata::store::{DROPS_FAILED, LAYOUTS_FAILED, TURN_INS_FAILED};
use crate::mapdata::{
    Assumptions, Atlas, DataStatus, ItemOrigin, KillSpeed, MapCard, MapMatch, MapPage, MapQuery, MapRef, MeasuredRate,
    MonsterDrops, MonsterFilters, MonsterPage, MonsterQuery, MonsterSheet, NpcPage, NpcQuery, NpcSheet, QuestGuide, QuestPage, QuestQuery, QuestSearch,
    QuestSheet, RefreshReport, Route, Store, TrainingSpot, Travel, TravelFile, drops, shops, turnins,
};

const FILE_NAME: &str = "map-data.sqlite3";
/// The player's copy of the travel connections, which replaces the downloaded one when present.
const TRAVEL_FILE: &str = "travel.json";
/// The travel connections downloaded from mapleclassic.wiki.
const TRAVEL_CACHE: &str = "travel-wiki.json";

/// Emitted to every window when the cached map data changes.
pub const MAP_DATA_EVENT: &str = "map-data";
/// Emitted to every window to bring up a quest in the Quests tab.
pub const SHOW_QUEST_EVENT: &str = "show-quest";
/// Emitted to every window to bring up a monster in the Lookup tab.
pub const SHOW_MONSTER_EVENT: &str = "show-monster";
/// Emitted to every window to bring up a map or an NPC in the Lookup tab.
pub const SHOW_PLACE_EVENT: &str = "show-place";

/// Sites Leafline may open in the browser: the link-outs and the credits.
const LINK_HOSTS: [&str; 9] = [
    "mapleclassic.wiki",
    "meowdb.com",
    "meso.watch",
    "henesys.gg",
    "osmsdataexplorer.com",
    "creativecommons.org",
    "lucide.dev",
    "tauri.app",
    "svelte.dev",
];

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
    /// Every map's layout, from Leafline's data repository. Empty until downloaded.
    layouts: RwLock<Arc<Layouts>>,
    downloading: AtomicBool,
    osms: Http,
    /// Shared so every wiki request (travel) keeps to its one-a-second pace.
    wiki: Http,
    /// One drop table download at a time; whoever waits then finds the table fresh.
    drops_refresh: Mutex<()>,
}

impl MapDataState {
    /// Opens the cache in the app's data folder. Nothing is downloaded here.
    pub fn load(app: &AppHandle) -> Result<Self> {
        let path = app.path().app_data_dir()?.join(FILE_NAME);
        // A damaged cache must not stop the app; downloading again rebuilds it.
        let (atlas, catalog) = match Store::open(&path).and_then(|store| store.load()) {
            Ok(data) => data.map(split).unzip(),
            Err(err) => {
                eprintln!("map data cache unreadable: {err}");
                (None, None)
            }
        };
        let layouts = Store::open(&path).and_then(|store| store.layouts()).unwrap_or_default();
        Ok(Self {
            path,
            atlas: RwLock::new(atlas),
            catalog: RwLock::new(catalog),
            layouts: RwLock::new(Arc::new(layouts)),
            downloading: AtomicBool::new(false),
            osms: Http::new(Duration::from_millis(250)),
            wiki: Http::new(Duration::from_secs(1)),
            drops_refresh: Mutex::new(()),
        })
    }

    pub(super) fn layouts(&self) -> Arc<Layouts> {
        self.layouts.read().expect("layouts lock").clone()
    }

    /// Renews the map layouts, once a day at most, from Leafline's data repository.
    fn refresh_layouts(&self) -> Result<()> {
        let _turn = self.drops_refresh.lock().expect("drops refresh lock");
        let mut store = Store::open(&self.path)?;
        if layout::refresh(&mut store, &self.osms, now())? {
            *self.layouts.write().expect("layouts lock") = Arc::new(store.layouts()?);
        }
        Ok(())
    }

    pub(super) fn atlas(&self) -> Option<Arc<Atlas>> {
        self.atlas.read().expect("atlas lock").clone()
    }

    pub(super) fn catalog(&self) -> Option<Arc<Catalog>> {
        self.catalog.read().expect("catalog lock").clone()
    }

    fn refresh(&self, force: bool) -> Result<RefreshReport> {
        if self.downloading.swap(true, Ordering::SeqCst) {
            return Err(Error::Invalid("Map data is already being checked. Try again in a moment.".into()));
        }
        let result = (|| {
            let mut store = Store::open(&self.path)?;
            let report = fetch::refresh(&mut store, &self.osms, force, now())?;
            if !report.downloaded.is_empty() || self.atlas().is_none() {
                let (atlas, catalog) = store.load()?.map(split).unzip();
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
    fn refresh_turn_ins(&self) -> Result<()> {
        let _turn = self.drops_refresh.lock().expect("drops refresh lock");
        let mut store = Store::open(&self.path)?;
        if turnins::refresh(&mut store, &self.osms, now())? {
            let (atlas, catalog) = store.load()?.map(split).unzip();
            *self.atlas.write().expect("atlas lock") = atlas;
            *self.catalog.write().expect("catalog lock") = catalog;
        }
        Ok(())
    }

    /// Renews Leafline's shop file once a day, and brings the atlas up to date with it.
    fn refresh_shops(&self) -> Result<()> {
        let _turn = self.drops_refresh.lock().expect("drops refresh lock");
        let mut store = Store::open(&self.path)?;
        if shops::refresh(&mut store, &self.osms, now())? {
            let (atlas, catalog) = store.load()?.map(split).unzip();
            *self.atlas.write().expect("atlas lock") = atlas;
            *self.catalog.write().expect("catalog lock") = catalog;
        }
        Ok(())
    }

    fn refresh_drops(&self) -> Result<()> {
        let _turn = self.drops_refresh.lock().expect("drops refresh lock");
        drops::refresh(&mut Store::open(&self.path)?, &self.osms, now())
    }

    /// Forgets a failed download (see [`drops::RETRY_AFTER`]), so the next refresh asks at
    /// once. Only for the player's own request: automatic lookups keep waiting.
    fn retry(&self, failed: &str) -> Result<()> {
        Store::open(&self.path)?.set_failure(failed, None)
    }
}

/// Once the user has downloaded map data, checks OSMS for changes in the background, at most
/// once a day, and renews the turn-in table and drop list once they are a day old. An update of
/// Leafline that replaced the cache gets back what the user had the same way.
pub fn check_for_updates(app: &AppHandle) {
    let state = app.state::<MapDataState>();
    let wanted = Store::open(&state.path).and_then(|s| s.wants_maps()).unwrap_or(false);
    if state.atlas().is_none() && !wanted {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        let state = app.state::<MapDataState>();
        match state.refresh(false) {
            Ok(report) if !report.downloaded.is_empty() => {
                if let Ok(status) = Store::open(&state.path).and_then(|s| s.status()) {
                    let _ = app.emit(MAP_DATA_EVENT, &status);
                }
            }
            Ok(_) => {}
            // Offline or OSMS unavailable: the cache keeps working, and the user can retry.
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
    });
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Downloaded {
    report: RefreshReport,
    status: DataStatus,
}

#[tauri::command]
pub fn map_data_status(state: State<MapDataState>) -> Result<DataStatus> {
    Store::open(&state.path)?.status()
}

/// The explicit "Download map data" action: fetches whatever changed on OSMS. Without `force`,
/// does nothing if the last check was under a day ago.
#[tauri::command]
pub async fn download_map_data(app: AppHandle, force: bool) -> Result<Downloaded> {
    let handle = app.clone();
    let downloaded = tauri::async_runtime::spawn_blocking(move || -> Result<Downloaded> {
        let state = handle.state::<MapDataState>();
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
        Ok(Downloaded { report, status: Store::open(&state.path)?.status()? })
    })
    .await
    .map_err(|err| Error::Invalid(err.to_string()))??;
    app.emit(MAP_DATA_EVENT, &downloaded.status)?;
    Ok(downloaded)
}

/// The card for the map a minimap header names, matched fuzzily against the cached names.
#[tauri::command]
pub fn find_map(
    state: State<MapDataState>,
    name: String,
    street: Option<String>,
    twin: Option<u32>,
) -> Option<MapMatch> {
    state.atlas()?.find_twin(&name, street.as_deref(), twin)
}

#[tauri::command]
pub fn map_card(state: State<MapDataState>, id: u32) -> Option<MapCard> {
    state.atlas()?.card(id)
}

#[tauri::command]
pub fn search_maps(state: State<MapDataState>, text: String) -> Vec<MapRef> {
    state.atlas().map(|atlas| atlas.search(&text, 8)).unwrap_or_default()
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

/// The fewest steps between two maps, through portals and any travel the player turned on.
#[tauri::command]
pub fn map_route(app: AppHandle, state: State<MapDataState>, from: u32, to: u32) -> Option<RouteInfo> {
    let atlas = state.atlas()?;
    let route = atlas.route_with(from, to, &travel(&app, &atlas))?;
    let walk_seconds = layout::route_walk_seconds(&route, &state.layouts());
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

/// The travel connections in use: the player's own copy in the app data folder if there is one
/// and it reads, otherwise the copy downloaded from the wiki, if any. Also says why the
/// player's copy could not be used.
fn travel_file(app: &AppHandle) -> (Option<TravelFile>, TravelOrigin, Option<String>) {
    let Ok(dir) = app.path().app_data_dir() else { return (None, TravelOrigin::None, None) };
    let read = |name: &str| std::fs::read_to_string(dir.join(name)).ok().map(|json| TravelFile::parse(&json));
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
fn travel(app: &AppHandle, atlas: &Atlas) -> Travel {
    let choice = app.state::<SettingsState>().get().travel;
    // NPC rides are always in: some maps can't be reached without one. Taxis and scrolls wait
    // for the player's switch, which `choice` checks for each ride.
    let file = travel_file(app).0.unwrap_or_else(TravelFile::empty).with_known_rides();
    atlas.travel(&file, choice).0
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TravelInfo {
    file: Option<TravelFile>,
    origin: TravelOrigin,
    /// The player's own copy, whether or not it exists yet.
    path: Option<String>,
    /// Why the player's copy could not be used, if it could not.
    error: Option<String>,
    /// Map names the map data has no map for; `None` before map data is downloaded.
    missing: Option<Vec<String>>,
}

/// The travel connections, for the Settings window.
#[tauri::command]
pub fn travel_info(app: AppHandle, state: State<MapDataState>) -> TravelInfo {
    let (file, origin, error) = travel_file(&app);
    let missing = file.as_ref().and_then(|file| Some(state.atlas()?.travel(file, Default::default()).1));
    let path = app.path().app_data_dir().ok().map(|dir| dir.join(TRAVEL_FILE).display().to_string());
    TravelInfo { file, origin, path, error, missing }
}

/// Downloads the taxis and return scrolls players recorded on mapleclassic.wiki and keeps them
/// in the app data folder. Only on the player's request.
#[tauri::command]
pub async fn download_travel(app: AppHandle) -> Result<TravelInfo> {
    let fetch_app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let state = fetch_app.state::<MapDataState>();
        let file = TravelFile::fetch(&state.wiki, now())?;
        let dir = fetch_app.path().app_data_dir()?;
        std::fs::create_dir_all(&dir)?;
        let json = serde_json::to_string_pretty(&file).map_err(|err| Error::Invalid(err.to_string()))?;
        std::fs::write(dir.join(TRAVEL_CACHE), json)?;
        Ok::<(), Error>(())
    })
    .await
    .map_err(|err| Error::Invalid(err.to_string()))??;
    Ok(travel_info(app.clone(), app.state::<MapDataState>()))
}

/// Opens the player's copy of the travel connections in their editor, making it first from the
/// downloaded connections, or empty if there are none.
#[tauri::command]
pub fn edit_travel_file(app: AppHandle) -> Result<()> {
    let dir = app.path().app_data_dir()?;
    let path = dir.join(TRAVEL_FILE);
    if !path.exists() {
        std::fs::create_dir_all(&dir)?;
        let start = match travel_file(&app) {
            (Some(file), TravelOrigin::Wiki, _) => file,
            _ => TravelFile::empty(),
        };
        let json = serde_json::to_string_pretty(&start).map_err(|err| Error::Invalid(err.to_string()))?;
        std::fs::write(&path, json)?;
    }
    app.opener().open_path(path.display().to_string(), None::<&str>).map_err(|err| Error::Invalid(err.to_string()))
}

/// Player-observed drops for the named monsters, from the downloaded table. The first call
/// downloads the table; the names are never sent.
#[tauri::command]
pub async fn monster_drops(app: AppHandle, monsters: Vec<String>) -> Result<Vec<MonsterDrops>> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<MapDataState>();
        state.refresh_drops()?;
        Store::open(&state.path)?.drops(&monsters)
    })
    .await
    .map_err(|err| Error::Invalid(err.to_string()))?
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
#[tauri::command]
pub fn quest_guide(
    app: AppHandle,
    state: State<MapDataState>,
    quests: Vec<QuestQuery>,
    map_name: Option<String>,
    map_street: Option<String>,
    map_twin: Option<u32>,
    measured: Vec<MeasuredRate>,
) -> Result<Option<Guide>> {
    let Some(atlas) = state.atlas() else { return Ok(None) };
    let from = map_name.and_then(|name| atlas.find_twin(&name, map_street.as_deref(), map_twin)).map(|m| m.card.map);
    let objectives: Vec<String> = quests.iter().flat_map(|q| q.objectives.iter().cloned()).collect();
    let items = drops::sources_for(&Store::open(&state.path)?, &objectives)?;
    let quests = atlas.quest_guide(&quests, from.as_ref().map(|m| m.id), &items, &measured, &travel(&app, &atlas));
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
#[tauri::command]
pub async fn item_origin(
    app: AppHandle,
    id: u32,
    map_name: Option<String>,
    map_street: Option<String>,
    map_twin: Option<u32>,
) -> Result<Option<ItemWhere>> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<MapDataState>();
        let (Some(atlas), Some(catalog)) = (state.atlas(), state.catalog()) else { return Ok(None) };
        let Some(item) = catalog.item(id) else { return Ok(None) };
        if let Err(err) = state.refresh_drops() {
            eprintln!("drop table download failed: {err}");
        }
        let store = Store::open(&state.path)?;
        let sources = drops::sources_for(&store, std::slice::from_ref(&item.name))?;
        let from = map_name.and_then(|name| atlas.find_twin(&name, map_street.as_deref(), map_twin)).map(|m| m.card.map);
        let start = from.as_ref().map(|m| m.id);
        let origin = atlas.item_origin(&catalog.copies_of(id), &item.name, sources.first(), start, &travel(&app, &atlas));
        Ok(Some(ItemWhere { from, origin }))
    })
    .await
    .map_err(|err| Error::Invalid(err.to_string()))?
}

#[tauri::command]
pub fn search_quests(state: State<MapDataState>, search: QuestSearch) -> QuestPage {
    state.atlas().map(|atlas| atlas.search_quests(&search)).unwrap_or_default()
}

#[tauri::command]
pub fn quest_regions(state: State<MapDataState>) -> Vec<String> {
    state.atlas().map(|atlas| atlas.quest_regions()).unwrap_or_default()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestWhere {
    /// The map portals are counted from, when the minimap was read and matched.
    from: Option<MapRef>,
    sheet: QuestSheet,
}

/// All the data says about a quest. Without the drop table, its items show no droppers.
#[tauri::command]
pub async fn quest_sheet(
    app: AppHandle,
    id: u32,
    map_name: Option<String>,
    map_street: Option<String>,
    map_twin: Option<u32>,
) -> Result<Option<QuestWhere>> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<MapDataState>();
        let (Some(atlas), Some(catalog)) = (state.atlas(), state.catalog()) else { return Ok(None) };
        let item_name = |id: u32| catalog.item(id).map(|item| item.name.clone());
        if let Err(err) = state.refresh_drops() {
            eprintln!("drop table download failed: {err}");
        }
        let items = atlas.quest_items(id, item_name);
        let sources = drops::sources_for(&Store::open(&state.path)?, &items)?;
        let from = map_name.and_then(|name| atlas.find_twin(&name, map_street.as_deref(), map_twin)).map(|m| m.card.map);
        let start = from.as_ref().map(|m| m.id);
        let sheet = atlas.quest_sheet(id, item_name, &sources, start, &travel(&app, &atlas));
        Ok(sheet.map(|sheet| QuestWhere { from, sheet }))
    })
    .await
    .map_err(|err| Error::Invalid(err.to_string()))?
}

/// Brings a quest up in the Quests tab, from whichever window asked.
#[tauri::command]
pub fn show_quest(app: AppHandle, id: u32) -> Result<()> {
    if !pins::shows_quests(&app) {
        overlay::show_full(&app)?;
    }
    Ok(app.emit(SHOW_QUEST_EVENT, id)?)
}

/// One page of the monster search. Drops are matched from the downloaded table, if there is one.
#[tauri::command]
pub fn search_monsters(state: State<MapDataState>, query: MonsterQuery) -> Result<MonsterPage> {
    let Some(atlas) = state.atlas() else { return Ok(MonsterPage::default()) };
    Ok(atlas.search_monsters(&query, &Store::open(&state.path)?.dropped_pairs()?))
}

#[tauri::command]
pub fn monster_filters(state: State<MapDataState>) -> MonsterFilters {
    state.atlas().map(|atlas| atlas.monster_filters()).unwrap_or_default()
}

/// The monster a name means.
#[tauri::command]
pub fn find_monster(state: State<MapDataState>, name: String) -> Option<u32> {
    state.atlas()?.monster_like(&name)
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
#[tauri::command]
pub async fn monster_sheet(
    app: AppHandle,
    id: u32,
    map_name: Option<String>,
    map_street: Option<String>,
    map_twin: Option<u32>,
    measured: Vec<MeasuredRate>,
) -> Result<Option<MonsterWhere>> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<MapDataState>();
        let Some(atlas) = state.atlas() else { return Ok(None) };
        let Some(name) = atlas.monster_name(id).map(str::to_string) else { return Ok(None) };
        if let Err(err) = state.refresh_drops() {
            eprintln!("drop table download failed: {err}");
        }
        let store = Store::open(&state.path)?;
        let drops = if store.drops_fetched_at()?.is_some() { store.drops(&[name])?.pop() } else { None };
        let from = map_name.and_then(|name| atlas.find_twin(&name, map_street.as_deref(), map_twin)).map(|m| m.card.map);
        let start = from.as_ref().map(|m| m.id);
        let sheet = atlas.monster_sheet(id, drops, start, &measured, &travel(&app, &atlas));
        Ok(sheet.map(|sheet| MonsterWhere { from, sheet }))
    })
    .await
    .map_err(|err| Error::Invalid(err.to_string()))?
}

/// What to bring up in the Lookup tab: one monster, or a search when no monster was matched.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MonsterLink {
    id: Option<u32>,
    text: Option<String>,
}

/// Brings a monster up in the Lookup tab, from whichever window asked. A pinned Lookup panel
/// shows it by itself; otherwise the overlay has to.
#[tauri::command]
pub fn show_monster(app: AppHandle, id: Option<u32>, text: Option<String>) -> Result<()> {
    if !pins::shows_item(&app) {
        overlay::show_full(&app)?;
    }
    Ok(app.emit(SHOW_MONSTER_EVENT, MonsterLink { id, text })?)
}

/// One page of the map search.
#[tauri::command]
pub fn browse_maps(state: State<MapDataState>, query: MapQuery) -> MapPage {
    state.atlas().map(|atlas| atlas.browse_maps(&query)).unwrap_or_default()
}

#[tauri::command]
pub fn map_regions(state: State<MapDataState>) -> Vec<String> {
    state.atlas().map(|atlas| atlas.map_regions()).unwrap_or_default()
}

/// The areas Lookup narrows maps, monsters and NPCs to.
#[tauri::command]
pub fn map_areas(state: State<MapDataState>) -> Vec<String> {
    state.atlas().map(|atlas| atlas.map_areas()).unwrap_or_default()
}

/// One page of the NPC search.
#[tauri::command]
pub fn search_npcs(state: State<MapDataState>, query: NpcQuery) -> NpcPage {
    state.atlas().map(|atlas| atlas.search_npcs(&query)).unwrap_or_default()
}

/// The NPC a name means.
#[tauri::command]
pub fn find_npc(state: State<MapDataState>, name: String) -> Option<u32> {
    state.atlas()?.npc_like(&name)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcWhere {
    /// The map portals are counted from, when the minimap was read and matched.
    from: Option<MapRef>,
    sheet: NpcSheet,
}

/// All the data says about an NPC.
#[tauri::command]
pub fn npc_sheet(
    app: AppHandle,
    state: State<MapDataState>,
    id: u32,
    map_name: Option<String>,
    map_street: Option<String>,
    map_twin: Option<u32>,
) -> Option<NpcWhere> {
    let atlas = state.atlas()?;
    let from = map_name.and_then(|name| atlas.find_twin(&name, map_street.as_deref(), map_twin)).map(|m| m.card.map);
    let sheet = atlas.npc_sheet(id, from.as_ref().map(|m| m.id), &travel(&app, &atlas))?;
    Some(NpcWhere { from, sheet })
}

/// What to bring up in the Lookup tab: a map or an NPC.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PlaceLink {
    mode: String,
    id: u32,
}

/// Brings a map (`maps`) or an NPC (`npcs`) up in the Lookup tab, from whichever window asked.
/// A pinned Lookup panel shows it by itself; otherwise the overlay has to.
#[tauri::command]
pub fn show_place(app: AppHandle, mode: String, id: u32) -> Result<()> {
    if !pins::shows_item(&app) {
        overlay::show_full(&app)?;
    }
    Ok(app.emit(SHOW_PLACE_EVENT, PlaceLink { mode, id })?)
}

/// Downloads Leafline's drop list if there is none or it is over a day old.
/// `retry` is set when the player pressed a button for it, so a download that failed recently is
/// tried again; panels that ask on their own leave it unset and wait out the back-off.
#[tauri::command]
pub async fn download_drops(app: AppHandle, retry: Option<bool>) -> Result<()> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<MapDataState>();
        if retry == Some(true) {
            state.retry(DROPS_FAILED)?;
        }
        state.refresh_drops()
    })
    .await
    .map_err(|err| Error::Invalid(err.to_string()))?
}

/// When the drop table was last downloaded, in Unix seconds, or `None` before the first time.
#[tauri::command]
pub fn drops_fetched_at(state: State<MapDataState>) -> Result<Option<i64>> {
    if !state.path.exists() {
        return Ok(None);
    }
    Store::open(&state.path)?.drops_fetched_at()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Spots {
    /// The map portals are counted from, when the minimap was read and matched.
    from: Option<MapRef>,
    /// How fast the player gets through monster HP, behind every estimate. None until enough
    /// solo play is saved.
    speed: Option<KillSpeed>,
    spots: Vec<TrainingSpot>,
}

/// Every map with monsters to hunt, for the training spot finder, with the EXP per hour the
/// player measured on each and an estimate from their kill speed for the rest. `level` is the
/// character's; `assumptions` what the player set for respawn time and EXP rate. `None` until
/// map data has been downloaded.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn training_spots(
    app: AppHandle,
    state: State<MapDataState>,
    map_name: Option<String>,
    map_street: Option<String>,
    map_twin: Option<u32>,
    measured: Vec<MeasuredRate>,
    level: Option<u32>,
    assumptions: Option<Assumptions>,
) -> Option<Spots> {
    let atlas = state.atlas()?;
    let from = map_name.and_then(|name| atlas.find_twin(&name, map_street.as_deref(), map_twin)).map(|m| m.card.map);
    let layouts = state.layouts();
    let estimator = atlas.estimator(&measured, level, assumptions.unwrap_or_default(), &layouts);
    let spots = atlas.training_spots(
        from.as_ref().map(|m| m.id),
        &measured,
        &travel(&app, &atlas),
        estimator.as_ref(),
        &layouts,
    );
    Some(Spots { from, speed: estimator.map(|e| e.speed), spots })
}

/// Opens one of the card's links in the default browser. Only the sites it links to are allowed.
#[tauri::command]
pub fn open_link(app: AppHandle, url: String) -> Result<()> {
    let parsed = Url::parse(&url).map_err(|err| Error::Invalid(format!("{url}: {err}")))?;
    if !link_allowed(&parsed) {
        return Err(Error::Invalid(format!("Leafline does not open links to {url}")));
    }
    app.opener().open_url(parsed.as_str(), None::<&str>).map_err(|err| Error::Invalid(err.to_string()))
}

/// A site from [`LINK_HOSTS`], or Leafline's own pages on GitHub: its releases and the forms
/// for reporting a problem or suggesting an idea. Nowhere else on GitHub.
fn link_allowed(url: &Url) -> bool {
    if url.scheme() != "https" {
        return false;
    }
    match url.host_str() {
        Some("github.com") => {
            let own = format!("/{}/", super::update::RELEASES_REPO).to_ascii_lowercase();
            url.path().to_ascii_lowercase().starts_with(&own)
        }
        Some(host) => LINK_HOSTS.contains(&host),
        None => false,
    }
}

fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

#[cfg(test)]
mod link_tests {
    use super::*;

    #[test]
    fn only_leaflines_own_github_pages_open() {
        let allowed = |url: &str| link_allowed(&Url::parse(url).unwrap());
        assert!(allowed("https://github.com/Crazy3001/Leafline/releases/tag/v0.1.1"));
        assert!(allowed("https://github.com/Crazy3001/Leafline/issues/new?template=bug_report.yml"));
        assert!(allowed("https://mapleclassic.wiki/index.php?search=Snail"));
        assert!(!allowed("https://github.com/someone/else"));
        assert!(!allowed("https://github.com/Crazy3001/LeaflineFake/issues"));
        assert!(!allowed("http://github.com/Crazy3001/Leafline/issues"));
        assert!(!allowed("https://example.com/"));
    }
}
