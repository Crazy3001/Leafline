//! Training spots the player saved: their own picks, listed first on the Train tab. Kept by map
//! id, with the name as it was when saved so the list still reads without the map data.

use rusqlite::params;
use serde::Serialize;

use super::UserData;
use crate::error::{Error, Result};

/// More than anyone keeps as favourites; stops a runaway list.
const MAX_SPOTS: usize = 50;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedSpot {
    pub map_id: i64,
    pub name: String,
    /// Unix seconds.
    pub saved_at: i64,
}

impl UserData {
    /// Every saved spot, the newest first.
    pub fn saved_spots(&self) -> Result<Vec<SavedSpot>> {
        let mut stmt =
            self.conn.prepare("SELECT map_id, name, saved_at FROM saved_spots ORDER BY saved_at DESC, map_id")?;
        let rows = stmt.query_map([], |row| Ok(SavedSpot { map_id: row.get(0)?, name: row.get(1)?, saved_at: row.get(2)? }))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Saves a map as a spot, or renames one already saved without moving it in the list.
    pub fn save_spot(&mut self, map_id: i64, name: &str, now: i64) -> Result<()> {
        let name = name.trim();
        if name.is_empty() {
            return Err(Error::Invalid("A saved spot needs the map's name.".into()));
        }
        let known: bool =
            self.conn.query_row("SELECT EXISTS (SELECT 1 FROM saved_spots WHERE map_id = ?1)", [map_id], |r| r.get(0))?;
        if !known && self.saved_spots()?.len() >= MAX_SPOTS {
            return Err(Error::Invalid(format!("Up to {MAX_SPOTS} spots can be saved. Remove one first.")));
        }
        self.conn.execute(
            "INSERT INTO saved_spots (map_id, name, saved_at) VALUES (?1, ?2, ?3)
             ON CONFLICT (map_id) DO UPDATE SET name = excluded.name",
            params![map_id, name, now],
        )?;
        Ok(())
    }

    pub fn remove_spot(&mut self, map_id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM saved_spots WHERE map_id = ?1", [map_id])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saves_lists_newest_first_and_removes() {
        let mut db = UserData::open_in_memory().unwrap();
        assert!(db.saved_spots().unwrap().is_empty());

        db.save_spot(100, "Mossbank Meadow II", 10).unwrap();
        db.save_spot(200, "Burrow Den", 20).unwrap();
        let names: Vec<_> = db.saved_spots().unwrap().into_iter().map(|s| s.name).collect();
        assert_eq!(names, ["Burrow Den", "Mossbank Meadow II"]);

        // Saving again keeps its place and takes the new name.
        db.save_spot(100, "Mossbank Meadow 2", 30).unwrap();
        let spots = db.saved_spots().unwrap();
        assert_eq!((spots[1].map_id, spots[1].name.as_str(), spots[1].saved_at), (100, "Mossbank Meadow 2", 10));

        db.remove_spot(200).unwrap();
        assert_eq!(db.saved_spots().unwrap().len(), 1);
        assert!(db.save_spot(300, "  ", 40).is_err());
    }

    #[test]
    fn the_list_has_a_limit() {
        let mut db = UserData::open_in_memory().unwrap();
        for id in 0..MAX_SPOTS as i64 {
            db.save_spot(id, "Spot", id).unwrap();
        }
        assert!(db.save_spot(999, "One too many", 999).is_err());
        db.save_spot(0, "Renamed", 1000).unwrap();
    }
}
