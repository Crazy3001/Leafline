//! The player's own notes on items, one per item, shown on the item check and in Lookup. Like the
//! price log, nothing here comes from a market or a server: it is what the player wrote.

use rusqlite::{OptionalExtension, params};
use serde::Serialize;
use strsim::normalized_levenshtein;

use super::UserData;
use super::prices::SAME_ITEM;
use crate::error::{Error, Result};
use crate::mapdata::drops::item_key;

/// Longer than anyone types beside an item; keeps a pasted wall of text out of the records.
const MAX_NOTE: usize = 500;

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
    fn an_empty_note_deletes_it_and_bad_ones_are_refused() {
        let mut db = UserData::open_in_memory().unwrap();
        db.set_item_note("Fern Dagger", "Gift for Mossbank", 1).unwrap();
        assert_eq!(db.set_item_note("Fern Dagger", "   ", 2).unwrap(), None);
        assert_eq!(db.item_note("Fern Dagger").unwrap(), None);

        assert!(db.set_item_note("  ", "note", 3).is_err());
        assert!(db.set_item_note("Fern Dagger", &"x".repeat(MAX_NOTE + 1), 3).is_err());
    }
}
