//! What Leafline remembers about each character between runs: the level, job, EXP, map and mesos
//! last read for them, and the last check of each bag tab, so the Character tab can show who they were
//! before the game is read again. Each value keeps when it was read. These are for showing only:
//! nothing that works out a rate, a forecast or a session starts from them.

use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

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

/// What the HUD showed of a character, to remember. `mesos` only when just read.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SeenCharacter {
    pub name: String,
    pub level: Option<u32>,
    pub job: Option<String>,
    pub exp_percent: Option<f32>,
    pub exp_points: Option<u64>,
    /// `Street : Name`, as the session tracker keys maps.
    pub map: Option<String>,
    pub mesos: Option<u64>,
}

/// What Leafline last read of a character, each part with when.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnownCharacter {
    pub name: String,
    pub level: Option<u32>,
    pub job: Option<String>,
    pub exp_percent: Option<f32>,
    pub exp_points: Option<u64>,
    pub map: Option<String>,
    /// When the level, job, EXP and map were read, Unix seconds.
    pub seen_at: Option<i64>,
    pub mesos: Option<u64>,
    pub mesos_seen_at: Option<i64>,
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

    /// Remembers what the HUD showed of a character at `seen_at`. A part not read this time keeps
    /// what was read before.
    pub fn save_seen(&mut self, seen: &SeenCharacter, seen_at: i64) -> Result<()> {
        self.conn.execute(
            "INSERT INTO character_profiles (name, level, job, exp_percent, exp_points, map, seen_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT (name) DO UPDATE SET name = excluded.name,
                 level = COALESCE(excluded.level, level), job = COALESCE(excluded.job, job),
                 exp_points = CASE WHEN excluded.exp_percent IS NULL THEN exp_points ELSE excluded.exp_points END,
                 exp_percent = COALESCE(excluded.exp_percent, exp_percent),
                 map = COALESCE(excluded.map, map), seen_at = excluded.seen_at",
            params![
                seen.name,
                seen.level,
                seen.job,
                seen.exp_percent.map(f64::from),
                seen.exp_points.map(|p| p as i64),
                seen.map,
                seen_at
            ],
        )?;
        if let Some(mesos) = seen.mesos {
            self.save_mesos(&seen.name, mesos, seen_at)?;
        }
        Ok(())
    }

    /// What was last read of `name`, or with no name, of whichever character was seen last.
    pub fn known_character(&self, name: Option<&str>) -> Result<Option<KnownCharacter>> {
        let found = self
            .conn
            .query_row(
                "SELECT n.name, p.level, p.job, p.exp_percent, p.exp_points, p.map, p.seen_at, c.mesos, c.mesos_seen_at
                 FROM (SELECT name FROM character_profiles UNION SELECT name FROM characters) n
                 LEFT JOIN character_profiles p ON p.name = n.name COLLATE NOCASE
                 LEFT JOIN characters c ON c.name = n.name COLLATE NOCASE
                 WHERE ?1 IS NULL OR n.name = ?1 COLLATE NOCASE
                 ORDER BY MAX(COALESCE(p.seen_at, 0), COALESCE(c.mesos_seen_at, 0)) DESC LIMIT 1",
                [name],
                |row| {
                    Ok(KnownCharacter {
                        name: row.get(0)?,
                        level: row.get(1)?,
                        job: row.get(2)?,
                        exp_percent: row.get::<_, Option<f64>>(3)?.map(|p| p as f32),
                        exp_points: row.get::<_, Option<i64>>(4)?.map(|p| p.max(0) as u64),
                        map: row.get(5)?,
                        seen_at: row.get(6)?,
                        mesos: row.get::<_, Option<i64>>(7)?.map(|m| m.max(0) as u64),
                        mesos_seen_at: row.get(8)?,
                    })
                },
            )
            .optional()?;
        Ok(found)
    }

    /// Keeps a character's last check of one bag tab (as JSON) and when it was read. Each tab keeps
    /// its own, so a look at Use leaves the last look at Etc as it was.
    pub fn save_bag(&mut self, name: &str, tab: &str, check: &str, read_at: i64) -> Result<()> {
        self.conn.execute(
            "INSERT INTO character_bags (name, tab, bag, read_at) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT (name, tab) DO UPDATE SET bag = excluded.bag, read_at = excluded.read_at",
            params![name, tab, check, read_at],
        )?;
        Ok(())
    }

    /// The last check of each bag tab kept for `name`, the newest first; with no name, those of the
    /// character whose bag was read last.
    pub fn known_bags(&self, name: Option<&str>) -> Result<Vec<String>> {
        let whose: Option<String> = match name {
            Some(name) => Some(name.to_owned()),
            None => self
                .conn
                .query_row("SELECT name FROM character_bags ORDER BY read_at DESC LIMIT 1", [], |row| row.get(0))
                .optional()?,
        };
        let Some(whose) = whose else { return Ok(Vec::new()) };
        let mut stmt = self.conn.prepare("SELECT bag FROM character_bags WHERE name = ?1 ORDER BY read_at DESC")?;
        let bags = stmt.query_map([whose], |row| row.get(0))?.collect::<std::result::Result<_, _>>()?;
        Ok(bags)
    }

    /// Every character Leafline has a record for, the one seen last first.
    pub fn character_names(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT n.name FROM (SELECT name FROM character_profiles UNION SELECT name FROM characters
                                 UNION SELECT name FROM character_bags) n
             LEFT JOIN character_profiles p ON p.name = n.name COLLATE NOCASE
             LEFT JOIN characters c ON c.name = n.name COLLATE NOCASE
             ORDER BY MAX(COALESCE(p.seen_at, 0), COALESCE(c.mesos_seen_at, 0),
                          COALESCE((SELECT MAX(b.read_at) FROM character_bags b WHERE b.name = n.name), 0)) DESC",

        )?;
        let names = stmt.query_map([], |row| row.get(0))?.collect::<std::result::Result<_, _>>()?;
        Ok(names)
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

/// A character's look, cut from the game's Character Info window when it was their own: a small
/// PNG on a transparent ground, one pixel per game pixel. Kept on this PC only.
#[derive(Debug, Clone, PartialEq)]
pub struct KnownLook {
    pub name: String,
    pub png: Vec<u8>,
    pub width: u32,
    pub height: u32,
    /// Unix seconds.
    pub read_at: i64,
}

impl UserData {
    /// Keeps `png` as `name`'s look, replacing the one kept before. Names match whatever their case.
    pub fn save_look(&mut self, name: &str, png: &[u8], width: u32, height: u32, read_at: i64) -> Result<()> {
        self.conn.execute(
            "INSERT INTO character_looks (name, png, width, height, read_at) VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT (name) DO UPDATE SET name = excluded.name, png = excluded.png, width = excluded.width,
                 height = excluded.height, read_at = excluded.read_at",
            params![name, png, width, height, read_at],
        )?;
        Ok(())
    }

    /// Every character's kept look, the newest first.
    pub fn looks(&self) -> Result<Vec<KnownLook>> {
        let mut stmt = self.conn.prepare("SELECT name, png, width, height, read_at FROM character_looks ORDER BY read_at DESC")?;
        let looks = stmt
            .query_map([], |row| {
                Ok(KnownLook { name: row.get(0)?, png: row.get(1)?, width: row.get(2)?, height: row.get(3)?, read_at: row.get(4)? })
            })?
            .collect::<std::result::Result<_, _>>()?;
        Ok(looks)
    }

    /// Forgets every kept look.
    pub fn forget_looks(&mut self) -> Result<()> {
        self.conn.execute("DELETE FROM character_looks", [])?;
        Ok(())
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

    fn seen(name: &str, level: u32, map: Option<&str>) -> SeenCharacter {
        SeenCharacter {
            name: name.into(),
            level: Some(level),
            job: Some("Archer".into()),
            exp_percent: Some(41.5),
            exp_points: Some(830),
            map: map.map(str::to_string),
            mesos: None,
        }
    }

    #[test]
    fn remembers_what_the_hud_last_showed_and_keeps_parts_not_read_again() {
        let mut db = UserData::open_in_memory().unwrap();
        assert_eq!(db.known_character(None).unwrap(), None);

        db.save_mesos("Mossbank", 500, 50).unwrap();
        db.save_seen(&seen("Mossbank", 12, Some("Victoria Road : Henesys")), 100).unwrap();
        // Read again with no map and no EXP: both stay as they were.
        let again = SeenCharacter { exp_percent: None, exp_points: None, mesos: Some(900), ..seen("mossbank", 13, None) };
        db.save_seen(&again, 200).unwrap();
        db.save_seen(&seen("Fernwick", 30, None), 150).unwrap();

        let moss = db.known_character(Some("MOSSBANK")).unwrap().unwrap();
        assert_eq!(moss.level, Some(13));
        assert_eq!(moss.map.as_deref(), Some("Victoria Road : Henesys"));
        assert_eq!((moss.exp_percent, moss.exp_points), (Some(41.5), Some(830)));
        assert_eq!((moss.seen_at, moss.mesos, moss.mesos_seen_at), (Some(200), Some(900), Some(200)));
        // With no name, whoever was seen last; a character known only by mesos counts.
        assert_eq!(db.known_character(None).unwrap().unwrap().name.to_lowercase(), "mossbank");
        db.save_mesos("Thornby", 10, 300).unwrap();
        let thornby = db.known_character(None).unwrap().unwrap();
        assert_eq!((thornby.name.as_str(), thornby.level, thornby.mesos), ("Thornby", None, Some(10)));
        assert_eq!(db.known_character(Some("Nobody")).unwrap(), None);
        assert_eq!(db.character_names().unwrap().first().map(String::as_str), Some("Thornby"));
    }

    #[test]
    fn keeps_each_characters_last_check_of_each_bag_tab() {
        let mut db = UserData::open_in_memory().unwrap();
        assert!(db.known_bags(None).unwrap().is_empty());
        db.save_seen(&seen("Mossbank", 12, None), 100).unwrap();
        db.save_bag("mossbank", "use", r#"{"at":1}"#, 110).unwrap();
        db.save_bag("Mossbank", "etc", r#"{"at":2}"#, 115).unwrap();
        db.save_bag("Fernwick", "use", r#"{"at":3}"#, 120).unwrap();
        // A newer look at a tab replaces only that tab's.
        db.save_bag("MOSSBANK", "use", r#"{"at":4}"#, 130).unwrap();
        assert_eq!(db.known_bags(Some("Mossbank")).unwrap(), vec![r#"{"at":4}"#, r#"{"at":2}"#]);
        assert_eq!(db.known_bags(None).unwrap(), vec![r#"{"at":4}"#, r#"{"at":2}"#]);
        assert_eq!(db.known_bags(Some("Nobody")).unwrap(), Vec::<String>::new());
        // Keeping the bag leaves what the HUD showed alone, and counts as seeing the character.
        assert_eq!(db.known_character(Some("Mossbank")).unwrap().unwrap().level, Some(12));
        assert_eq!(db.character_names().unwrap(), vec!["Mossbank", "Fernwick"]);
    }

    #[test]
    fn keeps_one_look_per_character_and_forgets_them_all() {
        let mut db = UserData::open_in_memory().unwrap();
        assert!(db.looks().unwrap().is_empty());
        db.save_look("Mossbank", &[1, 2, 3], 20, 60, 100).unwrap();
        db.save_look("Fernwick", &[4], 18, 58, 150).unwrap();
        db.save_look("mossbank", &[5, 6], 22, 62, 200).unwrap();
        let looks = db.looks().unwrap();
        assert_eq!(looks.len(), 2);
        assert_eq!((looks[0].name.as_str(), looks[0].png.as_slice(), looks[0].width, looks[0].read_at), ("mossbank", &[5u8, 6][..], 22, 200));
        assert_eq!(looks[1].name, "Fernwick");
        db.forget_looks().unwrap();
        assert!(db.looks().unwrap().is_empty());
    }
}
