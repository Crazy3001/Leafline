//! What the player records themselves: prices they saw or paid, and sessions they saved. Kept in
//! its own SQLite file, apart from the map data cache, because that cache can be thrown away and
//! downloaded again while these records cannot.

pub mod prices;
pub mod sessions;

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
];

pub struct UserData {
    conn: Connection,
}

impl UserData {
    pub fn open(path: &Path) -> Result<Self> {
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
}
