//! Map data commands: downloading the cache, the map card, routes and drops.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State, Url};
use tauri_plugin_opener::OpenerExt;

use super::settings::SettingsState;
use crate::error::{Error, Result};
use crate::mapdata::fetch::{self, Http};
use crate::mapdata::{
    Atlas, DataStatus, ItemSources, MapCard, MapMatch, MapRef, MeasuredRate, MonsterDrops, QuestGuide, QuestQuery,
    RefreshReport, Route, Store, TrainingSpot, Travel, TravelFile, drops,
};

const FILE_NAME: &str = "map-data.sqlite3";
/// The player's copy of the travel connections, which replaces the bundled one when present.
const TRAVEL_FILE: &str = "travel.json";

/// Emitted to every window when the cached map data changes.
pub const MAP_DATA_EVENT: &str = "map-data";

/// Sites Leafline may open in the browser: the map card's link-outs and the credits.
const LINK_HOSTS: [&str; 8] = [
    "mapleclassic.wiki",
    "meowdb.com",
    "henesys.gg",
    "osmsdataexplorer.com",
    "creativecommons.org",
    "lucide.dev",
    "tauri.app",
    "svelte.dev",
];

pub struct MapDataState {
    path: PathBuf,
    atlas: RwLock<Option<Arc<Atlas>>>,
    downloading: AtomicBool,
    osms: Http,
    /// Shared so every drops request keeps to the wiki's one-a-second pace.
    wiki: Http,
}

impl MapDataState {
    /// Opens the cache in the app's data folder. Nothing is downloaded here.
    pub fn load(app: &AppHandle) -> Result<Self> {
        let path = app.path().app_data_dir()?.join(FILE_NAME);
        // A damaged cache must not stop the app; downloading again rebuilds it.
        let atlas = match Store::open(&path).and_then(|store| store.load()) {
            Ok(data) => data.map(|data| Arc::new(Atlas::new(data))),
            Err(err) => {
                eprintln!("map data cache unreadable: {err}");
                None
            }
        };
        Ok(Self {
            path,
            atlas: RwLock::new(atlas),
            downloading: AtomicBool::new(false),
            osms: Http::new(Duration::from_millis(250)),
            wiki: Http::new(Duration::from_secs(1)),
        })
    }

    fn atlas(&self) -> Option<Arc<Atlas>> {
        self.atlas.read().expect("atlas lock").clone()
    }

    fn refresh(&self, force: bool) -> Result<RefreshReport> {
        if self.downloading.swap(true, Ordering::SeqCst) {
            return Err(Error::Invalid("Map data is already being checked. Try again in a moment.".into()));
        }
        let result = (|| {
            let mut store = Store::open(&self.path)?;
            let report = fetch::refresh(&mut store, &self.osms, force, now())?;
            if !report.downloaded.is_empty() || self.atlas().is_none() {
                let atlas = store.load()?.map(|data| Arc::new(Atlas::new(data)));
                *self.atlas.write().expect("atlas lock") = atlas;
            }
            Ok(report)
        })();
        self.downloading.store(false, Ordering::SeqCst);
        result
    }
}

/// Once the user has downloaded map data, checks OSMS for changes in the background, at most
/// once a day.
pub fn check_for_updates(app: &AppHandle) {
    if app.state::<MapDataState>().atlas().is_none() {
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
pub fn find_map(state: State<MapDataState>, name: String, street: Option<String>) -> Option<MapMatch> {
    state.atlas()?.find(&name, street.as_deref())
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

/// The travel connections in use: the player's file in the app data folder if there is one
/// (and it reads), otherwise the bundled one. Also says which it is, and why a file was not used.
fn travel_file(app: &AppHandle) -> (TravelFile, Option<PathBuf>, Option<String>) {
    let Ok(path) = app.path().app_data_dir().map(|dir| dir.join(TRAVEL_FILE)) else {
        return (TravelFile::default_file(), None, None);
    };
    match std::fs::read_to_string(&path) {
        Ok(json) => match TravelFile::parse(&json) {
            Ok(file) => (file, Some(path), None),
            Err(err) => (TravelFile::default_file(), Some(path), Some(err)),
        },
        Err(_) => (TravelFile::default_file(), Some(path), None),
    }
}

/// The rides and scrolls the player turned on in Settings, found in the map data.
fn travel(app: &AppHandle, atlas: &Atlas) -> Travel {
    let choice = app.state::<SettingsState>().get().travel;
    if !choice.enabled {
        return Travel::none();
    }
    atlas.travel(&travel_file(app).0, choice).0
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TravelInfo {
    file: TravelFile,
    /// The player's own copy is in use.
    custom: bool,
    path: Option<String>,
    /// Why the player's copy could not be used, if it could not.
    error: Option<String>,
    /// Map names the map data has no map for; `None` before map data is downloaded.
    missing: Option<Vec<String>>,
}

/// The travel connections, for the Settings window.
#[tauri::command]
pub fn travel_info(app: AppHandle, state: State<MapDataState>) -> TravelInfo {
    let (file, path, error) = travel_file(&app);
    let custom = error.is_none() && path.as_ref().is_some_and(|p| p.exists());
    let missing = state.atlas().map(|atlas| atlas.travel(&file, Default::default()).1);
    TravelInfo { file, custom, path: path.map(|p| p.display().to_string()), error, missing }
}

/// Opens the player's copy of the travel connections in their editor, making it from the
/// bundled one first if there is none.
#[tauri::command]
pub fn edit_travel_file(app: AppHandle) -> Result<()> {
    let path = app.path().app_data_dir()?.join(TRAVEL_FILE);
    if !path.exists() {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&path, crate::mapdata::travel::DEFAULT_FILE)?;
    }
    app.opener().open_path(path.display().to_string(), None::<&str>).map_err(|err| Error::Invalid(err.to_string()))
}

/// Player-observed drops for the named monsters, from mapleclassic.wiki or the week-long cache.
#[tauri::command]
pub async fn monster_drops(app: AppHandle, monsters: Vec<String>) -> Result<Vec<MonsterDrops>> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<MapDataState>();
        drops::drops_for(&mut Store::open(&state.path)?, &state.wiki, &monsters, now())
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

/// Where to go for each quest the Quest Helper shows. Item objectives use only drops already
/// looked up; nothing is fetched here. `None` until map data has been downloaded.
#[tauri::command]
pub fn quest_guide(
    app: AppHandle,
    state: State<MapDataState>,
    quests: Vec<QuestQuery>,
    map_name: Option<String>,
    map_street: Option<String>,
    measured: Vec<MeasuredRate>,
) -> Result<Option<Guide>> {
    let Some(atlas) = state.atlas() else { return Ok(None) };
    let from = map_name.and_then(|name| atlas.find(&name, map_street.as_deref())).map(|m| m.card.map);
    let mut keys: Vec<String> = quests.iter().flat_map(|q| &q.objectives).map(|o| drops::item_key(o)).collect();
    keys.sort();
    keys.dedup();
    let (items, _) = Store::open(&state.path)?.cached_item_sources(&keys, now() - drops::MAX_AGE)?;
    let quests = atlas.quest_guide(&quests, from.as_ref().map(|m| m.id), &items, &measured, &travel(&app, &atlas));
    Ok(Some(Guide { from, quests }))
}

/// Asks mapleclassic.wiki which monsters drop each quest item, one request a second, and
/// caches the answers for a week.
#[tauri::command]
pub async fn look_up_quest_items(app: AppHandle, items: Vec<String>) -> Result<Vec<ItemSources>> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<MapDataState>();
        drops::sources_for(&mut Store::open(&state.path)?, &state.wiki, &items, now())
    })
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
    measured: Vec<MeasuredRate>,
) -> Option<Spots> {
    let atlas = state.atlas()?;
    let from = map_name.and_then(|name| atlas.find(&name, map_street.as_deref())).map(|m| m.card.map);
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
