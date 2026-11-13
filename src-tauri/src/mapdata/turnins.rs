//! Who takes each quest in. OSMS's quests only name who gives one, so this comes from
//! `client/quests.json` in Leafline's data repository, which holds the turn-in NPC the game
//! client itself records for each quest, by the same quest and NPC ids OSMS uses.
//!
//! The whole file is downloaded and every lookup is answered from that copy, so nothing about
//! the player's quests is sent anywhere. Until 2026-11-11 this came from mapleclassic.wiki's
//! `quests` table, which no longer answers anonymous requests.

use std::collections::HashMap;

use serde::Deserialize;

use super::drops::{reason, recent_failure};
use super::fetch::Source;
use super::store::{Store, TURN_INS_FAILED};
use crate::error::{Error, Result};

/// Where the quest file lives. Nothing about the player goes with the request.
pub const TURN_INS_URL: &str = "https://raw.githubusercontent.com/Crazy3001/Leafline-data/main/client/quests.json";
/// The only `schema` this reader understands.
const SCHEMA: u32 = 1;
/// The file only changes when the game is patched, so it is checked at most once a day.
pub const MAX_AGE: i64 = 24 * 60 * 60;

/// The quest's id and the id of the NPC who takes it in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TurnIn {
    pub quest: u32,
    pub npc: u32,
}

/// Downloads the file if there is no copy or it is over a day old, and says whether it did. A
/// failed download keeps the old copy and, like the drop table's, is not tried again for a day.
pub fn refresh(store: &mut Store, source: &dyn Source, now: i64) -> Result<bool> {
    let fetched_at = store.turn_ins_fetched_at()?;
    if fetched_at.is_some_and(|at| now - at < MAX_AGE) {
        return Ok(false);
    }
    if let Some(reason) = recent_failure(store, TURN_INS_FAILED, now)? {
        return if fetched_at.is_some() { Ok(false) } else { Err(Error::Download(reason)) };
    }
    match source.get(TURN_INS_URL).and_then(|body| parse(&body)) {
        Ok(rows) => {
            store.replace_turn_ins(&rows, now)?;
            Ok(true)
        }
        Err(err) => {
            store.set_failure(TURN_INS_FAILED, Some((now, &reason(&err))))?;
            if fetched_at.is_none() {
                return Err(err);
            }
            eprintln!("turn-in table update failed: {err}");
            Ok(false)
        }
    }
}

/// The quests that name a turn-in NPC, from the file as written.
fn parse(body: &[u8]) -> Result<Vec<TurnIn>> {
    #[derive(Deserialize)]
    struct File {
        schema: u32,
        quests: Vec<Quest>,
    }
    #[derive(Deserialize)]
    struct Quest {
        id: u32,
        turn_in: Option<u32>,
    }
    let file: File = serde_json::from_slice(body)
        .map_err(|err| Error::Download(format!("Leafline's quest file can't be read: {err}")))?;
    if file.schema != SCHEMA {
        return Err(Error::Download(format!(
            "Leafline's quest file is in format {}; this version reads format {SCHEMA}. Update Leafline.",
            file.schema
        )));
    }
    let rows: Vec<TurnIn> = file.quests.into_iter().filter_map(|q| Some(TurnIn { quest: q.id, npc: q.turn_in? })).collect();
    if rows.is_empty() {
        return Err(Error::Download("Leafline's quest file names no turn-ins".into()));
    }
    Ok(rows)
}

/// Each quest's turn-in NPC, for the NPCs the map data knows.
pub(super) fn resolve(turn_ins: &[TurnIn], known: impl Fn(u32) -> bool) -> HashMap<u32, u32> {
    turn_ins.iter().filter(|t| known(t.npc)).map(|t| (t.quest, t.npc)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mapdata::drops::RETRY_AFTER;
    use crate::mapdata::fetch::tests::FakeSource;

    // A made-up file in the data repository's shape.
    const FILE: &str = r#"{"schema": 1, "about": "test", "quests": [
        {"id": 70, "giver": 5, "turn_in": 50},
        {"id": 71, "giver": 5, "turn_in": 51, "jobs": [100]},
        {"id": 72, "giver": 5}
    ]}"#;

    fn source(body: &str) -> FakeSource {
        let mut source = FakeSource::default();
        source.files.insert(TURN_INS_URL.to_string(), body.as_bytes().to_vec());
        source
    }

    #[test]
    fn keeps_quests_that_name_a_turn_in() {
        let mut store = Store::open_in_memory().unwrap();
        assert!(refresh(&mut store, &source(FILE), 1000).unwrap());
        assert_eq!(store.turn_ins().unwrap(), [TurnIn { quest: 70, npc: 50 }, TurnIn { quest: 71, npc: 51 }]);
        assert_eq!(store.turn_ins_fetched_at().unwrap(), Some(1000));
    }

    #[test]
    fn asks_once_for_the_whole_file_and_names_nothing() {
        let mut store = Store::open_in_memory().unwrap();
        let file = source(FILE);
        refresh(&mut store, &file, 1000).unwrap();
        assert_eq!(*file.requests.borrow(), [TURN_INS_URL]);
        assert!(!TURN_INS_URL.contains('?'));
    }

    #[test]
    fn downloads_at_most_daily_and_replaces_the_old_table() {
        let mut store = Store::open_in_memory().unwrap();
        let first = source(FILE);
        refresh(&mut store, &first, 1000).unwrap();
        assert!(!refresh(&mut store, &first, 1000 + MAX_AGE - 1).unwrap());
        assert_eq!(first.requests.borrow().len(), 1, "served from the copy");

        let later = source(r#"{"schema": 1, "quests": [{"id": 73, "turn_in": 52}]}"#);
        assert!(refresh(&mut store, &later, 1000 + MAX_AGE).unwrap());
        assert_eq!(store.turn_ins().unwrap(), [TurnIn { quest: 73, npc: 52 }]);
    }

    #[test]
    fn keeps_the_old_table_when_offline_or_sent_nothing() {
        let mut store = Store::open_in_memory().unwrap();
        refresh(&mut store, &source(FILE), 1000).unwrap();
        assert!(!refresh(&mut store, &FakeSource::default(), 1000 + MAX_AGE).unwrap());
        assert!(!refresh(&mut store, &source(r#"{"schema": 1, "quests": []}"#), 1000 + MAX_AGE + RETRY_AFTER).unwrap());
        assert_eq!(store.turn_ins().unwrap().len(), 2);
    }

    #[test]
    fn a_first_download_can_fail() {
        let mut store = Store::open_in_memory().unwrap();
        assert!(refresh(&mut store, &FakeSource::default(), 1000).is_err());
        assert_eq!(store.turn_ins_fetched_at().unwrap(), None);
    }

    #[test]
    fn refuses_a_format_it_does_not_know() {
        let mut store = Store::open_in_memory().unwrap();
        let err = refresh(&mut store, &source(r#"{"schema": 2, "quests": []}"#), 1000).unwrap_err();
        assert!(err.to_string().contains("Update Leafline"), "{err}");
    }

    #[test]
    fn a_failure_is_not_asked_again_for_a_day() {
        let mut store = Store::open_in_memory().unwrap();
        let missing = FakeSource::default();
        assert!(refresh(&mut store, &missing, 1000).is_err());
        assert!(refresh(&mut store, &missing, 1000 + RETRY_AFTER - 1).is_err());
        assert_eq!(missing.requests.borrow().len(), 1);

        assert!(refresh(&mut store, &source(FILE), 1000 + RETRY_AFTER).unwrap());
        assert_eq!(store.failure(TURN_INS_FAILED).unwrap(), None);
    }

    #[test]
    fn resolves_only_npcs_the_map_data_knows() {
        let rows = [TurnIn { quest: 70, npc: 50 }, TurnIn { quest: 71, npc: 99 }];
        assert_eq!(resolve(&rows, |npc| npc == 50), HashMap::from([(70, 50)]));
    }
}
