//! Builds the player planned. The goals are kept as one JSON value, which grows with the planner
//! without a new table for each kind of goal.

use rusqlite::{OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};

use super::UserData;
use crate::build::{Build, Plan, Start};
use crate::error::{Error, Result};

/// A build in a list, without its goals.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildSummary {
    pub id: i64,
    pub name: String,
    pub character: Option<String>,
    pub updated_at: i64,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct Body {
    guide: String,
    start: Start,
    plan: Plan,
}

const COLUMNS: &str = "id, name, character, body";

fn from_row(row: &Row) -> rusqlite::Result<(Build, String)> {
    let build = Build {
        id: row.get(0)?,
        name: row.get(1)?,
        character: row.get(2)?,
        ..Build::default()
    };
    Ok((build, row.get(3)?))
}

fn with_body((build, body): (Build, String)) -> Result<Build> {
    let Body { guide, start, plan } = serde_json::from_str(&body)
        .map_err(|err| Error::Invalid(format!("The saved build \"{}\" could not be read: {err}", build.name)))?;
    Ok(Build { guide, start, plan, ..build })
}

impl UserData {
    /// Every build, the most recently used first.
    pub fn builds(&self) -> Result<Vec<BuildSummary>> {
        let mut stmt =
            self.conn.prepare("SELECT id, name, character, updated_at FROM builds ORDER BY used_at DESC, id DESC")?;
        let builds = stmt.query_map([], |row| {
            Ok(BuildSummary { id: row.get(0)?, name: row.get(1)?, character: row.get(2)?, updated_at: row.get(3)? })
        })?;
        Ok(builds.collect::<rusqlite::Result<_>>()?)
    }

    pub fn build(&self, id: i64) -> Result<Option<Build>> {
        let sql = format!("SELECT {COLUMNS} FROM builds WHERE id = ?1");
        self.conn.query_row(&sql, [id], from_row).optional()?.map(with_body).transpose()
    }

    /// The build for `character`, or else the one used last.
    pub fn active_build(&self, character: Option<&str>) -> Result<Option<Build>> {
        if let Some(character) = character {
            let sql = format!(
                "SELECT {COLUMNS} FROM builds WHERE character = ?1 COLLATE NOCASE ORDER BY used_at DESC, id DESC LIMIT 1"
            );
            if let Some(found) = self.conn.query_row(&sql, [character], from_row).optional()? {
                return with_body(found).map(Some);
            }
        }
        let sql = format!("SELECT {COLUMNS} FROM builds ORDER BY used_at DESC, id DESC LIMIT 1");
        self.conn.query_row(&sql, [], from_row).optional()?.map(with_body).transpose()
    }

    /// Adds a build with no id yet, or replaces the one with this id. Saving counts as using it.
    pub fn save_build(&mut self, build: &Build, now: i64) -> Result<i64> {
        let body = Body { guide: build.guide.clone(), start: build.start.clone(), plan: build.plan.clone() };
        let body = serde_json::to_string(&body).map_err(|err| Error::Invalid(err.to_string()))?;
        if build.id == 0 {
            self.conn.execute(
                "INSERT INTO builds (name, character, body, updated_at, used_at) VALUES (?1, ?2, ?3, ?4, ?4)",
                params![build.name, build.character, body, now],
            )?;
            return Ok(self.conn.last_insert_rowid());
        }
        self.conn.execute(
            "UPDATE builds SET name = ?2, character = ?3, body = ?4, updated_at = ?5, used_at = ?5 WHERE id = ?1",
            params![build.id, build.name, build.character, body, now],
        )?;
        Ok(build.id)
    }

    /// Makes a build the one shown when no other is tied to the character.
    pub fn use_build(&mut self, id: i64, now: i64) -> Result<()> {
        self.conn.execute("UPDATE builds SET used_at = ?2 WHERE id = ?1", params![id, now])?;
        Ok(())
    }

    pub fn delete_build(&mut self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM builds WHERE id = ?1", [id])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::*;
    use crate::build::plan::{SkillLevel, Stat, StatGoal, Stats};

    fn a_build(name: &str, character: Option<&str>) -> Build {
        Build {
            name: name.into(),
            character: character.map(str::to_string),
            guide: "https://example.invalid/guide".into(),
            start: Start { level: 12, stats: Some(Stats { str: 4, dex: 30, int: 4, luk: 12 }), ..Start::default() },
            plan: Plan {
                job: 420,
                skills: vec![SkillLevel { skill_id: 9, level: 5 }],
                stats: vec![StatGoal { stat: Stat::Luk, target: None, from_level: None }],
                ..Plan::default()
            },
            ..Build::default()
        }
    }

    #[test]
    fn a_build_comes_back_as_it_was_saved() {
        let mut db = UserData::open_in_memory().unwrap();
        let id = db.save_build(&a_build("Fern runner", Some("Moss")), 100).unwrap();
        assert_eq!(db.build(id).unwrap().unwrap(), Build { id, ..a_build("Fern runner", Some("Moss")) });
        assert_eq!(db.build(id + 1).unwrap(), None);
    }

    #[test]
    fn saving_with_an_id_replaces_and_deleting_removes() {
        let mut db = UserData::open_in_memory().unwrap();
        let id = db.save_build(&a_build("One", None), 100).unwrap();
        let again = db.save_build(&Build { id, ..a_build("Two", None) }, 200).unwrap();
        assert_eq!(again, id);
        let list = db.builds().unwrap();
        assert_eq!((list.len(), list[0].name.as_str(), list[0].updated_at), (1, "Two", 200));
        db.delete_build(id).unwrap();
        assert!(db.builds().unwrap().is_empty());
    }

    #[test]
    fn the_active_build_follows_the_character_then_the_last_used() {
        let mut db = UserData::open_in_memory().unwrap();
        assert_eq!(db.active_build(None).unwrap(), None);
        let moss = db.save_build(&a_build("Moss", Some("MossRunner")), 100).unwrap();
        let free = db.save_build(&a_build("Free", None), 200).unwrap();
        assert_eq!(db.active_build(Some("mossrunner")).unwrap().unwrap().id, moss, "names ignore case");
        assert_eq!(db.active_build(Some("Stranger")).unwrap().unwrap().id, free);
        assert_eq!(db.active_build(None).unwrap().unwrap().id, free);
        db.use_build(moss, 300).unwrap();
        assert_eq!(db.active_build(None).unwrap().unwrap().id, moss);
    }

    #[test]
    fn an_older_file_gains_builds_and_keeps_its_records() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(super::super::MIGRATIONS[0]).unwrap();
        conn.execute(
            "INSERT INTO prices (item, item_key, price, kind, recorded_at) VALUES ('Fern Hat', 'fern hat', 5, 'traded', 1)",
            [],
        )
        .unwrap();
        conn.pragma_update(None, "user_version", 1).unwrap();
        let mut db = UserData::init(conn).unwrap();
        assert_eq!(db.all_prices().unwrap().len(), 1);
        assert!(db.save_build(&a_build("New", None), 5).is_ok());
    }

    #[test]
    fn a_saved_body_missing_new_fields_still_reads() {
        let mut db = UserData::open_in_memory().unwrap();
        let id = db.save_build(&a_build("Old", None), 1).unwrap();
        db.conn.execute("UPDATE builds SET body = ?1 WHERE id = ?2", params![r#"{"plan": {"job": 400}}"#, id]).unwrap();
        let build = db.build(id).unwrap().unwrap();
        assert_eq!((build.plan.job, build.plan.plan_to, build.start.level), (400, 70, 1));
    }

    #[test]
    fn a_damaged_build_names_itself() {
        let mut db = UserData::open_in_memory().unwrap();
        let id = db.save_build(&a_build("Broken", None), 1).unwrap();
        db.conn.execute("UPDATE builds SET body = 'not json' WHERE id = ?1", [id]).unwrap();
        assert!(db.build(id).unwrap_err().to_string().contains("Broken"));
    }
}
