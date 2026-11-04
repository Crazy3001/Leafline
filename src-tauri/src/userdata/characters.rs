//! What Leafline remembers about each character between runs: for now, the mesos last read for
//! them, so the Stats tab can show an amount before the inventory is opened again.

use rusqlite::{OptionalExtension, params};
use serde::Serialize;

use super::UserData;
use crate::error::Result;

/// A character's meso amount the last time it was on screen.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnownMesos {
    pub name: String,
    pub mesos: u64,
    /// Unix seconds.
    pub seen_at: i64,
}

impl UserData {
    /// Remembers `mesos` as what `name` held at `seen_at`. Names match whatever their case.
    pub fn save_mesos(&mut self, name: &str, mesos: u64, seen_at: i64) -> Result<()> {
        self.conn.execute(
            "INSERT INTO characters (name, mesos, mesos_seen_at) VALUES (?1, ?2, ?3)
             ON CONFLICT (name) DO UPDATE SET name = excluded.name, mesos = excluded.mesos,
                 mesos_seen_at = excluded.mesos_seen_at",
            params![name, mesos as i64, seen_at],
        )?;
        Ok(())
    }

    /// The mesos last seen for `name`, or with no name, for whichever character was seen last.
    pub fn known_mesos(&self, name: Option<&str>) -> Result<Option<KnownMesos>> {
        let row = |row: &rusqlite::Row| {
            Ok(KnownMesos { name: row.get(0)?, mesos: row.get::<_, i64>(1)?.max(0) as u64, seen_at: row.get(2)? })
        };
        let found = match name {
            Some(name) => self
                .conn
                .query_row("SELECT name, mesos, mesos_seen_at FROM characters WHERE name = ?1", [name], row)
                .optional()?,
            None => self
                .conn
                .query_row("SELECT name, mesos, mesos_seen_at FROM characters ORDER BY mesos_seen_at DESC LIMIT 1", [], row)
                .optional()?,
        };
        Ok(found)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remembers_each_characters_mesos_and_who_was_seen_last() {
        let mut db = UserData::open_in_memory().unwrap();
        assert_eq!(db.known_mesos(None).unwrap(), None);

        db.save_mesos("Mossbank", 36_644, 100).unwrap();
        db.save_mesos("Fernwick", 1_200, 200).unwrap();
        db.save_mesos("mossbank", 40_000, 300).unwrap();

        let moss = db.known_mesos(Some("MOSSBANK")).unwrap().unwrap();
        assert_eq!((moss.mesos, moss.seen_at), (40_000, 300));
        assert_eq!(db.known_mesos(Some("Fernwick")).unwrap().unwrap().mesos, 1_200);
        assert_eq!(db.known_mesos(Some("Nobody")).unwrap(), None);
        assert_eq!(db.known_mesos(None).unwrap().unwrap().name, "mossbank");
    }
}
