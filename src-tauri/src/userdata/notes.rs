//! The player's own notes: one per item, shown on the item check and in Lookup, and one per map,
//! shown in the Map tab, the spot list and the training log. Nothing here comes from a market or
//! a server: it is what the player wrote.

use rusqlite::{OptionalExtension, params};
use serde::Serialize;
use strsim::normalized_levenshtein;

use super::UserData;
use super::prices::SAME_ITEM;
use crate::error::{Error, Result};
use crate::mapdata::drops::item_key;

/// Longer than anyone types beside an item; keeps a pasted wall of text out of the records.
const MAX_NOTE: usize = 500;

/// The player's note on a map, such as where to stand or which channel is quiet.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapNote {
    pub map_id: i64,
    /// The map's name when the note was written, should the map data drop it.
    pub map: String,
    pub note: String,
    /// Unix seconds.
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemNote {
    /// The name as first written down.
    pub item: String,
    pub note: String,
    /// Unix seconds.
    pub updated_at: i64,
}

impl UserData {
    /// The note on `item`, matching a slightly misread name to the one it was written for.
    pub fn item_note(&self, item: &str) -> Result<Option<ItemNote>> {
        let Some(key) = self.closest_note_key(&item_key(item))? else { return Ok(None) };
        Ok(self
            .conn
            .query_row("SELECT item, note, updated_at FROM item_notes WHERE item_key = ?1", [&key], |row| {
                Ok(ItemNote { item: row.get(0)?, note: row.get(1)?, updated_at: row.get(2)? })
            })
            .optional()?)
    }

    /// Writes the note on `item`, over the one already there. An empty note deletes it.
    pub fn set_item_note(&mut self, item: &str, note: &str, now: i64) -> Result<Option<ItemNote>> {
        let name = item.trim();
        let note = note.trim();
        if item_key(name).is_empty() {
            return Err(Error::Invalid("The item's name wasn't read, so a note can't be kept for it.".into()));
        }
        if note.chars().count() > MAX_NOTE {
            return Err(Error::Invalid(format!("Keep a note under {MAX_NOTE} characters.")));
        }
        let key = self.closest_note_key(&item_key(name))?.unwrap_or_else(|| item_key(name));
        if note.is_empty() {
            self.conn.execute("DELETE FROM item_notes WHERE item_key = ?1", [&key])?;
            return Ok(None);
        }
        self.conn.execute(
            "INSERT INTO item_notes (item_key, item, note, updated_at) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT (item_key) DO UPDATE SET note = excluded.note, updated_at = excluded.updated_at",
            params![key, name, note, now],
        )?;
        self.item_note(name)
    }

    pub fn map_note(&self, map_id: i64) -> Result<Option<MapNote>> {
        Ok(self
            .conn
            .query_row("SELECT map_id, map, note, updated_at FROM map_notes WHERE map_id = ?1", [map_id], |row| {
                Ok(MapNote { map_id: row.get(0)?, map: row.get(1)?, note: row.get(2)?, updated_at: row.get(3)? })
            })
            .optional()?)
    }

    /// Writes the note on a map, over the one already there. An empty note deletes it.
    pub fn set_map_note(&mut self, map_id: i64, map: &str, note: &str, now: i64) -> Result<Option<MapNote>> {
        let note = note.trim();
        if note.chars().count() > MAX_NOTE {
            return Err(Error::Invalid(format!("Keep a note under {MAX_NOTE} characters.")));
        }
        if note.is_empty() {
            self.conn.execute("DELETE FROM map_notes WHERE map_id = ?1", [map_id])?;
            return Ok(None);
        }
        self.conn.execute(
            "INSERT INTO map_notes (map_id, map, note, updated_at) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT (map_id) DO UPDATE SET map = excluded.map, note = excluded.note, updated_at = excluded.updated_at",
            params![map_id, map.trim(), note, now],
        )?;
        self.map_note(map_id)
    }

    /// The noted item key that `key` names: itself, or the closest one when OCR misread it.
    fn closest_note_key(&self, key: &str) -> Result<Option<String>> {
        if key.is_empty() {
            return Ok(None);
        }
        let mut stmt = self.conn.prepare("SELECT item_key FROM item_notes")?;
        let keys = stmt.query_map([], |row| row.get::<_, String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
        if keys.iter().any(|k| k == key) {
            return Ok(Some(key.to_string()));
        }
        Ok(keys
            .into_iter()
            .map(|k| (normalized_levenshtein(&k, key), k))
            .filter(|(score, _)| *score >= SAME_ITEM)
            .max_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, k)| k))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_one_note_per_item_and_finds_it_under_a_misread_name() {
        let mut db = UserData::open_in_memory().unwrap();
        assert_eq!(db.item_note("Green Robin Hat").unwrap(), None);

        db.set_item_note("Green Robin Hat", "Keep for the 10% DEX scroll", 100).unwrap();
        let note = db.item_note("Green Robin Hal").unwrap().unwrap();
        assert_eq!((note.item.as_str(), note.note.as_str()), ("Green Robin Hat", "Keep for the 10% DEX scroll"));

        // A misread name writes over the same note rather than starting another.
        db.set_item_note("Green Robin Hal", "Sold", 200).unwrap();
        let note = db.item_note("Green Robin Hat").unwrap().unwrap();
        assert_eq!((note.note.as_str(), note.updated_at), ("Sold", 200));

        assert_eq!(db.item_note("Brown Bamboo Hat").unwrap(), None);
    }

    #[test]
    fn keeps_one_note_per_map() {
        let mut db = UserData::open_in_memory().unwrap();
        assert_eq!(db.map_note(7).unwrap(), None);
        db.set_map_note(7, "Mossbank Meadow II", " Left platform is best ", 100).unwrap();
        db.set_map_note(8, "Burrow Den", "Too crowded", 100).unwrap();
        let note = db.map_note(7).unwrap().unwrap();
        assert_eq!(
            (note.map.as_str(), note.note.as_str(), note.updated_at),
            ("Mossbank Meadow II", "Left platform is best", 100)
        );

        db.set_map_note(7, "Mossbank Meadow II", "Channel 4 is quiet", 200).unwrap();
        assert_eq!(db.map_note(7).unwrap().unwrap().note, "Channel 4 is quiet");
        assert_eq!(db.set_map_note(7, "Mossbank Meadow II", "  ", 300).unwrap(), None);
        assert_eq!(db.map_note(7).unwrap(), None);
        assert!(db.map_note(8).unwrap().is_some(), "other maps keep theirs");
        assert!(db.set_map_note(8, "Burrow Den", &"x".repeat(MAX_NOTE + 1), 3).is_err());
    }

    #[test]
    fn an_empty_note_deletes_it_and_bad_ones_are_refused() {
        let mut db = UserData::open_in_memory().unwrap();
        db.set_item_note("Fern Dagger", "Gift for Mossbank", 1).unwrap();
        assert_eq!(db.set_item_note("Fern Dagger", "   ", 2).unwrap(), None);
        assert_eq!(db.item_note("Fern Dagger").unwrap(), None);

        assert!(db.set_item_note("  ", "note", 3).is_err());
        assert!(db.set_item_note("Fern Dagger", &"x".repeat(MAX_NOTE + 1), 3).is_err());
    }
}
