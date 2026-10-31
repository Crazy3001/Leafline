//! Map data commands: downloading the cache, the map card, routes and drops.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State, Url};
use tauri_plugin_opener::OpenerExt;

use super::settings::SettingsState;
use crate::build::Catalog;
use crate::error::{Error, Result};
use crate::mapdata::fetch::{self, Http};
use crate::mapdata::osms::Dataset;
use crate::mapdata::{
    Atlas, DataStatus, MapCard, MapMatch, MapRef, MeasuredRate, MonsterDrops, QuestGuide, QuestQuery, RefreshReport,
    Route, Store, TrainingSpot, Travel, TravelFile, drops,
};

const FILE_NAME: &str = "map-data.sqlite3";
/// The player's copy of the travel connections, which replaces the downloaded one when present.
const TRAVEL_FILE: &str = "travel.json";
/// The travel connections downloaded from mapleclassic.wiki.
const TRAVEL_CACHE: &str = "travel-wiki.json";

/// Emitted to every window when the cached map data changes.
pub const MAP_DATA_EVENT: &str = "map-data";

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
    let catalog = Catalog::new(std::mem::take(&mut data.skills), std::mem::take(&mut data.items));
    (Arc::new(Atlas::new(data)), Arc::new(catalog))
}

pub struct MapDataState {
    path: PathBuf,
    atlas: RwLock<Option<Arc<Atlas>>>,
    catalog: RwLock<Option<Arc<Catalog>>>,
    downloading: AtomicBool,
    osms: Http,
    /// Shared so every wiki request keeps to its one-a-second pace.
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
        Ok(Self {
            path,
            atlas: RwLock::new(atlas),
            catalog: RwLock::new(catalog),
            downloading: AtomicBool::new(false),
            osms: Http::new(Duration::from_millis(250)),
            wiki: Http::new(Duration::from_secs(1)),
            drops_refresh: Mutex::new(()),
        })
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

    fn refresh_drops(&self) -> Result<()> {
        let _turn = self.drops_refresh.lock().expect("drops refresh lock");
        drops::refresh(&mut Store::open(&self.path)?, &self.wiki, now())
    }
}

/// Once the user has downloaded map data, checks OSMS for changes in the background, at most
/// once a day, and renews the drop table once it is a week old, if it was ever downloaded. An
/// update of Leafline that replaced the cache gets back what the user had the same way.
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
        let has_drops = Store::open(&state.path).and_then(|s| s.wants_drops()).unwrap_or(false);
        if has_drops && let Err(err) = state.refresh_drops() {
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

/// The fewest steps between two maps, through portals and any travel the player turned on.
#[tauri::command]
pub fn map_route(app: AppHandle, state: State<MapDataState>, from: u32, to: u32) -> Option<Route> {
    let atlas = state.atlas()?;
    atlas.route_with(from, to, &travel(&app, &atlas))
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
    match travel_file(app).0 {
        Some(file) if choice.enabled => atlas.travel(&file, choice).0,
        _ => Travel::none(),
    }
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

/// Downloads the drop table from mapleclassic.wiki, for finding who drops a quest item. Only on
/// the player's request.
#[tauri::command]
pub async fn download_drops(app: AppHandle) -> Result<()> {
    tauri::async_runtime::spawn_blocking(move || app.state::<MapDataState>().refresh_drops())
        .await
        .map_err(|err| Error::Invalid(err.to_string()))?
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Spots {
    /// The map portals are counted from, when the minimap was read and matched.
    from: Option<MapRef>,
    spots: Vec<TrainingSpot>,
}

/// Every map with monsters to hunt, for the training spot finder, with the EXP per hour the
/// player measured on each. `None` until map data has been downloaded.
#[tauri::command]
pub fn training_spots(
    app: AppHandle,
    state: State<MapDataState>,
    map_name: Option<String>,
    map_street: Option<String>,
    map_twin: Option<u32>,
    measured: Vec<MeasuredRate>,
) -> Option<Spots> {
    let atlas = state.atlas()?;
    let from = map_name.and_then(|name| atlas.find_twin(&name, map_street.as_deref(), map_twin)).map(|m| m.card.map);
    let spots = atlas.training_spots(from.as_ref().map(|m| m.id), &measured, &travel(&app, &atlas));
    Some(Spots { from, spots })
}

/// Opens one of the card's links in the default browser. Only the sites it links to are allowed.
#[tauri::command]
pub fn open_link(app: AppHandle, url: String) -> Result<()> {
    let parsed = Url::parse(&url).map_err(|err| Error::Invalid(format!("{url}: {err}")))?;
    let allowed = parsed.scheme() == "https" && parsed.host_str().is_some_and(|host| LINK_HOSTS.contains(&host));
    if !allowed {
        return Err(Error::Invalid(format!("Leafline does not open links to {url}")));
    }
    app.opener().open_url(parsed.as_str(), None::<&str>).map_err(|err| Error::Invalid(err.to_string()))
}

fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}
