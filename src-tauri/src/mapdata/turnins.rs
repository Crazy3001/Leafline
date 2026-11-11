//! Who takes each quest in, from mapleclassic.wiki's `quests` table. The client data only names
//! who gives a quest, so this is the one place that says.
//!
//! Like the drop table, the whole table is downloaded and every lookup is answered from that copy,
//! so nothing about the player's quests is sent to the wiki. Quests are matched by their id, which
//! the wiki shares with the client data, because the wiki words some names differently.

use std::collections::HashMap;

use serde::Deserialize;

use super::drops::{MAX_AGE, WIKI_API, cargo_rows, encode, reason, recent_failure, unescape};
use super::fetch::Source;
use super::store::{Store, TURN_INS_FAILED};
use crate::error::{Error, Result};

/// Rows per request, the most the wiki gives an anonymous caller.
const PAGE: usize = 500;

/// Far more pages than the table needs, so a reply that ignores `offset` can't loop forever.
const MAX_PAGES: usize = 20;

/// The quest's id in the data, and the name of who takes it in.
#[derive(Debug, Clone, PartialEq)]
pub struct TurnIn {
    pub quest: u32,
    pub npc: String,
}

/// Downloads the table if there is no copy or it is over a week old, and says whether it did. A
/// failed download keeps the old copy and, like the drop table's, is not tried again for a day.
pub fn refresh(store: &mut Store, source: &dyn Source, now: i64) -> Result<bool> {
    let fetched_at = store.turn_ins_fetched_at()?;
    if fetched_at.is_some_and(|at| now - at < MAX_AGE) {
        return Ok(false);
    }
    if let Some(reason) = recent_failure(store, TURN_INS_FAILED, now)? {
        return if fetched_at.is_some() { Ok(false) } else { Err(Error::Download(reason)) };
    }
    match download(source) {
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

fn download(source: &dyn Source) -> Result<Vec<TurnIn>> {
    let mut turn_ins = Vec::new();
    for page in 0..MAX_PAGES {
        let batch: Vec<Row> = cargo_rows(&source.get(&page_url(page * PAGE))?)?;
        let last = batch.len() < PAGE;
        turn_ins.extend(batch.into_iter().filter_map(Row::turn_in));
        if last && turn_ins.is_empty() {
            return Err(Error::Download("mapleclassic.wiki sent an empty quest table".into()));
        }
        if last {
            return Ok(turn_ins);
        }
    }
    Err(Error::Download("mapleclassic.wiki's quest table is longer than expected".into()))
}

/// One page of the whole table. It has no `where`: nothing the player is doing goes in the URL.
fn page_url(offset: usize) -> String {
    let query = [
        ("action", "cargoquery".to_string()),
        ("format", "json".to_string()),
        ("tables", "quests".to_string()),
        ("fields", "id=quest,turn_in=npc".to_string()),
        ("order_by", "_ID".to_string()),
        ("limit", PAGE.to_string()),
        ("offset", offset.to_string()),
    ];
    let encoded: Vec<String> = query.iter().map(|(k, v)| format!("{k}={}", encode(v))).collect();
    format!("{WIKI_API}?{}", encoded.join("&"))
}

/// Cargo returns every value as a string or null, under the aliases chosen in [`page_url`].
#[derive(Deserialize)]
struct Row {
    quest: Option<String>,
    npc: Option<String>,
}

impl Row {
    fn turn_in(self) -> Option<TurnIn> {
        let quest = self.quest?.trim().parse().ok()?;
        let npc = unescape(self.npc?.trim());
        (!npc.is_empty()).then_some(TurnIn { quest, npc })
    }
}

/// The wiki's name for an NPC without the note that tells two pages apart: `Sam (Maple Island)`.
pub(super) fn plain_name(name: &str) -> &str {
    name.split(" (").next().unwrap_or(name).trim()
}

/// Each quest's id and the id of the NPC who takes it in, for those the map data has both of.
/// `npc_named` finds an NPC for a quest, given who gives that quest.
pub(super) fn resolve(turn_ins: &[TurnIn], npc_named: impl Fn(u32, &str) -> Option<u32>) -> HashMap<u32, u32> {
    turn_ins.iter().filter_map(|t| Some((t.quest, npc_named(t.quest, plain_name(&t.npc))?))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mapdata::fetch::tests::FakeSource;
    use crate::mapdata::drops::RETRY_AFTER;

    // A made-up reply in Cargo's shape.
    const REPLY: &str = r#"{"cargoquery": [
        {"title": {"quest": "70", "npc": "Warden Ilse"}},
        {"title": {"quest": "71", "npc": "Bo&#039;s Brother"}},
        {"title": {"quest": null, "npc": "Nobody"}},
        {"title": {"quest": "72", "npc": null}}
    ]}"#;

    fn source(reply: &str) -> FakeSource {
        let mut source = FakeSource::default();
        source.files.insert(page_url(0), reply.as_bytes().to_vec());
        source
    }

    #[test]
    fn keeps_rows_with_a_quest_and_an_npc() {
        let mut store = Store::open_in_memory().unwrap();
        assert!(refresh(&mut store, &source(REPLY), 1000).unwrap());
        let quests: Vec<(u32, String)> = store.turn_ins().unwrap().into_iter().map(|t| (t.quest, t.npc)).collect();
        assert_eq!(quests, [(70, "Warden Ilse".to_string()), (71, "Bo's Brother".to_string())]);
        assert_eq!(store.turn_ins_fetched_at().unwrap(), Some(1000));
    }

    #[test]
    fn asks_for_the_whole_table_and_names_nothing() {
        let url = page_url(500);
        assert!(url.starts_with("https://mapleclassic.wiki/api.php?action=cargoquery&format=json&tables=quests"), "{url}");
        assert!(!url.contains("where="), "{url}");
    }

    #[test]
    fn reads_every_page() {
        let row = r#"{"title": {"quest": "70", "npc": "Warden Ilse"}}"#;
        let full = format!(r#"{{"cargoquery": [{}]}}"#, vec![row; PAGE].join(","));
        let mut pages = source(&full);
        pages.files.insert(page_url(PAGE), br#"{"cargoquery": [{"title": {"quest": "71", "npc": "Bo"}}]}"#.to_vec());
        let mut store = Store::open_in_memory().unwrap();
        refresh(&mut store, &pages, 1000).unwrap();
        assert_eq!(pages.requests.borrow().len(), 2);
    }

    #[test]
    fn downloads_at_most_weekly_and_replaces_the_old_table() {
        let mut store = Store::open_in_memory().unwrap();
        let first = source(REPLY);
        refresh(&mut store, &first, 1000).unwrap();
        assert!(!refresh(&mut store, &first, 1000 + MAX_AGE - 1).unwrap());
        assert_eq!(first.requests.borrow().len(), 1, "served from the copy");

        let later = source(r#"{"cargoquery": [{"title": {"quest": "73", "npc": "Bo"}}]}"#);
        assert!(refresh(&mut store, &later, 1000 + MAX_AGE).unwrap());
        assert_eq!(store.turn_ins().unwrap().len(), 1);
    }

    #[test]
    fn keeps_the_old_table_when_offline_or_sent_nothing() {
        let mut store = Store::open_in_memory().unwrap();
        refresh(&mut store, &source(REPLY), 1000).unwrap();
        assert!(!refresh(&mut store, &FakeSource::default(), 1000 + MAX_AGE).unwrap());
        assert!(!refresh(&mut store, &source(r#"{"cargoquery": []}"#), 1000 + MAX_AGE).unwrap());
        assert_eq!(store.turn_ins().unwrap().len(), 2);
    }

    #[test]
    fn a_first_download_can_fail() {
        let mut store = Store::open_in_memory().unwrap();
        assert!(refresh(&mut store, &FakeSource::default(), 1000).is_err());
        assert_eq!(store.turn_ins_fetched_at().unwrap(), None);
    }

    #[test]
    fn drops_the_note_that_tells_pages_apart() {
        assert_eq!(plain_name("Sam (Maple Island)"), "Sam");
        assert_eq!(plain_name("Mrs. Ming Ming"), "Mrs. Ming Ming");
    }

    #[test]
    fn a_refusal_is_not_asked_again_for_a_day() {
        let refused = source(r#"{"error": {"code": "cargogate-table", "info": "Log in to make this request."}}"#);
        let mut store = Store::open_in_memory().unwrap();
        assert!(refresh(&mut store, &refused, 1000).is_err());
        assert!(refresh(&mut store, &refused, 1000 + RETRY_AFTER - 1).is_err());
        assert_eq!(refused.requests.borrow().len(), 1);

        let fixed = source(REPLY);
        assert!(refresh(&mut store, &fixed, 1000 + RETRY_AFTER).unwrap());
        assert_eq!(store.failure(TURN_INS_FAILED).unwrap(), None);
    }
}
