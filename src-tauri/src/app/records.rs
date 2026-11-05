//! Commands for what the player records: the price log, saved sessions and CSV exports. The
//! records live in the user's own database and are never sent anywhere.

use std::sync::{Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};

use crate::error::{Error, Result};
use crate::userdata::UserData;
use crate::userdata::characters::KnownMesos;
use crate::userdata::notes::ItemNote;
use crate::userdata::prices::{NewPrice, PriceLog};
use crate::userdata::sessions::{MapRate, SavedSession, SessionTimeline};
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

/// What has been logged for an item, summarised for a copy with these upgrades and slots.
#[tauri::command]
pub fn price_log(
    state: State<UserDataState>,
    item: String,
    upgrades: Option<u32>,
    slots: Option<u32>,
) -> Result<PriceLog> {
    state.with(|db| db.price_log(&item, upgrades, slots))
}

/// Records a price and returns the item's updated log.
#[tauri::command]
pub fn record_price(state: State<UserDataState>, price: NewPrice) -> Result<PriceLog> {
    state.with(|db| {
        db.record_price(&price, now())?;
        db.price_log(&price.item, price.upgrades, price.slots)
    })
}

#[tauri::command]
pub fn delete_price(state: State<UserDataState>, id: i64) -> Result<()> {
    state.with(|db| db.delete_price(id))
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

/// Saves the session the overlay is showing to the history. Live sessions save themselves; this
/// is for the last replay.
#[tauri::command]
pub fn save_session(app: AppHandle, state: State<UserDataState>, label: String) -> Result<i64> {
    let session = super::readings::current_session(&app).ok_or_else(|| {
        Error::Invalid("There is no session to save. Start live reading, or replay a folder in the workbench.".into())
    })?;
    let id = state.with(|db| db.save_session(&session, &label, now()))?;
    app.emit(HISTORY_EVENT, ())?;
    Ok(id)
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

/// The training log window's label.
pub const TRAINING: &str = "training";

/// Opens the training log, or brings it forward.
#[tauri::command]
pub async fn show_training(app: AppHandle) -> Result<()> {
    if let Some(window) = app.get_webview_window(TRAINING) {
        window.show()?;
        window.unminimize()?;
        window.set_focus()?;
        return Ok(());
    }
    WebviewWindowBuilder::new(&app, TRAINING, WebviewUrl::App("training".into()))
        .title("Leafline — Training log")
        .inner_size(1180.0, 800.0)
        .min_inner_size(760.0, 520.0)
        .center()
        .background_color(tauri::window::Color(20, 20, 18, 255))
        .additional_browser_args(super::BROWSER_ARGS)
        .build()?;
    Ok(())
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
}

/// Writes the price log or the session history as CSV to `path`, which the user picked in a
/// save dialog. Returns how many rows were written.
#[tauri::command]
pub fn export_csv(state: State<UserDataState>, what: Export, path: std::path::PathBuf) -> Result<usize> {
    let (header, rows) = state.with(|db| {
        Ok(match what {
            Export::Prices => (PRICE_COLUMNS, db.all_prices()?.into_iter().map(price_row).collect::<Vec<_>>()),
            Export::Sessions => (SESSION_COLUMNS, db.sessions()?.into_iter().flat_map(session_rows).collect()),
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
