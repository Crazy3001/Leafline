//! The website's small JSON files (settings, Go here, read damage, travel), kept in a SQLite file
//! of their own beside the records, in the browser's private storage for this site.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use leafline_core::error::Result;
use leafline_core::service::files::Files;
use rusqlite::{Connection, OptionalExtension};

pub const FILE_NAME: &str = "/leafline-files.sqlite3";

pub struct DbFiles(Mutex<Connection>);

impl DbFiles {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("CREATE TABLE IF NOT EXISTS files (name TEXT PRIMARY KEY, body BLOB NOT NULL)")?;
        Ok(Self(Mutex::new(conn)))
    }
}

impl Files for DbFiles {
    fn read(&self, name: &str) -> Option<Vec<u8>> {
        let conn = self.0.lock().expect("files lock");
        conn.query_row("SELECT body FROM files WHERE name = ?1", [name], |row| row.get(0)).optional().ok().flatten()
    }

    fn write(&self, name: &str, bytes: &[u8]) -> Result<()> {
        let conn = self.0.lock().expect("files lock");
        conn.execute("INSERT OR REPLACE INTO files (name, body) VALUES (?1, ?2)", rusqlite::params![name, bytes])?;
        Ok(())
    }

    fn path(&self, _name: &str) -> Option<PathBuf> {
        None
    }
}
