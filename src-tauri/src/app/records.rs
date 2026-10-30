//! Commands for what the player records: the price log, saved sessions and CSV exports. The
//! records live in the user's own database and are never sent anywhere.

use std::sync::{Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::error::{Error, Result};
use crate::userdata::UserData;
use crate::userdata::prices::{NewPrice, PriceLog};
use crate::userdata::sessions::{MapRate, SavedSession};

const FILE_NAME: &str = "leafline.sqlite3";

/// Emitted to every window when a session is saved or deleted.
pub const HISTORY_EVENT: &str = "history";

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

/// Saves the session the overlay is showing (from the last replay) to the history.
#[tauri::command]
pub fn save_session(app: AppHandle, state: State<UserDataState>, label: String) -> Result<i64> {
    let session = super::readings::current_session(&app)
        .ok_or_else(|| Error::Invalid("There is no session to save. Replay a folder in the workbench first.".into()))?;
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

#[tauri::command]
pub fn delete_session(app: AppHandle, state: State<UserDataState>, id: i64) -> Result<()> {
    state.with(|db| db.delete_session(id))?;
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
const SESSION_COLUMNS: &[&str] =
    &["started", "label", "map", "active_minutes", "exp_gained", "exp_per_hour", "mesos_gained", "mesos_per_hour"];

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

/// A row for the whole session, then one per map.
fn session_rows(s: SavedSession) -> Vec<Vec<String>> {
    let started = date(s.started_at.unwrap_or(s.saved_at as f64));
    let rate = |rate: Option<f64>| rate.map(|r| format!("{r:.0}")).unwrap_or_default();
    let mut rows = vec![vec![
        started.clone(),
        s.label.clone(),
        "(all maps)".into(),
        format!("{:.1}", s.active_seconds / 60.0),
        s.exp_gained.to_string(),
        rate(s.exp_per_hour),
        s.mesos_gained.to_string(),
        rate(s.mesos_per_hour),
    ]];
    for m in s.maps {
        let hours = m.active_seconds / 3600.0;
        rows.push(vec![
            started.clone(),
            s.label.clone(),
            m.map,
            format!("{:.1}", m.active_seconds / 60.0),
            m.exp_gained.to_string(),
            format!("{:.0}", m.exp_gained as f64 / hours),
            m.mesos_gained.to_string(),
            format!("{:.0}", m.mesos_gained as f64 / hours),
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
