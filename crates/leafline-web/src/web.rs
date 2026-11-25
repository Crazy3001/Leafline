//! The worker's side: opening storage, running commands, records files.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

use leafline_core::clock::Duration;
use leafline_core::error::{Error, Result};
use leafline_core::pipeline::CharacterScan;
use leafline_core::readers::questlog::QuestLogReading;
use leafline_core::service::bag::{self, BAG_EVENT, BagCheck, BagRead, Pictures, SeenBag};
use leafline_core::service::builds;
use leafline_core::service::damage::DamageState;
use leafline_core::service::questwindow::{QuestWindow, Strictness};
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
    /// Item pictures downloaded this visit for sell or keep, decoded, by address (the browser's
    /// cache keeps the files themselves).
    static ITEM_PICTURES: RefCell<HashMap<String, image::RgbaImage>> = RefCell::new(HashMap::new());
    /// The last inventory tab read and its check, for "Look further" and a page that opens later.
    static LAST_BAG: RefCell<(Option<BagRead>, Option<BagCheck>)> = const { RefCell::new((None, None)) };
    /// What the HUD showed of the character, written down as live reading goes.
    static CHARACTER: RefCell<records::CharacterMemory> = RefCell::new(records::CharacterMemory::default());
    /// The Quest window's views put together, kept strictly (see `service::questwindow`).
    static QUEST_WINDOW: RefCell<Option<QuestWindow>> = const { RefCell::new(None) };
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
    struct Scan {
        scan: CharacterScan,
    }
    #[derive(Deserialize)]
    struct Bag {
        bag: SeenBag,
    }
    #[derive(Deserialize)]
    struct QuestLog {
        name: String,
        reading: QuestLogReading,
        fresh: bool,
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
        // The game's windows, read by the live reading when the player holds them open
        // (platform/live.ts): the build moved along, the bag checked, the quest log kept.
        "catch_up_character" => match parse::<Scan>(args) {
            Ok(Scan { scan }) => reply_result(builds::catch_up_clearly(host, scan).inspect(|read| {
                if read.changed() {
                    emit(host, builds::CHARACTER_READ_EVENT, &read.clone().on_its_own());
                }
            })),
            Err(err) => Err(err),
        },
        "check_bag" => match parse::<Bag>(args) {
            Ok(Bag { bag }) => match bag.read() {
                Some(read) => reply(check_bag(host, read, false)),
                None => Err("That isn't an inventory tab.".into()),
            },
            Err(err) => Err(err),
        },
        // The pictures a check would try, for the page to fetch together first: `bag` given, or
        // the last tab read, every item of it (`wide`, for "Look further").
        "bag_wants" => {
            #[derive(Deserialize)]
            struct Wants {
                bag: Option<SeenBag>,
                #[serde(default)]
                wide: bool,
            }
            match parse::<Wants>(args) {
                Ok(Wants { bag, wide }) => {
                    let read = bag.and_then(|b| b.read()).or_else(|| LAST_BAG.with(|b| b.borrow().0.clone()));
                    let urls = read.map_or(Ok(Vec::new()), |read| bag::wanted(host, &Downloads, &read, wide));
                    reply_result(urls.map(|wanted| {
                        wanted.into_iter().map(|(_, url)| url).filter(|url| !Downloads.has(url)).collect::<Vec<_>>()
                    }))
                }
                Err(err) => Err(err),
            }
        }
        "bag_look_further" => match LAST_BAG.with(|b| b.borrow().0.clone()) {
            Some(read) => reply(check_bag(host, read, true)),
            None => Err("Open your inventory in game first.".into()),
        },
        "last_bag" => reply(LAST_BAG.with(|b| b.borrow().1.clone())),
        // "This misread": what was read, put the way the program saves it beside the picture.
        "misread_report" => {
            #[derive(Deserialize)]
            struct Read {
                analysis: Option<serde_json::Value>,
                session: Option<serde_json::Value>,
                sight: Option<serde_json::Value>,
            }
            match parse::<Read>(args) {
                Ok(Read { analysis, session, sight }) => reply(leafline_core::sight::misread_report(
                    &version(),
                    leafline_core::clock::now_f64(),
                    analysis,
                    session,
                    sight,
                )),
                Err(err) => Err(err),
            }
        }
        // What the live reading's HUD showed of the character (platform/live.ts), remembered as the
        // program's live loop does, for showing until the game is read again.
        "note_character" => {
            #[derive(Deserialize)]
            struct Seen {
                seen: Option<leafline_core::userdata::characters::SeenCharacter>,
            }
            match parse::<Seen>(args) {
                Ok(Seen { seen }) => reply_result(CHARACTER.with(|c| c.borrow_mut().note(host, seen))),
                Err(err) => Err(err),
            }
        }
        "follow_quest_log" => match parse::<QuestLog>(args) {
            Ok(QuestLog { name, reading, fresh }) => reply_result(QUEST_WINDOW.with(|w| {
                let mut w = w.borrow_mut();
                let window = w.get_or_insert_with(|| QuestWindow::new(Strictness::Matched));
                if fresh {
                    window.closed();
                }
                window.keep(host, &name, &reading)
            })),
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

/// Checks an inventory tab, keeps it and its result for later, and tells the page.
fn check_bag(host: &WebHost, read: BagRead, wide: bool) -> BagCheck {
    let check = bag::name_items(host, &Downloads, &read, wide).unwrap_or_else(|err| BagCheck::failed(err.to_string()));
    LAST_BAG.with(|b| *b.borrow_mut() = (Some(read), Some(check.clone())));
    // Kept as the character's last check; a failure to keep it shouldn't spoil the check.
    let _ = records::keep_bag(host, &check);
    emit(host, BAG_EVENT, &check);
    check
}

/// Item pictures from OSMS, downloaded once a visit (the browser's cache keeps them between).
struct Downloads;

impl Pictures for Downloads {
    fn has(&self, url: &str) -> bool {
        ITEM_PICTURES.with(|p| p.borrow().contains_key(url))
    }

    fn load(&self, wanted: Vec<(u32, String)>) -> Vec<(u32, image::RgbaImage)> {
        use leafline_core::mapdata::fetch::Source;
        let xhr = Xhr::new(Duration::ZERO);
        wanted
            .into_iter()
            .filter_map(|(id, url)| {
                if let Some(image) = ITEM_PICTURES.with(|p| p.borrow().get(&url).cloned()) {
                    return Some((id, image));
                }
                if !leafline_core::mapdata::pictures::allowed(&url) {
                    return None;
                }
                let image = image::load_from_memory(&xhr.get(&url).ok()?).ok()?.to_rgba8();
                ITEM_PICTURES.with(|p| p.borrow_mut().insert(url, image.clone()));
                Some((id, image))
            })
            .collect()
    }
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
