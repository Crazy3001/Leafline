//! The worker's side: opening storage, running commands, records files.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

use leafline_core::clock::Duration;
use leafline_core::error::{Error, Result};
use leafline_core::service::damage::DamageState;
use leafline_core::service::files::Files;
use leafline_core::service::host::{Host, emit};
use leafline_core::service::mapdata::{self, MAP_DATA_EVENT, MapDataState};
use leafline_core::service::nav::{Destination, NAV_EVENT, Nav, NavState};
use leafline_core::service::records::{self, Export, RECORDS_REPLACED_EVENT, UserDataState};
use leafline_core::service::settings::{SETTINGS_EVENT, Settings, SettingsState};
use serde::Deserialize;
use sqlite_wasm_rs::WasmOsCallback;
use sqlite_wasm_rs::vfs::VfsFilesManager;
use sqlite_wasm_rs::vfs::transfer::DbTransfer;
use sqlite_wasm_vfs::sahpool::{OpfsSAHPoolCfgBuilder, OpfsSAHPoolUtil, install};
use wasm_bindgen::prelude::*;

use crate::files::{self, DbFiles};
use crate::reply::{Reply, parse, reply, reply_result};
use crate::xhr::Xhr;

const RECORDS: &str = "/leafline.sqlite3";
const MAP_DATA: &str = "/map-data.sqlite3";
/// The records as they were before the last records file was opened.
const BEFORE_IMPORT: &str = "/leafline-before-import.sqlite3";
const EXPORT: &str = "/export.leafline";
const IMPORT: &str = "/import.leafline";

/// Everything the commands use, held by the worker for as long as the tab is open.
struct WebHost {
    map: MapDataState,
    records: UserDataState,
    damage: DamageState,
    settings: SettingsState,
    nav: NavState,
    files: DbFiles,
    /// Events the last command raised, sent back with its value.
    events: RefCell<Vec<(String, serde_json::Value)>>,
}

impl Host for WebHost {
    fn map(&self) -> &MapDataState {
        &self.map
    }

    fn records(&self) -> &UserDataState {
        &self.records
    }

    fn damage(&self) -> &DamageState {
        &self.damage
    }

    fn settings(&self) -> Settings {
        self.settings.get()
    }

    fn files(&self) -> &dyn Files {
        &self.files
    }

    fn emit_json(&self, event: &str, payload: serde_json::Value) {
        self.events.borrow_mut().push((event.to_string(), payload));
    }
}

thread_local! {
    /// Map pictures cut this visit, by map id: the page asks again each time a map card opens.
    static MAP_VIEWS: RefCell<HashMap<u32, serde_json::Value>> = RefCell::new(HashMap::new());
    static HOST: RefCell<Option<Rc<WebHost>>> = const { RefCell::new(None) };
    static POOL: RefCell<Option<OpfsSAHPoolUtil>> = const { RefCell::new(None) };
}

fn host() -> std::result::Result<Rc<WebHost>, JsValue> {
    HOST.with(|h| h.borrow().clone()).ok_or_else(|| JsValue::from_str("Leafline's storage isn't open."))
}

fn pool() -> std::result::Result<OpfsSAHPoolUtil, JsValue> {
    POOL.with(|p| p.borrow().clone()).ok_or_else(|| JsValue::from_str("Leafline's storage isn't open."))
}

fn js(err: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&err.to_string())
}

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

/// Opens Leafline's storage for this site. Fails with `in-use` when another tab has it open:
/// only one tab at a time can (the page asks the other to let go).
#[wasm_bindgen]
pub async fn open() -> std::result::Result<String, JsValue> {
    if HOST.with(|h| h.borrow().is_some()) {
        let pool = pool()?;
        if pool.is_paused() {
            pool.resume().await.map_err(|err| JsValue::from_str(&format!("in-use: {err}")))?;
            reopen()?;
        }
        return Ok(version());
    }
    let cfg = OpfsSAHPoolCfgBuilder::new().vfs_name("leafline").directory("leafline").initial_capacity(12).build();
    let pool = install::<WasmOsCallback>(&cfg, true).await.map_err(|err| JsValue::from_str(&format!("in-use: {err}")))?;
    POOL.with(|p| *p.borrow_mut() = Some(pool));
    reopen()?;
    Ok(version())
}

/// Opens every file over again, after the storage was opened or handed back.
fn reopen() -> std::result::Result<(), JsValue> {
    let files = DbFiles::open(Path::new(files::FILE_NAME)).map_err(js)?;
    let host = WebHost {
        map: MapDataState::load(
            MAP_DATA.into(),
            Box::new(Xhr::new(Duration::from_millis(250))),
            Box::new(Xhr::new(Duration::from_secs(1))),
        )
        .mirror_first(),
        records: UserDataState::load(Path::new(RECORDS), None),
        damage: DamageState::load(&files),
        settings: SettingsState::load(&files),
        nav: NavState::load(&files),
        files,
        events: RefCell::new(Vec::new()),
    };
    HOST.with(|h| *h.borrow_mut() = Some(Rc::new(host)));
    Ok(())
}

/// Lets go of the storage so another tab can open it. The worker can open it again later.
#[wasm_bindgen]
pub fn close() -> std::result::Result<(), JsValue> {
    HOST.with(|h| h.borrow_mut().take());
    pool()?.pause().map_err(js)
}

fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Runs a command by the name the program's windows use, with their camelCase arguments as JSON.
/// Returns `{"value": ..., "events": [[name, payload], ...]}`, or fails with the command's error
/// message.
#[wasm_bindgen]
pub fn call(name: &str, args: &str) -> std::result::Result<String, JsValue> {
    let host = host()?;
    let args: serde_json::Value = if args.is_empty() { serde_json::Value::Null } else { serde_json::from_str(args).map_err(js)? };
    let args = if args.is_null() { serde_json::json!({}) } else { args };
    host.events.borrow_mut().clear();
    let reply = crate::generated::dispatch(&*host, name, args.clone())
        .or_else(|| own(&host, name, args))
        .unwrap_or_else(|| Err(format!("The website can't do {name}.")));
    let events = std::mem::take(&mut *host.events.borrow_mut());
    match reply {
        Ok(value) => Ok(serde_json::json!({ "value": value, "events": events }).to_string()),
        Err(message) => Err(JsValue::from_str(&message)),
    }
}

/// The commands whose program versions also move windows, so they aren't shared as they are.
fn own(host: &WebHost, name: &str, args: serde_json::Value) -> Option<Reply> {
    #[derive(Deserialize)]
    struct Place {
        destination: Option<Destination>,
    }
    #[derive(Deserialize)]
    struct NewSettings {
        settings: Settings,
    }
    #[derive(Deserialize)]
    struct What {
        what: Export,
    }
    #[derive(Deserialize)]
    struct MapId {
        id: u32,
    }
    #[derive(Deserialize)]
    struct LiveSession {
        id: Option<i64>,
        session: leafline_core::session::SessionSummary,
    }
    let told = |result: Result<(Nav, bool)>| {
        reply_result(result.map(|(nav, changed)| {
            if changed {
                emit(host, NAV_EVENT, &nav);
            }
            nav
        }))
    };
    let place = |args: serde_json::Value| parse::<Place>(args);
    Some(match name {
        "version" => reply(version()),
        "picture_index" => reply(host.map.pictures().table()),
        "map_picture" => match parse::<MapId>(args) {
            Ok(MapId { id }) => reply_result(map_picture(host, id)),
            Err(err) => Err(err),
        },
        "destination" => reply(host.nav.get()),
        "set_destination" => match place(args) {
            Ok(p) => told(host.nav.set_destination(&host.files, p.destination)),
            Err(err) => Err(err),
        },
        "arrive_at" | "forget_destination" => match place(args) {
            Ok(Place { destination: Some(d) }) if name == "arrive_at" => told(host.nav.arrive_at(&host.files, d)),
            Ok(Place { destination: Some(d) }) => told(host.nav.forget_destination(&host.files, d)),
            Ok(_) => Err("A destination is needed.".into()),
            Err(err) => Err(err),
        },
        "get_settings" => reply(host.settings.get()),
        "update_settings" => match parse::<NewSettings>(args) {
            Ok(NewSettings { settings }) => reply_result(host.settings.replace(&host.files, settings).map(|(_, now)| {
                emit(host, SETTINGS_EVENT, &now);
                now
            })),
            Err(err) => Err(err),
        },
        // The page saves the text as a download.
        "export_csv" => match parse::<What>(args) {
            Ok(What { what }) => reply_result(records::csv(host, what).map(|(text, rows)| serde_json::json!({ "text": text, "rows": rows }))),
            Err(err) => Err(err),
        },
        // A session read from a shared game window (platform/live.svelte.ts), saved as it goes
        // the way the program's live loop saves its own: first as a new log entry, then over it.
        "save_live_session" => match parse::<LiveSession>(args) {
            Ok(LiveSession { id, session }) => reply_result(save_live_session(host, id, &session)),
            Err(err) => Err(err),
        },
        // Map data's daily check, which the program runs in the background at start.
        "update_map_data" => {
            if mapdata::wants_updates(&host.map)
                && let Some(status) = mapdata::update_in_background(&host.map)
            {
                emit(host, MAP_DATA_EVENT, &status);
            }
            reply(())
        }
        _ => return None,
    })
}

/// Saves or updates a live session; its log id.
fn save_live_session(host: &WebHost, id: Option<i64>, session: &leafline_core::session::SessionSummary) -> Result<i64> {
    let now = leafline_core::clock::now();
    let id = host.records.with(|db| match id {
        Some(id) => db.update_session(id, session, now).map(|()| id),
        None => db.save_session(session, "", now),
    })?;
    emit(host, records::HISTORY_EVENT, &());
    Ok(id)
}

/// A map's picture cut to what is drawn, as the program's map cards show it, with its portals, NPCs
/// and spawns placed on it: downloaded from OSMS (the browser keeps it in its cache), cut by the
/// shared code, and handed to the page as a JPEG. `None` with pictures off or none in the data.
fn map_picture(host: &WebHost, id: u32) -> Result<Option<serde_json::Value>> {
    use base64::Engine;
    use leafline_core::mapdata::fetch::Source;
    if !host.settings.get().show_pictures {
        return Ok(None);
    }
    if let Some(done) = MAP_VIEWS.with(|v| v.borrow().get(&id).cloned()) {
        return Ok(Some(done));
    }
    let Some(url) = host.map.pictures().resolve(&format!("/map/{id}")).map(str::to_string) else { return Ok(None) };
    let Some(atlas) = host.map.atlas() else { return Ok(None) };
    let points = host.map.map_points(id)?;
    let spots: Vec<(i32, i32)> = points.iter().map(|p| (p.x, p.y)).collect();
    let bytes = Xhr::new(Duration::ZERO).get(&url)?;
    let (jpeg, view) = leafline_core::mapdata::pictures::cut_map(&bytes, &spots)?;
    let done = serde_json::json!({
        "width": view.out_width,
        "height": view.out_height,
        "points": atlas.view_points(id, &points, &view),
        "jpeg": base64::engine::general_purpose::STANDARD.encode(jpeg),
    });
    MAP_VIEWS.with(|v| v.borrow_mut().insert(id, done.clone()));
    Ok(Some(done))
}

fn remove(path: &Path) -> Result<()> {
    let pool = POOL.with(|p| p.borrow().clone()).ok_or_else(|| Error::Invalid("storage isn't open".into()))?;
    pool.remove(&path.to_string_lossy()).map(|_| ()).map_err(|err| Error::Invalid(err.to_string()))
}

/// A copy of every record, as a `.leafline` file's bytes for the page to save.
#[wasm_bindgen]
pub fn export_records() -> std::result::Result<Vec<u8>, JsValue> {
    let host = host()?;
    let pool = pool()?;
    let _ = pool.remove(EXPORT);
    host.records.export_to(Path::new(EXPORT)).map_err(js)?;
    let bytes = pool.export_db(EXPORT).map_err(js);
    let _ = pool.remove(EXPORT);
    bytes
}

/// What a records file holds, checked before anything is replaced. Leaves the file in storage
/// for [`import_records`].
#[wasm_bindgen]
pub fn check_records(bytes: &[u8]) -> std::result::Result<String, JsValue> {
    let pool = pool()?;
    let _ = pool.remove(IMPORT);
    let _ = pool.remove(&format!("{IMPORT}-journal"));
    pool.import_db(IMPORT, bytes).map_err(|_| JsValue::from_str("That file isn't a Leafline records file."))?;
    let summary = leafline_core::userdata::UserData::open_file(Path::new(IMPORT)).and_then(|db| db.summary()).map_err(js)?;
    serde_json::to_string(&summary).map_err(js)
}

/// Replaces the records with the file [`check_records`] took in. The records as they were stay
/// in storage until the next import.
#[wasm_bindgen]
pub fn import_records() -> std::result::Result<String, JsValue> {
    let host = host()?;
    let summary = host
        .records
        .replace_from(Path::new(RECORDS), Path::new(IMPORT), Path::new(BEFORE_IMPORT), &remove)
        .map_err(js)?;
    let _ = remove(Path::new(IMPORT));
    host.events.borrow_mut().clear();
    emit(&*host, RECORDS_REPLACED_EVENT, &summary);
    let events = std::mem::take(&mut *host.events.borrow_mut());
    Ok(serde_json::json!({ "value": summary, "events": events }).to_string())
}

/// What the records hold now.
#[wasm_bindgen]
pub fn records_summary() -> std::result::Result<String, JsValue> {
    let host = host()?;
    let summary = host.records.with(|db| db.summary()).map_err(js)?;
    serde_json::to_string(&summary).map_err(js)
}
