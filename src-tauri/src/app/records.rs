//! Commands for what the player records: the price log, saved sessions and CSV exports. The
//! records live in the user's own database and are never sent anywhere.

use std::sync::{Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::error::{Error, Result};
use crate::userdata::UserData;
use crate::userdata::characters::KnownMesos;
use crate::userdata::drops::{LoggedDrop, NewDrop};
use crate::userdata::notes::{ItemNote, MapNote};
use crate::userdata::potions::{PotionSlot, SessionPotion};
use crate::userdata::sessions::{LevelExp, MapRate, SavedSession, SessionTimeline};
use crate::userdata::spots::SavedSpot;

const FILE_NAME: &str = "leafline.sqlite3";
const BACKUP_FOLDER: &str = "backups";

/// Emitted to every window when a session is saved or deleted.
pub const HISTORY_EVENT: &str = "history";

/// Emitted to every window when a character's mesos are remembered.
pub const MESOS_EVENT: &str = "known-mesos";

/// The user's records, or why they couldn't be opened. A damaged file must not stop the app;
/// the commands report it instead.
pub struct UserDataState(Mutex<std::result::Result<UserData, String>>);

impl UserDataState {
    pub fn load(app: &AppHandle) -> Result<Self> {
        let path = app.path().app_data_dir()?.join(FILE_NAME);
        let opened = UserData::open(&path).map_err(|err| {
            eprintln!("user data unreadable: {err}");
            format!("Your saved records ({}) could not be opened: {err}", path.display())
        });
        if let (Ok(db), Some(dir)) = (&opened, path.parent()) {
            let day = date(now() as f64);
            let day = day.split(' ').next().unwrap_or_default();
            if let Err(err) = db.back_up(&dir.join(BACKUP_FOLDER), day) {
                eprintln!("no backup of the records today: {err}");
            }
        }
        Ok(Self(Mutex::new(opened)))
    }

    pub fn with<T>(&self, f: impl FnOnce(&mut UserData) -> Result<T>) -> Result<T> {
        let mut guard: MutexGuard<_> = self.0.lock().expect("user data lock");
        match guard.as_mut() {
            Ok(db) => f(db),
            Err(message) => Err(Error::Invalid(message.clone())),
        }
    }
}

/// The mesos last read for a character, or with no name, for the one seen last.
#[tauri::command]
pub fn known_mesos(state: State<UserDataState>, name: Option<String>) -> Result<Option<KnownMesos>> {
    state.with(|db| db.known_mesos(name.as_deref()))
}

/// Emitted to every window when the saved training spots change, with the new list.
const SPOTS_EVENT: &str = "saved-spots";

/// The training spots the player saved, the newest first.
#[tauri::command]
pub fn saved_spots(state: State<UserDataState>) -> Result<Vec<SavedSpot>> {
    state.with(|db| db.saved_spots())
}

/// Saves a map as a training spot, or with `saved` false takes it off the list.
#[tauri::command]
pub fn set_spot_saved(
    app: AppHandle,
    state: State<UserDataState>,
    map_id: i64,
    name: String,
    saved: bool,
) -> Result<Vec<SavedSpot>> {
    let spots = state.with(|db| {
        if saved { db.save_spot(map_id, &name, now())? } else { db.remove_spot(map_id)? }
        db.saved_spots()
    })?;
    app.emit(SPOTS_EVENT, &spots)?;
    Ok(spots)
}

/// Emitted to every window when the potion slots or a session's potion counts change.
const POTIONS_EVENT: &str = "potions";

/// The potion slots the player set up, in slot order.
#[tauri::command]
pub fn potion_slots(state: State<UserDataState>, character: Option<String>) -> Result<Vec<PotionSlot>> {
    state.with(|db| db.potion_slots(character.as_deref()))
}

/// Replaces the potion slots and returns them with their numbers.
#[tauri::command]
pub fn set_potion_slots(
    app: AppHandle,
    state: State<UserDataState>,
    character: Option<String>,
    slots: Vec<PotionSlot>,
) -> Result<Vec<PotionSlot>> {
    let kept = state.with(|db| {
        db.set_potion_slots(character.as_deref(), &slots)?;
        db.potion_slots(character.as_deref())
    })?;
    app.emit(POTIONS_EVENT, ())?;
    Ok(kept)
}

/// The potion counts typed for the session that started at `started_at`.
#[tauri::command]
pub fn session_potions(state: State<UserDataState>, started_at: f64) -> Result<Vec<SessionPotion>> {
    state.with(|db| db.session_potions(started_at))
}

/// Records how many of a slot's potion the player had at the start of a session and has now.
#[tauri::command]
pub fn set_session_potion(
    app: AppHandle,
    state: State<UserDataState>,
    character: Option<String>,
    started_at: f64,
    slot: u32,
    start: Option<i64>,
    now: Option<i64>,
) -> Result<Vec<SessionPotion>> {
    let kept = state.with(|db| {
        db.set_session_potion(character.as_deref(), started_at, slot, start, now)?;
        db.session_potions(started_at)
    })?;
    app.emit(POTIONS_EVENT, ())?;
    // Saved sessions carry their potion cost.
    app.emit(HISTORY_EVENT, ())?;
    Ok(kept)
}

/// The player's note on an item, if they wrote one.
#[tauri::command]
pub fn item_note(state: State<UserDataState>, item: String) -> Result<Option<ItemNote>> {
    state.with(|db| db.item_note(&item))
}

/// Writes the player's note on an item; an empty one deletes it. Returns what is kept.
#[tauri::command]
pub fn set_item_note(state: State<UserDataState>, item: String, note: String) -> Result<Option<ItemNote>> {
    state.with(|db| db.set_item_note(&item, &note, now()))
}

/// Emitted to every window when a map note changes, with the map's id.
const MAP_NOTE_EVENT: &str = "map-note";

/// The player's note on a map, if they wrote one.
#[tauri::command]
pub fn map_note(state: State<UserDataState>, map_id: i64) -> Result<Option<MapNote>> {
    state.with(|db| db.map_note(map_id))
}

/// Writes the player's note on a map; an empty one deletes it. Returns what is kept.
#[tauri::command]
pub fn set_map_note(
    app: AppHandle,
    state: State<UserDataState>,
    map_id: i64,
    map: String,
    note: String,
) -> Result<Option<MapNote>> {
    let kept = state.with(|db| db.set_map_note(map_id, &map, &note, now()))?;
    app.emit(MAP_NOTE_EVENT, map_id)?;
    Ok(kept)
}

/// Emitted to every window when the drop log changes.
const DROPS_EVENT: &str = "drops";

/// Every drop the player logged, the newest first.
#[tauri::command]
pub fn drop_log(state: State<UserDataState>) -> Result<Vec<LoggedDrop>> {
    state.with(|db| db.drops())
}

/// Logs a drop the player got and says which monster dropped. Returns its id.
#[tauri::command]
pub fn log_drop(app: AppHandle, state: State<UserDataState>, drop: NewDrop) -> Result<i64> {
    let id = state.with(|db| db.log_drop(&drop, now()))?;
    app.emit(DROPS_EVENT, ())?;
    Ok(id)
}

#[tauri::command]
pub fn delete_drop(app: AppHandle, state: State<UserDataState>, id: i64) -> Result<()> {
    state.with(|db| db.delete_drop(id))?;
    app.emit(DROPS_EVENT, ())?;
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct History {
    sessions: Vec<SavedSession>,
    /// Each map's rates over every saved session.
    maps: Vec<MapRate>,
}

#[tauri::command]
pub fn session_history(state: State<UserDataState>) -> Result<History> {
    state.with(|db| Ok(History { sessions: db.sessions()?, maps: db.map_rates()? }))
}

/// The EXP each level takes, as far as the player's HUD has shown it.
#[tauri::command]
pub fn level_exp(state: State<UserDataState>) -> Result<Vec<LevelExp>> {
    state.with(|db| db.level_exp())
}

/// Opens the training log: a tab of the desk view.
#[tauri::command]
pub async fn show_training(app: AppHandle) -> Result<()> {
    super::desk::open_tab(&app, "log")
}

/// How a saved session's rates moved over time.
#[tauri::command]
pub fn session_timeline(state: State<UserDataState>, id: i64) -> Result<SessionTimeline> {
    state.with(|db| db.session_timeline(id))
}

#[tauri::command]
pub fn delete_session(app: AppHandle, state: State<UserDataState>, id: i64) -> Result<()> {
    state.with(|db| db.delete_session(id))?;
    app.emit(HISTORY_EVENT, ())?;
    Ok(())
}

#[tauri::command]
pub fn set_session_note(app: AppHandle, state: State<UserDataState>, id: i64, note: String) -> Result<()> {
    state.with(|db| db.set_session_note(id, &note))?;
    app.emit(HISTORY_EVENT, ())?;
    Ok(())
}

/// Corrects a misread map name, in what is saved and whenever it is read that way again.
#[tauri::command]
pub fn rename_map(app: AppHandle, state: State<UserDataState>, from: String, to: String) -> Result<()> {
    state.with(|db| db.rename_map(&from, &to))?;
    app.emit(HISTORY_EVENT, ())?;
    Ok(())
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Export {
    Prices,
    Sessions,
    Drops,
}

/// Writes the price log, the session history or the drop log as CSV to `path`, which the user picked in a
/// save dialog. Returns how many rows were written.
#[tauri::command]
pub fn export_csv(state: State<UserDataState>, what: Export, path: std::path::PathBuf) -> Result<usize> {
    let (header, rows) = state.with(|db| {
        Ok(match what {
            Export::Prices => (PRICE_COLUMNS, db.all_prices()?.into_iter().map(price_row).collect::<Vec<_>>()),
            Export::Sessions => (SESSION_COLUMNS, db.sessions()?.into_iter().flat_map(session_rows).collect()),
            Export::Drops => (DROP_COLUMNS, db.drops()?.into_iter().map(drop_row).collect()),
        })
    })?;
    let mut csv = header.join(",") + "\r\n";
    for row in &rows {
        csv += &row.iter().map(|field| csv_field(field)).collect::<Vec<_>>().join(",");
        csv += "\r\n";
    }
    std::fs::write(path, csv)?;
    Ok(rows.len())
}

const PRICE_COLUMNS: &[&str] = &["date", "item", "upgrades", "slots_left", "stats", "price", "kind", "note"];
const SESSION_COLUMNS: &[&str] = &[
    "started",
    "label",
    "map",
    "level",
    "party",
    "class",
    "active_minutes",
    "exp_gained",
    "exp_per_hour",
    "percent_gained",
    "mesos_gained",
    "mesos_per_hour",
    "note",
];

/// The largest picture `save_image` writes: a share card is well under 1 MB.
const MAX_IMAGE: usize = 8 * 1024 * 1024;

/// Writes a picture the app drew, such as a session's share card, to `path`, which the player
/// picked in a save dialog. Only PNG files.
#[tauri::command]
pub fn save_image(path: std::path::PathBuf, png: Vec<u8>) -> Result<()> {
    if !path.extension().is_some_and(|e| e.eq_ignore_ascii_case("png")) {
        return Err(Error::Invalid("Save the picture as a .png file.".into()));
    }
    if png.len() > MAX_IMAGE || !png.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err(Error::Invalid("That isn't a picture Leafline drew.".into()));
    }
    std::fs::write(path, png)?;
    Ok(())
}

const DROP_COLUMNS: &[&str] = &["date", "item", "monster", "map", "character"];

fn drop_row(d: LoggedDrop) -> Vec<String> {
    vec![
        date(d.seen_at as f64),
        d.item,
        d.monster,
        d.map.unwrap_or_default(),
        d.character.unwrap_or_default(),
    ]
}

fn price_row(p: crate::userdata::prices::PriceEntry) -> Vec<String> {
    vec![
        date(p.recorded_at as f64),
        p.item,
        opt(p.upgrades),
        opt(p.slots),
        p.stats,
        p.price.to_string(),
        format!("{:?}", p.kind).to_lowercase(),
        p.note.unwrap_or_default(),
    ]
}

/// A row for the whole session, then one for each map, level, party size and class.
fn session_rows(s: SavedSession) -> Vec<Vec<String>> {
    let started = date(s.started_at.unwrap_or(s.saved_at as f64));
    let rate = |rate: Option<f64>| rate.map(|r| format!("{r:.0}")).unwrap_or_default();
    let mut rows = vec![vec![
        started.clone(),
        s.label.clone(),
        "(all maps)".into(),
        opt(s.level),
        String::new(),
        s.job.clone().unwrap_or_default(),
        format!("{:.1}", s.active_seconds / 60.0),
        s.exp_gained.to_string(),
        rate(s.exp_per_hour),
        String::new(),
        s.mesos_gained.to_string(),
        rate(s.mesos_per_hour),
        s.note.clone(),
    ]];
    for line in s.lines {
        let hours = line.active_seconds / 3600.0;
        let mesos_hours = line.mesos_seconds / 3600.0;
        rows.push(vec![
            started.clone(),
            s.label.clone(),
            line.map.unwrap_or_else(|| "(between maps)".into()),
            opt(line.level),
            opt(line.party),
            line.job.unwrap_or_default(),
            format!("{:.1}", line.active_seconds / 60.0),
            line.exp_gained.to_string(),
            if hours > 0.0 { format!("{:.0}", line.exp_gained as f64 / hours) } else { String::new() },
            format!("{:.2}", line.percent_gained),
            line.mesos_gained.to_string(),
            if mesos_hours > 0.0 { format!("{:.0}", line.mesos_gained as f64 / mesos_hours) } else { String::new() },
            String::new(),
        ]);
    }
    rows
}

/// Quotes a field when it holds a comma, quote or line break. A text field starting with `=`,
/// `+`, `-` or `@` gets a leading `'` so a spreadsheet never runs it as a formula.
fn csv_field(field: &str) -> String {
    let field = if field.starts_with(['=', '+', '-', '@']) && field.parse::<f64>().is_err() {
        format!("'{field}")
    } else {
        field.to_string()
    };
    if field.contains([',', '"', '\n', '\r']) { format!("\"{}\"", field.replace('"', "\"\"")) } else { field }
}

fn opt(value: Option<u32>) -> String {
    value.map(|v| v.to_string()).unwrap_or_default()
}

/// `2026-10-29 14:05` in UTC, which spreadsheets read as a date.
fn date(unix: f64) -> String {
    let secs = unix as i64;
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02} {:02}:{:02}", rem / 3600, rem % 3600 / 60)
}

pub fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_fields_are_quoted_and_defused() {
        assert_eq!(csv_field("plain"), "plain");
        assert_eq!(csv_field("a, b"), "\"a, b\"");
        assert_eq!(csv_field("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(csv_field("=SUM(A1)"), "'=SUM(A1)");
        assert_eq!(csv_field("-5"), "-5");
    }

    #[test]
    fn dates_are_utc_calendar_dates() {
        assert_eq!(date(0.0), "1970-01-01 00:00");
        assert_eq!(date(1_793_300_000.0), "2026-10-29 18:53");
        assert_eq!(date(951_782_400.0), "2000-02-29 00:00");
    }
}
