//! What the player records themselves: prices they saw or paid, notes on items, sessions they
//! saved, training spots they picked, builds they planned, drops they logged, the potions they
//! hunt with, each character's last known level, EXP, map, mesos and bag, and each character's quest log as the game's
//! Quest window last showed it. Kept in its own SQLite file, apart from
//! the map data cache, because that cache can be thrown away and downloaded again while these
//! records cannot.

pub mod builds;
pub mod characters;
pub mod drops;
pub mod loot;
pub mod notes;
pub mod potions;
pub mod prices;
pub mod questlog;
pub mod sessions;
pub mod spots;

use std::path::Path;

use rusqlite::Connection;

use crate::error::Result;

/// Each step brings the file from the version before it to its own; steps are never edited
/// once released, only added, so no record is ever dropped by an upgrade.
const MIGRATIONS: &[&str] = &[
    "
CREATE TABLE prices (
    id INTEGER PRIMARY KEY,
    item TEXT NOT NULL,
    item_key TEXT NOT NULL,
    upgrades INTEGER,
    slots INTEGER,
    stats TEXT NOT NULL DEFAULT '',
    price INTEGER NOT NULL,
    kind TEXT NOT NULL,
    note TEXT,
    recorded_at INTEGER NOT NULL);
CREATE INDEX prices_by_item ON prices (item_key);
",
    "
CREATE TABLE sessions (
    id INTEGER PRIMARY KEY,
    label TEXT NOT NULL,
    saved_at INTEGER NOT NULL,
    started_at REAL,
    level INTEGER,
    level_ups INTEGER NOT NULL,
    active_seconds REAL NOT NULL,
    elapsed_seconds REAL NOT NULL,
    exp_gained INTEGER NOT NULL,
    exp_per_hour REAL,
    mesos_gained INTEGER NOT NULL,
    mesos_per_hour REAL);
CREATE TABLE session_maps (
    session_id INTEGER NOT NULL,
    map TEXT NOT NULL,
    active_seconds REAL NOT NULL,
    exp_gained INTEGER NOT NULL,
    mesos_gained INTEGER NOT NULL);
CREATE INDEX session_maps_by_session ON session_maps (session_id);
",
    "
ALTER TABLE sessions ADD COLUMN note TEXT NOT NULL DEFAULT '';
ALTER TABLE sessions ADD COLUMN job TEXT;
ALTER TABLE sessions ADD COLUMN deaths INTEGER NOT NULL DEFAULT 0;
ALTER TABLE sessions ADD COLUMN percent_per_hour REAL;
CREATE TABLE session_lines (
    session_id INTEGER NOT NULL,
    map TEXT,
    level INTEGER,
    party INTEGER,
    job TEXT,
    active_seconds REAL NOT NULL,
    exp_gained INTEGER NOT NULL,
    percent_gained REAL NOT NULL,
    mesos_gained INTEGER NOT NULL,
    mesos_seconds REAL NOT NULL);
CREATE INDEX session_lines_by_session ON session_lines (session_id);
CREATE TABLE map_renames (
    read_as TEXT PRIMARY KEY,
    name TEXT NOT NULL);
",
    "
CREATE TABLE builds (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    character TEXT,
    body TEXT NOT NULL,
    updated_at INTEGER NOT NULL,
    used_at INTEGER NOT NULL);
",
    "
CREATE TABLE characters (
    name TEXT PRIMARY KEY COLLATE NOCASE,
    mesos INTEGER NOT NULL,
    mesos_seen_at INTEGER NOT NULL);
",
    "
CREATE TABLE item_notes (
    item_key TEXT PRIMARY KEY,
    item TEXT NOT NULL,
    note TEXT NOT NULL,
    updated_at INTEGER NOT NULL);
",
    "
CREATE TABLE saved_spots (
    map_id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    saved_at INTEGER NOT NULL);
",
    "
ALTER TABLE sessions ADD COLUMN character TEXT;
CREATE TABLE session_points (
    session_id INTEGER NOT NULL,
    at REAL NOT NULL,
    exp_per_hour REAL,
    percent_per_hour REAL,
    mesos_per_hour REAL,
    level INTEGER,
    map TEXT);
CREATE INDEX session_points_by_session ON session_points (session_id);
CREATE TABLE session_events (
    session_id INTEGER NOT NULL,
    at REAL NOT NULL,
    kind TEXT NOT NULL,
    level INTEGER);
CREATE INDEX session_events_by_session ON session_events (session_id);
",
    "
CREATE TABLE map_notes (
    map_id INTEGER PRIMARY KEY,
    map TEXT NOT NULL,
    note TEXT NOT NULL,
    updated_at INTEGER NOT NULL);
",
    "
CREATE TABLE drops (
    id INTEGER PRIMARY KEY,
    item TEXT NOT NULL,
    item_key TEXT NOT NULL,
    item_id INTEGER,
    monster TEXT NOT NULL,
    monster_id INTEGER,
    map TEXT,
    map_id INTEGER,
    character TEXT,
    seen_at INTEGER NOT NULL);
CREATE INDEX drops_by_time ON drops (seen_at);
",
    "
CREATE TABLE potion_slots (
    slot INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    price INTEGER NOT NULL);
CREATE TABLE session_potions (
    started_at INTEGER NOT NULL,
    slot INTEGER NOT NULL,
    name TEXT NOT NULL,
    price INTEGER NOT NULL,
    start_count INTEGER,
    now_count INTEGER,
    PRIMARY KEY (started_at, slot));
",
    "
CREATE TABLE level_exp (
    level INTEGER PRIMARY KEY,
    needed INTEGER NOT NULL,
    seen_at INTEGER NOT NULL);
",
    "
CREATE TABLE potion_slots_by_character (
    character TEXT NOT NULL DEFAULT '' COLLATE NOCASE,
    slot INTEGER NOT NULL,
    name TEXT NOT NULL,
    price INTEGER NOT NULL,
    PRIMARY KEY (character, slot));
INSERT INTO potion_slots_by_character (character, slot, name, price) SELECT '', slot, name, price FROM potion_slots;
DROP TABLE potion_slots;
ALTER TABLE potion_slots_by_character RENAME TO potion_slots;
",
    "
ALTER TABLE session_lines ADD COLUMN kills INTEGER;
ALTER TABLE session_lines ADD COLUMN other_exp INTEGER NOT NULL DEFAULT 0;
ALTER TABLE session_lines ADD COLUMN deaths INTEGER NOT NULL DEFAULT 0;
ALTER TABLE session_lines ADD COLUMN exp_lost INTEGER NOT NULL DEFAULT 0;
ALTER TABLE session_lines ADD COLUMN gains_missed INTEGER NOT NULL DEFAULT 0;
ALTER TABLE session_lines ADD COLUMN fighter TEXT;
",
    "
ALTER TABLE sessions ADD COLUMN exp_lost INTEGER NOT NULL DEFAULT 0;
ALTER TABLE sessions ADD COLUMN percent_lost REAL NOT NULL DEFAULT 0;
ALTER TABLE session_lines ADD COLUMN percent_lost REAL NOT NULL DEFAULT 0;
UPDATE sessions SET exp_lost = (SELECT COALESCE(SUM(l.exp_lost), 0) FROM session_lines l WHERE l.session_id = sessions.id);
",
    "
ALTER TABLE potion_slots ADD COLUMN quick_slot INTEGER;
ALTER TABLE session_potions ADD COLUMN read_used INTEGER;
",
    "
CREATE TABLE session_pickups (
    started_at INTEGER PRIMARY KEY,
    mesos INTEGER NOT NULL,
    seconds REAL NOT NULL,
    lines INTEGER NOT NULL,
    unread INTEGER NOT NULL,
    unnamed INTEGER NOT NULL);
CREATE TABLE session_loot (
    started_at INTEGER NOT NULL,
    item_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    count INTEGER NOT NULL,
    PRIMARY KEY (started_at, item_id));
",
    "
CREATE TABLE quest_log (
    character TEXT NOT NULL COLLATE NOCASE,
    key TEXT NOT NULL,
    title TEXT NOT NULL,
    cut INTEGER NOT NULL DEFAULT 0,
    quest_id INTEGER,
    region TEXT,
    state TEXT NOT NULL DEFAULT '',
    state_seen_at INTEGER,
    done INTEGER NOT NULL DEFAULT 0,
    done_on TEXT,
    done_seen_at INTEGER,
    on_helper INTEGER,
    icon TEXT,
    level INTEGER,
    counters TEXT,
    talk_to TEXT,
    rewards TEXT,
    reward_exp INTEGER,
    reward_mesos INTEGER,
    detail_seen_at INTEGER,
    PRIMARY KEY (character, key));
CREATE TABLE quest_log_regions (
    character TEXT NOT NULL COLLATE NOCASE,
    tab TEXT NOT NULL,
    region TEXT NOT NULL,
    count INTEGER NOT NULL,
    seen_at INTEGER NOT NULL,
    PRIMARY KEY (character, tab, region));
CREATE TABLE quest_log_looks (
    character TEXT NOT NULL COLLATE NOCASE,
    tab TEXT NOT NULL,
    seen_at INTEGER NOT NULL,
    whole_at INTEGER,
    PRIMARY KEY (character, tab));
",
    "
CREATE TABLE character_profiles (
    name TEXT PRIMARY KEY COLLATE NOCASE,
    level INTEGER,
    job TEXT,
    exp_percent REAL,
    exp_points INTEGER,
    map TEXT,
    seen_at INTEGER);
CREATE TABLE character_bags (
    name TEXT NOT NULL COLLATE NOCASE,
    tab TEXT NOT NULL,
    bag TEXT NOT NULL,
    read_at INTEGER NOT NULL,
    PRIMARY KEY (name, tab));
",
    "
CREATE TABLE character_looks (
    name TEXT PRIMARY KEY COLLATE NOCASE,
    png BLOB NOT NULL,
    width INTEGER NOT NULL,
    height INTEGER NOT NULL,
    read_at INTEGER NOT NULL);
",
];

/// Daily copies of the records that are kept.
const BACKUPS_KEPT: usize = 7;

pub struct UserData {
    conn: Connection,
}

/// How much a records file holds.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordsSummary {
    pub sessions: i64,
    pub drops: i64,
    pub builds: i64,
    pub notes: i64,
    pub characters: i64,
    pub spots: i64,
    pub prices: i64,
    /// When the newest session or drop was saved, Unix seconds; none with neither.
    pub newest: Option<i64>,
}

impl UserData {
    /// Opens a records file the player picked, to bring in. Refuses a file that isn't Leafline's
    /// or comes from a newer Leafline; brings an older one up to date. Open a copy: this changes
    /// the file it opens.
    pub fn open_file(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        let version = conn
            .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
            .map_err(|_| crate::error::Error::Invalid("That file isn't a Leafline records file.".into()))?;
        let has_sessions: bool = conn
            .query_row("SELECT EXISTS (SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'sessions')", [], |row| {
                row.get(0)
            })
            .unwrap_or(false);
        if version < 2 || !has_sessions {
            return Err(crate::error::Error::Invalid("That file isn't a Leafline records file.".into()));
        }
        Self::init(conn)
    }

    /// Writes a copy of every record to `path`, a `.leafline` file the player keeps or moves to
    /// the website or another PC. Map data is never in it: each side downloads its own. `path`
    /// must not exist yet.
    pub fn export_to(&self, path: &Path) -> Result<()> {
        self.conn.execute("VACUUM INTO ?1", [path.to_string_lossy()])?;
        Ok(())
    }

    /// What the records hold, for the player to check before replacing theirs with a file.
    pub fn summary(&self) -> Result<RecordsSummary> {
        let count = |table: &str| -> Result<i64> {
            Ok(self.conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| row.get(0))?)
        };
        Ok(RecordsSummary {
            sessions: count("sessions")?,
            drops: count("drops")?,
            builds: count("builds")?,
            notes: count("item_notes")? + count("map_notes")?,
            characters: count("characters")?,
            spots: count("saved_spots")?,
            prices: count("prices")?,
            newest: self.conn.query_row(
                "SELECT MAX(at) FROM (SELECT MAX(saved_at) AS at FROM sessions UNION ALL SELECT MAX(seen_at) FROM drops)",
                [],
                |row| row.get(0),
            )?,
        })
    }

    pub fn open(path: &Path) -> Result<Self> {
        // The website's files live in its browser storage, which has no folders to make.
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        Self::init(Connection::open(path)?)
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(mut conn: Connection) -> Result<Self> {
        let version = conn.query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))? as usize;
        if version > MIGRATIONS.len() {
            return Err(crate::error::Error::Invalid(
                "Your records were saved by a newer version of Leafline. Update Leafline to open them.".into(),
            ));
        }
        for (i, step) in MIGRATIONS.iter().enumerate().skip(version) {
            let tx = conn.transaction()?;
            tx.execute_batch(step)?;
            tx.pragma_update(None, "user_version", (i + 1) as i64)?;
            tx.commit()?;
        }
        Ok(Self { conn })
    }

    /// Writes today's copy into `folder`, if there isn't one yet, and drops all but the newest
    /// few. The names sort by date.
    pub fn back_up(&self, folder: &Path, day: &str) -> Result<()> {
        std::fs::create_dir_all(folder)?;
        let path = folder.join(format!("leafline-{day}.sqlite3"));
        if !path.exists() {
            self.conn.execute("VACUUM INTO ?1", [path.to_string_lossy()])?;
        }
        let mut copies: Vec<_> = std::fs::read_dir(folder)?
            .filter_map(|entry| Some(entry.ok()?.path()))
            .filter(|p| p.extension().is_some_and(|e| e == "sqlite3"))
            .collect();
        copies.sort();
        let extra = copies.len().saturating_sub(BACKUPS_KEPT);
        for old in &copies[..extra] {
            std::fs::remove_file(old)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_a_copy_a_day_and_only_the_newest_few() {
        let folder = std::env::temp_dir().join(format!("leafline-backup-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        let db = UserData::open_in_memory().unwrap();
        for day in 1..=9 {
            db.back_up(&folder, &format!("2026-10-{day:02}")).unwrap();
        }
        db.back_up(&folder, "2026-10-09").unwrap();
        let mut names: Vec<String> = std::fs::read_dir(&folder)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        assert_eq!(names.len(), BACKUPS_KEPT);
        assert_eq!(names[0], "leafline-2026-10-03.sqlite3");
        std::fs::remove_dir_all(&folder).unwrap();
    }
}
