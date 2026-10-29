//! Saved sessions: what the session tracker measured, kept so training spots can be compared
//! across play sessions.

use rusqlite::params;
use serde::Serialize;

use super::UserData;
use crate::error::{Error, Result};
use crate::session::SessionSummary;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedSession {
    pub id: i64,
    pub label: String,
    /// Unix seconds.
    pub saved_at: i64,
    /// When the first frame was taken, in Unix seconds, if known.
    pub started_at: Option<f64>,
    pub level: Option<u32>,
    pub level_ups: u32,
    pub active_seconds: f64,
    pub elapsed_seconds: f64,
    pub exp_gained: i64,
    pub exp_per_hour: Option<f64>,
    pub mesos_gained: i64,
    pub mesos_per_hour: Option<f64>,
    /// Per map, most active time first. Time that couldn't be placed on a map is left out.
    pub maps: Vec<SavedMap>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedMap {
    /// `Street : Name`, as the session tracker keys maps.
    pub map: String,
    pub active_seconds: f64,
    pub exp_gained: i64,
    pub mesos_gained: i64,
}

/// One map's rates over every saved session that spent time there.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapRate {
    pub map: String,
    pub sessions: u32,
    pub active_seconds: f64,
    pub exp_per_hour: f64,
    pub mesos_per_hour: f64,
}

impl UserData {
    pub fn save_session(&mut self, session: &SessionSummary, label: &str, now: i64) -> Result<i64> {
        if session.active_seconds <= 0.0 {
            return Err(Error::Invalid("This session has no active time to save yet.".into()));
        }
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO sessions (label, saved_at, started_at, level, level_ups, active_seconds, elapsed_seconds,
                                   exp_gained, exp_per_hour, mesos_gained, mesos_per_hour)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                label.trim(),
                now,
                session.started_at,
                session.level,
                session.level_ups,
                session.active_seconds,
                session.elapsed_seconds,
                session.exp_gained,
                session.exp_per_hour,
                session.mesos_gained,
                session.mesos_per_hour
            ],
        )?;
        let id = tx.last_insert_rowid();
        for map in session.maps.iter().filter(|m| m.active_seconds > 0.0) {
            let Some(name) = &map.map else { continue };
            tx.execute(
                "INSERT INTO session_maps (session_id, map, active_seconds, exp_gained, mesos_gained)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![id, name, map.active_seconds, map.exp_gained, map.mesos_gained],
            )?;
        }
        tx.commit()?;
        Ok(id)
    }

    pub fn delete_session(&mut self, id: i64) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM session_maps WHERE session_id = ?1", [id])?;
        tx.execute("DELETE FROM sessions WHERE id = ?1", [id])?;
        tx.commit()?;
        Ok(())
    }

    /// Every saved session, newest first.
    pub fn sessions(&self) -> Result<Vec<SavedSession>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, label, saved_at, started_at, level, level_ups, active_seconds, elapsed_seconds, exp_gained,
                    exp_per_hour, mesos_gained, mesos_per_hour
             FROM sessions ORDER BY COALESCE(started_at, saved_at) DESC, id DESC",
        )?;
        let mut sessions: Vec<SavedSession> = stmt
            .query_map([], |row| {
                Ok(SavedSession {
                    id: row.get(0)?,
                    label: row.get(1)?,
                    saved_at: row.get(2)?,
                    started_at: row.get(3)?,
                    level: row.get(4)?,
                    level_ups: row.get(5)?,
                    active_seconds: row.get(6)?,
                    elapsed_seconds: row.get(7)?,
                    exp_gained: row.get(8)?,
                    exp_per_hour: row.get(9)?,
                    mesos_gained: row.get(10)?,
                    mesos_per_hour: row.get(11)?,
                    maps: Vec::new(),
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        let mut maps = self.conn.prepare(
            "SELECT map, active_seconds, exp_gained, mesos_gained FROM session_maps
             WHERE session_id = ?1 ORDER BY active_seconds DESC",
        )?;
        for session in &mut sessions {
            session.maps = maps
                .query_map([session.id], |row| {
                    Ok(SavedMap {
                        map: row.get(0)?,
                        active_seconds: row.get(1)?,
                        exp_gained: row.get(2)?,
                        mesos_gained: row.get(3)?,
                    })
                })?
                .collect::<rusqlite::Result<_>>()?;
        }
        Ok(sessions)
    }

    /// Each map's EXP and mesos per hour over all saved sessions, weighted by time spent there.
    /// Most time first.
    pub fn map_rates(&self) -> Result<Vec<MapRate>> {
        let mut stmt = self.conn.prepare(
            "SELECT map, COUNT(DISTINCT session_id), SUM(active_seconds), SUM(exp_gained), SUM(mesos_gained)
             FROM session_maps GROUP BY map HAVING SUM(active_seconds) > 0 ORDER BY SUM(active_seconds) DESC",
        )?;
        let rates = stmt.query_map([], |row| {
            let seconds: f64 = row.get(2)?;
            Ok(MapRate {
                map: row.get(0)?,
                sessions: row.get(1)?,
                active_seconds: seconds,
                exp_per_hour: row.get::<_, i64>(3)? as f64 * 3600.0 / seconds,
                mesos_per_hour: row.get::<_, i64>(4)? as f64 * 3600.0 / seconds,
            })
        })?;
        Ok(rates.collect::<rusqlite::Result<_>>()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::MapSummary;

    // A made-up session on made-up maps.
    fn session(minutes: f64, exp: i64, maps: &[(Option<&str>, f64, i64)]) -> SessionSummary {
        SessionSummary {
            frames: 10,
            started_at: Some(1000.0),
            active_seconds: minutes * 60.0,
            elapsed_seconds: minutes * 60.0 + 120.0,
            level: Some(30),
            exp_gained: exp,
            exp_per_hour: Some(exp as f64 / minutes * 60.0),
            maps: maps
                .iter()
                .map(|&(map, minutes, exp)| MapSummary {
                    map: map.map(String::from),
                    active_seconds: minutes * 60.0,
                    exp_gained: exp,
                    ..MapSummary::default()
                })
                .collect(),
            ..SessionSummary::default()
        }
    }

    #[test]
    fn saves_sessions_and_weighs_map_rates_by_time() {
        let mut db = UserData::open_in_memory().unwrap();
        let meadow = "Fern Road : Mossbank Meadow I";
        let den = "Fern Road : Burrow Den";
        db.save_session(&session(30.0, 6000, &[(Some(meadow), 20.0, 4000), (None, 10.0, 2000)]), "First", 1).unwrap();
        db.save_session(&session(60.0, 15000, &[(Some(meadow), 40.0, 12000), (Some(den), 20.0, 3000)]), "", 2).unwrap();

        let sessions = db.sessions().unwrap();
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[1].label, "First");
        assert_eq!(sessions[1].maps.len(), 1, "unplaced time isn't kept per map");

        let rates = db.map_rates().unwrap();
        assert_eq!(rates[0].map, meadow);
        assert_eq!(rates[0].sessions, 2);
        assert_eq!(rates[0].active_seconds, 3600.0);
        assert!((rates[0].exp_per_hour - 16000.0).abs() < 1e-9);
        assert!((rates[1].exp_per_hour - 9000.0).abs() < 1e-9);

        db.delete_session(sessions[0].id).unwrap();
        assert_eq!(db.map_rates().unwrap()[0].active_seconds, 1200.0);
    }

    #[test]
    fn an_empty_session_is_not_saved() {
        let mut db = UserData::open_in_memory().unwrap();
        assert!(db.save_session(&session(0.0, 0, &[]), "", 1).is_err());
    }
}
