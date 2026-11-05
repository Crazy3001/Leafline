//! The player's drop log: items they got and the monster they say dropped it, logged from the
//! item check with one click. Leafline never sees a drop by itself; every row is the player's
//! own word, kept on this PC.

use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::UserData;
use crate::error::{Error, Result};
use crate::mapdata::drops::item_key;

/// Far more than anyone logs by hand; stops a runaway list.
const MAX_DROPS: i64 = 20_000;

/// A drop the player is logging.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewDrop {
    pub item: String,
    /// The map data's id, when the item was matched.
    pub item_id: Option<i64>,
    pub monster: String,
    pub monster_id: Option<i64>,
    /// The map it dropped on, by name, and by id when the map was matched.
    pub map: Option<String>,
    pub map_id: Option<i64>,
    pub character: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoggedDrop {
    pub id: i64,
    pub item: String,
    pub item_id: Option<i64>,
    pub monster: String,
    pub monster_id: Option<i64>,
    pub map: Option<String>,
    pub map_id: Option<i64>,
    pub character: Option<String>,
    /// Unix seconds.
    pub seen_at: i64,
}

impl UserData {
    /// Logs a drop and returns its id.
    pub fn log_drop(&mut self, drop: &NewDrop, now: i64) -> Result<i64> {
        let item = drop.item.trim();
        let monster = drop.monster.trim();
        if item_key(item).is_empty() {
            return Err(Error::Invalid("The item's name wasn't read, so the drop can't be logged.".into()));
        }
        if monster.is_empty() {
            return Err(Error::Invalid("Pick the monster that dropped it.".into()));
        }
        let count: i64 = self.conn.query_row("SELECT COUNT(*) FROM drops", [], |r| r.get(0))?;
        if count >= MAX_DROPS {
            return Err(Error::Invalid(format!("The drop log holds up to {MAX_DROPS} drops. Delete some first.")));
        }
        let blank = |s: &Option<String>| s.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string);
        self.conn.execute(
            "INSERT INTO drops (item, item_key, item_id, monster, monster_id, map, map_id, character, seen_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                item,
                item_key(item),
                drop.item_id,
                monster,
                drop.monster_id,
                blank(&drop.map),
                drop.map_id,
                blank(&drop.character),
                now
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Every logged drop, the newest first.
    pub fn drops(&self) -> Result<Vec<LoggedDrop>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, item, item_id, monster, monster_id, map, map_id, character, seen_at
             FROM drops ORDER BY seen_at DESC, id DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(LoggedDrop {
                id: row.get(0)?,
                item: row.get(1)?,
                item_id: row.get(2)?,
                monster: row.get(3)?,
                monster_id: row.get(4)?,
                map: row.get(5)?,
                map_id: row.get(6)?,
                character: row.get(7)?,
                seen_at: row.get(8)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn delete_drop(&mut self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM drops WHERE id = ?1", [id])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drop(item: &str, monster: &str) -> NewDrop {
        NewDrop {
            item: item.into(),
            item_id: Some(1),
            monster: monster.into(),
            monster_id: Some(2),
            map: Some(" Fern Hollow ".into()),
            map_id: Some(3),
            character: Some("".into()),
        }
    }

    #[test]
    fn logs_lists_newest_first_and_deletes() {
        let mut db = UserData::open_in_memory().unwrap();
        assert!(db.drops().unwrap().is_empty());

        let first = db.log_drop(&drop("Fern Dagger", "Moss Snail"), 10).unwrap();
        db.log_drop(&drop("Blue Tonic", "Bog Toad"), 20).unwrap();
        let drops = db.drops().unwrap();
        assert_eq!(drops.iter().map(|d| d.item.as_str()).collect::<Vec<_>>(), ["Blue Tonic", "Fern Dagger"]);
        assert_eq!(drops[1].map.as_deref(), Some("Fern Hollow"), "trimmed");
        assert_eq!(drops[1].character, None, "a blank name is kept as none");

        db.delete_drop(first).unwrap();
        assert_eq!(db.drops().unwrap().len(), 1);
    }

    #[test]
    fn needs_an_item_and_a_monster() {
        let mut db = UserData::open_in_memory().unwrap();
        assert!(db.log_drop(&drop("  ", "Moss Snail"), 1).is_err());
        assert!(db.log_drop(&drop("Fern Dagger", " "), 1).is_err());
    }
}
