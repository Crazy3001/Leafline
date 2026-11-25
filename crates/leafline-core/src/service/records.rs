//! Commands for what the player records: the price log, saved sessions and CSV exports. The
//! records live in the user's own database and are never sent anywhere.

use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use serde::{Deserialize, Serialize};

use super::bag::BagCheck;
use super::host::{Host, emit};

use crate::error::{Error, Result};
use crate::session::SessionSummary;
use crate::userdata::{RecordsSummary, UserData};
use crate::userdata::characters::{KnownCharacter, SeenCharacter};
use crate::userdata::drops::{LoggedDrop, NewDrop};
use crate::userdata::notes::{ItemNote, MapNote};
use crate::userdata::potions::{PotionSlot, SessionPotion};
use crate::userdata::sessions::{LevelExp, MapRate, SavedSession, SessionTimeline};
use crate::userdata::spots::SavedSpot;

pub const FILE_NAME: &str = "leafline.sqlite3";
pub const BACKUP_FOLDER: &str = "backups";

/// Emitted to every window when a session is saved or deleted.
pub const HISTORY_EVENT: &str = "history";

/// Emitted to every window when what was last read of a character (level, EXP, map, mesos, bag)
/// is remembered.
pub const KNOWN_EVENT: &str = "known-character";

/// The user's records, or why they couldn't be opened. A damaged file must not stop the app;
/// the commands report it instead.
pub struct UserDataState(Mutex<std::result::Result<UserData, String>>);

impl UserDataState {
    /// Opens the records at `path`, keeping a copy a day in `backups` when given (the program's
    /// data folder; the website saves its records to a file instead).
    pub fn load(path: &Path, backups: Option<&Path>) -> Self {
        let opened = UserData::open(path).map_err(|err| {
            eprintln!("user data unreadable: {err}");
            format!("Your saved records ({}) could not be opened: {err}", path.display())
        });
        if let (Ok(db), Some(folder)) = (&opened, backups) {
            let day = date(now() as f64);
            let day = day.split(' ').next().unwrap_or_default();
            if let Err(err) = db.back_up(folder, day) {
                eprintln!("no backup of the records today: {err}");
            }
        }
        Self(Mutex::new(opened))
    }

    /// Records that couldn't be opened, with why.
    pub fn failed(message: String) -> Self {
        Self(Mutex::new(Err(message)))
    }

    pub fn with<T>(&self, f: impl FnOnce(&mut UserData) -> Result<T>) -> Result<T> {
        let mut guard: MutexGuard<_> = self.0.lock().expect("user data lock");
        match guard.as_mut() {
            Ok(db) => f(db),
            Err(message) => Err(Error::Invalid(message.clone())),
        }
    }

    /// Replaces the records at `path` with those in the records file at `incoming` (a copy the
    /// player picked, already checked with [`UserData::open_file`]). The records as they were
    /// are kept at `previous` first. `remove` deletes a file, the program's or the website's way.
    /// If the new records can't be put in place, the previous ones are.
    pub fn replace_from(
        &self,
        path: &Path,
        incoming: &Path,
        previous: &Path,
        remove: &dyn Fn(&Path) -> Result<()>,
    ) -> Result<RecordsSummary> {
        let mut guard = self.0.lock().expect("user data lock");
        let incoming = UserData::open_file(incoming)?;
        let summary = incoming.summary()?;
        if let Ok(db) = guard.as_ref() {
            let _ = remove(previous);
            db.export_to(previous)?;
        }
        // Close the records before their file goes.
        *guard = Err("Your records are being replaced.".into());
        let journal = path.with_file_name(format!(
            "{}-journal",
            path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
        ));
        let _ = remove(&journal);
        let put = remove(path).and_then(|()| incoming.export_to(path));
        drop(incoming);
        if let Err(err) = put {
            let _ = remove(path);
            if let Ok(before) = UserData::open(previous) {
                let _ = before.export_to(path);
            }
            *guard = UserData::open(path).map_err(|e| e.to_string());
            return Err(err);
        }
        *guard = UserData::open(path).map_err(|err| format!("Your records could not be opened: {err}"));
        Ok(summary)
    }

    /// Writes a copy of the records to `path`, which must not exist yet.
    pub fn export_to(&self, path: &Path) -> Result<RecordsSummary> {
        self.with(|db| {
            db.export_to(path)?;
            db.summary()
        })
    }
}

/// Emitted to every window when the records were replaced from a file: everything shown from
/// them is out of date.
pub const RECORDS_REPLACED_EVENT: &str = "records-replaced";

/// What the records hold now, to set beside a backup or a records file before using it.
pub fn records_summary(h: &dyn Host) -> Result<RecordsSummary> {
    h.records().with(|db| db.summary())
}

/// What was last read of a character (level, job, EXP, map, mesos, each with when), or with no
/// name, of the one seen last. For showing until the game is read again: never for a rate.
pub fn known_character(h: &dyn Host, name: Option<String>) -> Result<Option<KnownCharacter>> {
    h.records().with(|db| db.known_character(name.as_deref()))
}

/// A character's last bag check, or with no name, the one read last. A check that no longer
/// reads as one (from an older Leafline) is left out.
pub fn known_bag(h: &dyn Host, name: Option<String>) -> Result<Option<BagCheck>> {
    let text = h.records().with(|db| db.known_bag(name.as_deref()))?;
    Ok(text.and_then(|t| serde_json::from_str(&t).ok()))
}

/// Keeps a finished bag check as its character's last one. A check with no character, still
/// under way or that failed isn't kept.
pub fn keep_bag(h: &dyn Host, check: &BagCheck) -> Result<()> {
    let Some(name) = check.character.as_deref().filter(|_| !check.busy && check.problem.is_none()) else {
        return Ok(());
    };
    let text = serde_json::to_string(check).map_err(|err| Error::Invalid(err.to_string()))?;
    h.records().with(|db| db.save_bag(name, &text, check.at))?;
    emit(h, KNOWN_EVENT, &());
    Ok(())
}

/// Seconds the mesos may have gone unseen and still count as just read.
const MESOS_FRESH: f64 = 5.0;
/// How often what the HUD shows is written down while only the EXP or mesos move.
const SEEN_EVERY: i64 = 60;

/// What the session shows of the character now, to remember: once its name is settled.
pub fn seen_of(session: &SessionSummary) -> Option<SeenCharacter> {
    let name = session.name.clone()?;
    Some(SeenCharacter {
        name,
        level: session.level,
        job: session.job.clone(),
        exp_percent: session.exp.as_ref().map(|e| e.percent).or(session.percent),
        exp_points: session.exp.as_ref().map(|e| e.points),
        map: session.map.clone().filter(|_| !session.map_hidden),
        mesos: session.mesos.filter(|_| session.mesos_age.is_some_and(|age| age <= MESOS_FRESH)),
    })
}

/// Writes down what the HUD shows of the character as live reading goes: at once when the
/// character, level, job or map changes, else once a minute. Only for showing who they were when
/// the game is next closed; nothing read back from it feeds a rate or a session.
#[derive(Default)]
pub struct CharacterMemory {
    saved: Option<(SeenCharacter, i64)>,
}

impl CharacterMemory {
    /// `true` when it was written down.
    pub fn note(&mut self, h: &dyn Host, seen: Option<SeenCharacter>) -> Result<bool> {
        let Some(seen) = seen else { return Ok(false) };
        let now = now();
        if let Some((last, at)) = &self.saved {
            // A part that wasn't read this time (the minimap hidden, say) isn't a change.
            fn changed<T: PartialEq>(was: &Option<T>, now: &Option<T>) -> bool {
                now.is_some() && was != now
            }
            let moved = last.name != seen.name
                || changed(&last.level, &seen.level)
                || changed(&last.job, &seen.job)
                || changed(&last.map, &seen.map);
            if !moved && now - at < SEEN_EVERY {
                return Ok(false);
            }
        }
        h.records().with(|db| db.save_seen(&seen, now))?;
        self.saved = Some((seen, now));
        emit(h, KNOWN_EVENT, &());
        Ok(true)
    }
}

/// Emitted to every window when the saved training spots change, with the new list.
pub const SPOTS_EVENT: &str = "saved-spots";

/// The training spots the player saved, the newest first.
pub fn saved_spots(h: &dyn Host) -> Result<Vec<SavedSpot>> {
    h.records().with(|db| db.saved_spots())
}

/// Saves a map as a training spot, or with `saved` false takes it off the list.
pub fn set_spot_saved(
    h: &dyn Host,
    map_id: i64,
    name: String,
    saved: bool,
) -> Result<Vec<SavedSpot>> {
    let spots = h.records().with(|db| {
        if saved { db.save_spot(map_id, &name, now())? } else { db.remove_spot(map_id)? }
        db.saved_spots()
    })?;
    emit(h, SPOTS_EVENT, &spots);
    Ok(spots)
}

/// Emitted to every window when the potion slots or a session's potion counts change.
pub const POTIONS_EVENT: &str = "potions";

/// The potion slots the player set up, in slot order.
/// What one of an item picked up is worth: the player's own logged price when there is one,
/// else what an NPC pays going by the item data.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LootPrice {
    pub id: u32,
    /// The NPC price the item data lists. What it means in Classic isn't confirmed.
    pub npc: Option<i64>,
    /// The middle of the prices the player logged for it.
    pub own: Option<u64>,
}

pub fn loot_prices(h: &dyn Host, ids: Vec<u32>) -> Result<Vec<LootPrice>> {
    let catalog = h.map().catalog();
    h.records().with(|db| {
        ids.iter()
            .map(|&id| {
                let item = catalog.as_ref().and_then(|c| c.item(id));
                let own = match item {
                    Some(item) => db.price_log(&item.name, None, None)?.all.map(|s| s.median),
                    None => None,
                };
                Ok(LootPrice { id, npc: item.and_then(|i| i.price), own })
            })
            .collect()
    })
}

pub fn potion_slots(h: &dyn Host, character: Option<String>) -> Result<Vec<PotionSlot>> {
    h.records().with(|db| db.potion_slots(character.as_deref()))
}

/// Replaces the potion slots and returns them with their numbers.
pub fn set_potion_slots(
    h: &dyn Host,
    character: Option<String>,
    slots: Vec<PotionSlot>,
) -> Result<Vec<PotionSlot>> {
    let kept = h.records().with(|db| {
        db.set_potion_slots(character.as_deref(), &slots)?;
        db.potion_slots(character.as_deref())
    })?;
    emit(h, POTIONS_EVENT, &());
    Ok(kept)
}

/// The potion counts typed for the session that started at `started_at`.
pub fn session_potions(h: &dyn Host, started_at: f64) -> Result<Vec<SessionPotion>> {
    h.records().with(|db| db.session_potions(started_at))
}

/// Records how many of a slot's potion the player had at the start of a session and has now.
pub fn set_session_potion(
    h: &dyn Host,
    character: Option<String>,
    started_at: f64,
    slot: u32,
    start: Option<i64>,
    now: Option<i64>,
) -> Result<Vec<SessionPotion>> {
    let kept = h.records().with(|db| {
        db.set_session_potion(character.as_deref(), started_at, slot, start, now)?;
        db.session_potions(started_at)
    })?;
    emit(h, POTIONS_EVENT, &());
    // Saved sessions carry their potion cost.
    emit(h, HISTORY_EVENT, &());
    Ok(kept)
}

/// The player's note on an item, if they wrote one.
pub fn item_note(h: &dyn Host, item: String) -> Result<Option<ItemNote>> {
    h.records().with(|db| db.item_note(&item))
}

/// Writes the player's note on an item; an empty one deletes it. Returns what is kept.
pub fn set_item_note(h: &dyn Host, item: String, note: String) -> Result<Option<ItemNote>> {
    h.records().with(|db| db.set_item_note(&item, &note, now()))
}

/// Emitted to every window when a map note changes, with the map's id.
pub const MAP_NOTE_EVENT: &str = "map-note";

/// The player's note on a map, if they wrote one.
pub fn map_note(h: &dyn Host, map_id: i64) -> Result<Option<MapNote>> {
    h.records().with(|db| db.map_note(map_id))
}

/// Writes the player's note on a map; an empty one deletes it. Returns what is kept.
pub fn set_map_note(
    h: &dyn Host,
    map_id: i64,
    map: String,
    note: String,
) -> Result<Option<MapNote>> {
    let kept = h.records().with(|db| db.set_map_note(map_id, &map, &note, now()))?;
    emit(h, MAP_NOTE_EVENT, &map_id);
    Ok(kept)
}

/// Emitted to every window when the drop log changes.
pub const DROPS_EVENT: &str = "drops";

/// Every drop the player logged, the newest first.
pub fn drop_log(h: &dyn Host) -> Result<Vec<LoggedDrop>> {
    h.records().with(|db| db.drops())
}

/// Logs a drop the player got and says which monster dropped. Returns its id.
pub fn log_drop(h: &dyn Host, drop: NewDrop) -> Result<i64> {
    let id = h.records().with(|db| db.log_drop(&drop, now()))?;
    emit(h, DROPS_EVENT, &());
    Ok(id)
}

pub fn delete_drop(h: &dyn Host, id: i64) -> Result<()> {
    h.records().with(|db| db.delete_drop(id))?;
    emit(h, DROPS_EVENT, &());
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct History {
    sessions: Vec<SavedSession>,
    /// Each map's rates over every saved session.
    maps: Vec<MapRate>,
}

pub fn session_history(h: &dyn Host) -> Result<History> {
    h.records().with(|db| Ok(History { sessions: db.sessions()?, maps: db.map_rates()? }))
}

/// The EXP each level takes, as far as the player's HUD has shown it.
pub fn level_exp(h: &dyn Host) -> Result<Vec<LevelExp>> {
    h.records().with(|db| db.level_exp())
}

/// How a saved session's rates moved over time.
pub fn session_timeline(h: &dyn Host, id: i64) -> Result<SessionTimeline> {
    h.records().with(|db| db.session_timeline(id))
}

pub fn delete_session(h: &dyn Host, id: i64) -> Result<()> {
    h.records().with(|db| db.delete_session(id))?;
    emit(h, HISTORY_EVENT, &());
    Ok(())
}

pub fn set_session_note(h: &dyn Host, id: i64, note: String) -> Result<()> {
    h.records().with(|db| db.set_session_note(id, &note))?;
    emit(h, HISTORY_EVENT, &());
    Ok(())
}

/// Corrects a misread map name, in what is saved and whenever it is read that way again.
pub fn rename_map(h: &dyn Host, from: String, to: String) -> Result<()> {
    h.records().with(|db| db.rename_map(&from, &to))?;
    emit(h, HISTORY_EVENT, &());
    Ok(())
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Export {
    Prices,
    Sessions,
    Drops,
    Loot,
}

/// The price log, the session history or the drop log as CSV, and how many rows it holds.
pub fn csv(h: &dyn Host, what: Export) -> Result<(String, usize)> {
    let (header, rows) = h.records().with(|db| {
        Ok(match what {
            Export::Prices => (PRICE_COLUMNS, db.all_prices()?.into_iter().map(price_row).collect::<Vec<_>>()),
            Export::Sessions => (SESSION_COLUMNS, db.sessions()?.into_iter().flat_map(session_rows).collect()),
            Export::Drops => (DROP_COLUMNS, db.drops()?.into_iter().map(drop_row).collect()),
            Export::Loot => (LOOT_COLUMNS, db.sessions()?.into_iter().flat_map(loot_rows).collect()),
        })
    })?;
    let mut csv = header.join(",") + "\r\n";
    for row in &rows {
        csv += &row.iter().map(|field| csv_field(field)).collect::<Vec<_>>().join(",");
        csv += "\r\n";
    }
    Ok((csv, rows.len()))
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
    "deaths",
    "exp_lost",
    "note",
];

const DROP_COLUMNS: &[&str] = &["date", "item", "monster", "map", "character"];

const LOOT_COLUMNS: &[&str] = &["started", "character", "item", "item_id", "count", "watched_minutes"];

/// One row per item a session picked up, and one for its mesos, oldest session last.
fn loot_rows(s: SavedSession) -> Vec<Vec<String>> {
    let Some(loot) = s.loot else { return Vec::new() };
    let started = s.started_at.map(date).unwrap_or_default();
    let character = s.character.unwrap_or_default();
    let minutes = format!("{:.1}", loot.seconds / 60.0);
    let row = |item: &str, id: String, count: u64| vec![started.clone(), character.clone(), item.to_string(), id, count.to_string(), minutes.clone()];
    std::iter::once(row("Mesos", String::new(), loot.mesos))
        .chain(loot.items.iter().map(|i| row(&i.name, i.id.to_string(), i.count)))
        .collect()
}

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
        s.deaths.to_string(),
        s.exp_lost.to_string(),
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
            line.deaths.to_string(),
            line.exp_lost.to_string(),
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
pub fn date(unix: f64) -> String {
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

pub use crate::clock::now;

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

    fn drop_of(item: &str) -> NewDrop {
        NewDrop {
            item: item.into(),
            item_id: None,
            monster: "Pebble Snail".into(),
            monster_id: None,
            map: None,
            map_id: None,
            character: None,
        }
    }

    #[test]
    fn a_records_file_replaces_the_records_and_keeps_the_old_ones() {
        let dir = std::env::temp_dir().join(format!("leafline-records-swap-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let (path, file, previous) = (dir.join("leafline.sqlite3"), dir.join("in.leafline"), dir.join("before.sqlite3"));
        let remove = |p: &Path| -> Result<()> {
            match std::fs::remove_file(p) {
                Err(err) if err.kind() != std::io::ErrorKind::NotFound => Err(err.into()),
                _ => Ok(()),
            }
        };

        // The file to bring in: two drops, written by another copy of Leafline.
        let other = UserDataState::load(&dir.join("other.sqlite3"), None);
        other.with(|db| db.log_drop(&drop_of("Moss Scroll"), 1)).unwrap();
        other.with(|db| db.log_drop(&drop_of("Twig Wand"), 2)).unwrap();
        other.export_to(&file).unwrap();

        let mine = UserDataState::load(&path, None);
        mine.with(|db| db.log_drop(&drop_of("Shell"), 3)).unwrap();
        let summary = mine.replace_from(&path, &file, &previous, &remove).unwrap();
        assert_eq!(summary.drops, 2);
        assert_eq!(mine.with(|db| db.drops()).unwrap().len(), 2);
        // The records as they were are kept.
        assert_eq!(UserData::open(&previous).unwrap().summary().unwrap().drops, 1);
        // Something that isn't a records file is refused, and nothing changes.
        std::fs::write(dir.join("junk.leafline"), b"not a database").unwrap();
        assert!(mine.replace_from(&path, &dir.join("junk.leafline"), &previous, &remove).is_err());
        assert_eq!(mine.with(|db| db.drops()).unwrap().len(), 2);
        drop((other, mine));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn dates_are_utc_calendar_dates() {
        assert_eq!(date(0.0), "1970-01-01 00:00");
        assert_eq!(date(1_793_300_000.0), "2026-10-29 18:53");
        assert_eq!(date(951_782_400.0), "2000-02-29 00:00");
    }
}
