//! Map data commands: downloading the cache, the map card, routes and drops.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State, Url};
use tauri_plugin_opener::OpenerExt;

use crate::error::{Error, Result};
use crate::mapdata::fetch::{self, Http};
use crate::mapdata::{Atlas, DataStatus, MapCard, MapMatch, MapRef, MonsterDrops, RefreshReport, Route, Store, drops};

const FILE_NAME: &str = "map-data.sqlite3";

/// Emitted to every window when the cached map data changes.
pub const MAP_DATA_EVENT: &str = "map-data";

/// Sites the card may open in the browser: its link-outs and its credits.
const LINK_HOSTS: [&str; 5] =
    ["mapleclassic.wiki", "meowdb.com", "henesys.gg", "osmsdataexplorer.com", "creativecommons.org"];

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

#[tauri::command]
pub fn map_route(state: State<MapDataState>, from: u32, to: u32) -> Option<Route> {
    state.atlas()?.route(from, to)
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
