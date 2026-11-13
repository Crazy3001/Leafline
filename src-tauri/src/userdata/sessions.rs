//! Saved sessions: what the session tracker measured, kept so training spots can be compared
//! across play sessions.

use rusqlite::params;
use serde::Serialize;

use super::UserData;
use super::potions::{SessionPotion, cost_of, session_key};
use crate::error::{Error, Result};
use crate::session::{EventKind, SessionEvent, SessionLine, SessionSummary, TimelinePoint};

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
    pub percent_per_hour: Option<f64>,
    pub job: Option<String>,
    /// The character's name, when the tracker read it.
    pub character: Option<String>,
    pub deaths: u32,
    /// The player's own note. It stays on this PC.
    pub note: String,
    /// Per map, most active time first. Time that couldn't be placed on a map is left out.
    pub maps: Vec<SavedMap>,
    /// Per map, level, party size and job, most active time first.
    pub lines: Vec<SessionLine>,
    /// The potion counts the player typed for the session, in slot order.
    pub potions: Vec<SessionPotion>,
    /// What the potions used cost, in mesos.
    pub potion_cost: i64,
}

/// The EXP a level takes, as the player's own HUD showed it. Leafline ships no EXP table; this
/// one is learned as the player levels, for the time left to a level goal.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LevelExp {
    pub level: u32,
    pub needed: u64,
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

/// How a saved session's rates moved over time, for its graph.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionTimeline {
    pub points: Vec<TimelinePoint>,
    pub events: Vec<SessionEvent>,
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
        tx.execute("INSERT INTO sessions (label, saved_at, level_ups, active_seconds, elapsed_seconds, exp_gained, mesos_gained)
                    VALUES (?1, ?2, 0, 0, 0, 0, 0)", params![label.trim(), now])?;
        let id = tx.last_insert_rowid();
        write_session(&tx, id, session, now)?;
        tx.commit()?;
        Ok(id)
    }

    /// Brings a saved session up to date as it goes on. Its label and note stay as they are.
    pub fn update_session(&mut self, id: i64, session: &SessionSummary, now: i64) -> Result<()> {
        let tx = self.conn.transaction()?;
        write_session(&tx, id, session, now)?;
        tx.commit()?;
        Ok(())
    }

    pub fn set_session_note(&mut self, id: i64, note: &str) -> Result<()> {
        self.conn.execute("UPDATE sessions SET note = ?2 WHERE id = ?1", params![id, note.trim()])?;
        Ok(())
    }

    /// Corrects a map name everywhere it was saved, and from now on when it is read that way.
    pub fn rename_map(&mut self, from: &str, to: &str) -> Result<()> {
        let to = to.trim();
        if to.is_empty() {
            return Err(Error::Invalid("A map needs a name.".into()));
        }
        let tx = self.conn.transaction()?;
        tx.execute("UPDATE session_maps SET map = ?2 WHERE map = ?1", params![from, to])?;
        tx.execute("UPDATE session_lines SET map = ?2 WHERE map = ?1", params![from, to])?;
        tx.execute("UPDATE session_points SET map = ?2 WHERE map = ?1", params![from, to])?;
        tx.execute("UPDATE map_renames SET name = ?2 WHERE name = ?1", params![from, to])?;
        tx.execute("INSERT OR REPLACE INTO map_renames (read_as, name) VALUES (?1, ?2)", params![from, to])?;
        tx.commit()?;
        Ok(())
    }

    pub fn delete_session(&mut self, id: i64) -> Result<()> {
        let started_at: Option<f64> = self
            .conn
            .query_row("SELECT started_at FROM sessions WHERE id = ?1", [id], |row| row.get(0))
            .or_else(|err| if err == rusqlite::Error::QueryReturnedNoRows { Ok(None) } else { Err(err) })?;
        let tx = self.conn.transaction()?;
        UserData::delete_session_potions(&tx, started_at)?;
        tx.execute("DELETE FROM session_points WHERE session_id = ?1", [id])?;
        tx.execute("DELETE FROM session_events WHERE session_id = ?1", [id])?;
        tx.execute("DELETE FROM session_maps WHERE session_id = ?1", [id])?;
        tx.execute("DELETE FROM session_lines WHERE session_id = ?1", [id])?;
        tx.execute("DELETE FROM sessions WHERE id = ?1", [id])?;
        tx.commit()?;
        Ok(())
    }

    /// The EXP each level takes, as far as the player's HUD has shown it, lowest level first.
    pub fn level_exp(&self) -> Result<Vec<LevelExp>> {
        let mut stmt = self.conn.prepare("SELECT level, needed FROM level_exp ORDER BY level")?;
        let rows = stmt.query_map([], |row| Ok(LevelExp { level: row.get(0)?, needed: row.get::<_, i64>(1)? as u64 }))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Every saved session, newest first.
    pub fn sessions(&self) -> Result<Vec<SavedSession>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, label, saved_at, started_at, level, level_ups, active_seconds, elapsed_seconds, exp_gained,
                    exp_per_hour, mesos_gained, mesos_per_hour, percent_per_hour, job, deaths, note, character
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
                    percent_per_hour: row.get(12)?,
                    job: row.get(13)?,
                    deaths: row.get(14)?,
                    note: row.get(15)?,
                    character: row.get(16)?,
                    maps: Vec::new(),
                    lines: Vec::new(),
                    potions: Vec::new(),
                    potion_cost: 0,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        let mut maps = self.conn.prepare(
            "SELECT map, active_seconds, exp_gained, mesos_gained FROM session_maps
             WHERE session_id = ?1 ORDER BY active_seconds DESC",
        )?;
        let mut lines = self.conn.prepare(
            "SELECT map, level, party, job, active_seconds, exp_gained, percent_gained, mesos_gained, mesos_seconds
             FROM session_lines WHERE session_id = ?1 ORDER BY active_seconds DESC",
        )?;
        let mut potions = self.all_session_potions()?;
        for session in &mut sessions {
            if let Some(typed) = session.started_at.and_then(|at| potions.remove(&session_key(at))) {
                session.potion_cost = cost_of(&typed);
                session.potions = typed;
            }
            session.lines = lines
                .query_map([session.id], |row| {
                    Ok(SessionLine {
                        map: row.get(0)?,
                        town: false,
                        level: row.get(1)?,
                        party: row.get(2)?,
                        job: row.get(3)?,
                        active_seconds: row.get(4)?,
                        exp_gained: row.get(5)?,
                        percent_gained: row.get(6)?,
                        mesos_gained: row.get(7)?,
                        mesos_seconds: row.get(8)?,
                    })
                })?
                .collect::<rusqlite::Result<_>>()?;
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

    /// A saved session's timeline. Sessions saved before timelines were kept have none.
    pub fn session_timeline(&self, id: i64) -> Result<SessionTimeline> {
        let mut points = self.conn.prepare(
            "SELECT at, exp_per_hour, percent_per_hour, mesos_per_hour, level, map FROM session_points
             WHERE session_id = ?1 ORDER BY at",
        )?;
        let points = points
            .query_map([id], |row| {
                Ok(TimelinePoint {
                    at: row.get(0)?,
                    exp_per_hour: row.get(1)?,
                    percent_per_hour: row.get(2)?,
                    mesos_per_hour: row.get(3)?,
                    level: row.get(4)?,
                    map: row.get(5)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        let mut events =
            self.conn.prepare("SELECT at, kind, level FROM session_events WHERE session_id = ?1 ORDER BY at")?;
        let events = events
            .query_map([id], |row| {
                let kind: String = row.get(1)?;
                Ok(SessionEvent {
                    at: row.get(0)?,
                    kind: if kind == "death" { EventKind::Death } else { EventKind::LevelUp },
                    level: row.get(2)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        Ok(SessionTimeline { points, events })
    }

    /// Each map's EXP and mesos per hour over all saved sessions, weighted by time spent there.
    /// Most time first.
    pub fn map_rates(&self) -> Result<Vec<MapRate>> {
        let mut stmt = self.conn.prepare(
            "SELECT map, COUNT(DISTINCT session_id), SUM(active_seconds), SUM(exp_gained), SUM(mesos_gained)
             FROM session_maps GROUP BY map HAVING SUM(active_seconds) > 0 AND SUM(exp_gained) > 0
             ORDER BY SUM(active_seconds) DESC",
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

/// Writes what `session` holds to the row `id`, replacing what was there. Towns aren't logged.
fn write_session(tx: &rusqlite::Transaction, id: i64, session: &SessionSummary, now: i64) -> Result<()> {
    tx.execute(
        "UPDATE sessions SET saved_at = ?2, started_at = ?3, level = ?4, level_ups = ?5, active_seconds = ?6,
                             elapsed_seconds = ?7, exp_gained = ?8, exp_per_hour = ?9, mesos_gained = ?10,
                             mesos_per_hour = ?11, percent_per_hour = ?12, job = ?13, deaths = ?14,
                             character = COALESCE(?15, character)
         WHERE id = ?1",
        params![
            id,
            now,
            session.started_at,
            session.level,
            session.level_ups,
            session.active_seconds,
            session.elapsed_seconds,
            session.exp_gained,
            session.exp_per_hour,
            session.mesos_gained,
            session.mesos_per_hour,
            session.percent_per_hour,
            session.job,
            session.deaths,
            session.name
        ],
    )?;
    tx.execute("DELETE FROM session_maps WHERE session_id = ?1", [id])?;
    tx.execute("DELETE FROM session_lines WHERE session_id = ?1", [id])?;
    tx.execute("DELETE FROM session_points WHERE session_id = ?1", [id])?;
    tx.execute("DELETE FROM session_events WHERE session_id = ?1", [id])?;
    // What the current level takes, once the tracker has narrowed it down from the HUD.
    if let (Some(level), Some(needed)) = (session.level, session.exp_needed) {
        tx.execute(
            "INSERT INTO level_exp (level, needed, seen_at) VALUES (?1, ?2, ?3)
             ON CONFLICT (level) DO UPDATE SET needed = excluded.needed, seen_at = excluded.seen_at",
            params![level, needed as i64, now],
        )?;
    }
    let renamed = |map: &str| -> rusqlite::Result<String> {
        match tx.query_row("SELECT name FROM map_renames WHERE read_as = ?1", [map], |row| row.get(0)) {
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(map.to_string()),
            found => found,
        }
    };
    let town = |map: &str| session.lines.iter().any(|l| l.town && l.map.as_deref() == Some(map));
    for map in session.maps.iter().filter(|m| m.active_seconds > 0.0) {
        let Some(name) = map.map.as_deref().filter(|name| !town(name)) else { continue };
        tx.execute(
            "INSERT INTO session_maps (session_id, map, active_seconds, exp_gained, mesos_gained)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, renamed(name)?, map.active_seconds, map.exp_gained, map.mesos_gained],
        )?;
    }
    for point in &session.timeline {
        let map = point.map.as_deref().map(renamed).transpose()?;
        tx.execute(
            "INSERT INTO session_points (session_id, at, exp_per_hour, percent_per_hour, mesos_per_hour, level, map)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, point.at, point.exp_per_hour, point.percent_per_hour, point.mesos_per_hour, point.level, map],
        )?;
    }
    for event in &session.events {
        let kind = match event.kind {
            EventKind::LevelUp => "levelUp",
            EventKind::Death => "death",
        };
        tx.execute(
            "INSERT INTO session_events (session_id, at, kind, level) VALUES (?1, ?2, ?3, ?4)",
            params![id, event.at, kind, event.level],
        )?;
    }
    for line in session.lines.iter().filter(|l| !l.town && l.active_seconds > 0.0) {
        let map = line.map.as_deref().map(renamed).transpose()?;
        tx.execute(
            "INSERT INTO session_lines (session_id, map, level, party, job, active_seconds, exp_gained,
                                        percent_gained, mesos_gained, mesos_seconds)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                id,
                map,
                line.level,
                line.party,
                line.job,
                line.active_seconds,
                line.exp_gained,
                line.percent_gained,
                line.mesos_gained,
                line.mesos_seconds
            ],
        )?;
    }
    Ok(())
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
    fn a_session_saved_as_it_goes_keeps_its_note() {
        let mut db = UserData::open_in_memory().unwrap();
        let meadow = "Fern Road : Mossbank Meadow I";
        let id = db.save_session(&session(10.0, 1000, &[(Some(meadow), 10.0, 1000)]), "", 1).unwrap();
        db.set_session_note(id, " Good spot ").unwrap();
        db.update_session(id, &session(20.0, 2000, &[(Some(meadow), 20.0, 2000)]), 2).unwrap();
        let saved = &db.sessions().unwrap()[0];
        assert_eq!(saved.note, "Good spot");
        assert_eq!(saved.exp_gained, 2000);
        assert_eq!(saved.maps.len(), 1);
        assert_eq!(saved.maps[0].active_seconds, 1200.0);
    }

    #[test]
    fn the_exp_a_level_takes_is_learned_from_saves() {
        let mut db = UserData::open_in_memory().unwrap();
        let mut at_30 = session(10.0, 1000, &[]);
        at_30.exp_needed = Some(25_000);
        let id = db.save_session(&at_30, "", 1).unwrap();
        let mut at_31 = session(20.0, 2000, &[]);
        at_31.level = Some(31);
        at_31.exp_needed = Some(27_500);
        db.update_session(id, &at_31, 2).unwrap();
        let mut unknown = session(30.0, 3000, &[]);
        unknown.level = Some(32);
        db.update_session(id, &unknown, 3).unwrap();
        assert_eq!(db.level_exp().unwrap(), [LevelExp { level: 30, needed: 25_000 }, LevelExp { level: 31, needed: 27_500 }]);
    }

    #[test]
    fn potions_typed_before_the_save_reach_the_saved_session() {
        use crate::userdata::potions::PotionSlot;
        let mut db = UserData::open_in_memory().unwrap();
        db.set_potion_slots(None, &[PotionSlot { slot: 0, name: "White Potion".into(), price: 320 }]).unwrap();
        // Typed in the first minutes, while the session wasn't saved yet.
        db.set_session_potion(None, 1000.0, 1, Some(300), Some(250)).unwrap();
        let id = db.save_session(&session(30.0, 6000, &[]), "", 1).unwrap();
        let saved = &db.sessions().unwrap()[0];
        assert_eq!((saved.potions.len(), saved.potion_cost), (1, 50 * 320));
        db.delete_session(id).unwrap();
        assert!(db.session_potions(1000.0).unwrap().is_empty());
    }

    #[test]
    fn a_renamed_map_stays_renamed() {
        let mut db = UserData::open_in_memory().unwrap();
        let (wrong, right) = ("Fern Road : Mossbank Meadovv", "Fern Road : Mossbank Meadow");
        let id = db.save_session(&session(10.0, 1000, &[(Some(wrong), 10.0, 1000)]), "", 1).unwrap();
        db.rename_map(wrong, right).unwrap();
        assert_eq!(db.sessions().unwrap()[0].maps[0].map, right);
        db.update_session(id, &session(20.0, 2000, &[(Some(wrong), 20.0, 2000)]), 2).unwrap();
        assert_eq!(db.sessions().unwrap()[0].maps[0].map, right, "the same misread later on is corrected too");
        assert!(db.rename_map(right, "  ").is_err());
    }

    #[test]
    fn keeps_the_timeline_events_and_character() {
        let mut db = UserData::open_in_memory().unwrap();
        let meadow = "Fern Road : Mossbank Meadow I";
        let mut summary = session(10.0, 1000, &[(Some(meadow), 10.0, 1000)]);
        summary.name = Some("Leafling".into());
        summary.timeline = (1..=3)
            .map(|i| TimelinePoint {
                at: f64::from(i) * 30.0,
                exp_per_hour: Some(f64::from(i) * 1000.0),
                level: Some(30),
                map: Some(meadow.into()),
                ..TimelinePoint::default()
            })
            .collect();
        summary.events = vec![SessionEvent { at: 45.0, kind: EventKind::LevelUp, level: Some(31) }];
        let id = db.save_session(&summary, "", 1).unwrap();
        db.rename_map(meadow, "Fern Road : Mossbank Meadow").unwrap();

        let timeline = db.session_timeline(id).unwrap();
        assert_eq!(timeline.points.len(), 3);
        assert_eq!(timeline.points[2].exp_per_hour, Some(3000.0));
        assert_eq!(timeline.points[0].map.as_deref(), Some("Fern Road : Mossbank Meadow"));
        assert_eq!(timeline.events, summary.events);
        assert_eq!(db.sessions().unwrap()[0].character.as_deref(), Some("Leafling"));

        // A later save without a name read keeps the one saved.
        summary.name = None;
        db.update_session(id, &summary, 2).unwrap();
        assert_eq!(db.sessions().unwrap()[0].character.as_deref(), Some("Leafling"));
        assert_eq!(db.session_timeline(id).unwrap().points.len(), 3, "points are replaced, not added");

        db.delete_session(id).unwrap();
        assert!(db.session_timeline(id).unwrap().points.is_empty());
    }

    #[test]
    fn towns_are_not_logged() {
        let mut db = UserData::open_in_memory().unwrap();
        let town = "Fern Road : Mossbank";
        let mut summary = session(10.0, 1000, &[(Some(town), 10.0, 1000)]);
        summary.lines =
            vec![SessionLine { map: Some(town.into()), town: true, active_seconds: 600.0, ..SessionLine::default() }];
        db.save_session(&summary, "", 1).unwrap();
        let saved = &db.sessions().unwrap()[0];
        assert!(saved.maps.is_empty() && saved.lines.is_empty());
    }

    #[test]
    fn an_empty_session_is_not_saved() {
        let mut db = UserData::open_in_memory().unwrap();
        assert!(db.save_session(&session(0.0, 0, &[]), "", 1).is_err());
    }
}
