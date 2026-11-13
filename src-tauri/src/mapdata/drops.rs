//! Which monster drops what, from `drops/signature.json` in Leafline's data repository.
//!
//! The client has no drop tables. The file lists each monster's etc drops as the game's own item
//! text, StrategyWiki's drop table and players' reports have it: presence only, no rates. The
//! whole file is downloaded when first needed and checked again after a day, and every lookup is
//! answered from that copy, so no monster or item name, least of all one read from the screen,
//! is ever sent anywhere. Until 2026-11-11 this came from mapleclassic.wiki's `monster_drops`
//! table, which no longer answers anonymous requests.
//!
//! Quest items are looked up the other way round, by item: which monsters drop it. The item's
//! name comes from OCR, so the closest name in the table is kept.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::fetch::Source;
use super::store::{DROPS_FAILED, Store};
use crate::error::{Error, Result};

/// Where the drop list lives. Nothing about the player goes with the request.
pub const DROPS_URL: &str = "https://raw.githubusercontent.com/Crazy3001/Leafline-data/main/drops/signature.json";
/// The only `schema` this reader understands.
const SCHEMA: u32 = 1;

/// How long the downloaded list is used before it is checked again.
pub const MAX_AGE: i64 = 24 * 60 * 60;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Drop {
    pub item: String,
    /// `etc`, `use`, `equip`, `meso` and so on.
    pub category: Option<String>,
    /// Observed drop rate in percent, with its confidence interval. The drop list has none, so
    /// these are missing: the item is a known drop.
    pub rate: Option<f64>,
    pub rate_low: Option<f64>,
    pub rate_high: Option<f64>,
    pub min_quantity: Option<u32>,
    pub max_quantity: Option<u32>,
    pub sample_kills: Option<u32>,
    pub sample_drops: Option<u32>,
    pub game_version: Option<String>,
    /// Only drops while a quest wants it.
    pub quest: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonsterDrops {
    pub monster: String,
    /// The most kills any of its rates was measured over.
    pub sample_kills: Option<u32>,
    pub game_version: Option<String>,
    pub fetched_at: i64,
    pub drops: Vec<Drop>,
}

/// How long after a failed download the source is left alone. Lookups come often, and a source
/// that refused once would otherwise be asked again on every one. A download button the player
/// presses tries again at once.
pub const RETRY_AFTER: i64 = 24 * 60 * 60;

/// Why the last download under `key` failed, if that was under [`RETRY_AFTER`] ago.
pub(super) fn recent_failure(store: &Store, key: &str, now: i64) -> Result<Option<String>> {
    Ok(store.failure(key)?.filter(|(at, _)| now - at < RETRY_AFTER).map(|(_, reason)| reason))
}

/// The reason kept for a failed download: the message without "download failed: ".
pub(super) fn reason(err: &Error) -> String {
    match err {
        Error::Download(reason) => reason.clone(),
        other => other.to_string(),
    }
}

/// Downloads the whole list if there is no copy or it is over a day old. A failed download
/// keeps the old copy, so drops still work offline, and is not tried again for a day.
pub fn refresh(store: &mut Store, source: &dyn Source, now: i64) -> Result<()> {
    let fetched_at = store.drops_fetched_at()?;
    if fetched_at.is_some_and(|at| now - at < MAX_AGE) {
        return Ok(());
    }
    if let Some(reason) = recent_failure(store, DROPS_FAILED, now)? {
        return if fetched_at.is_some() { Ok(()) } else { Err(Error::Download(reason)) };
    }
    match source.get(DROPS_URL).and_then(|body| parse(&body, now)) {
        Ok(table) => store.replace_drops(&table, now),
        Err(err) => {
            store.set_failure(DROPS_FAILED, Some((now, &reason(&err))))?;
            if fetched_at.is_none() {
                return Err(err);
            }
            eprintln!("drop table update failed: {err}");
            Ok(())
        }
    }
}

/// The list as written, grouped by monster name. Monsters that share a name (the same monster in
/// another area) share their drops, and items that share a name (the three Omok sets) show once.
fn parse(body: &[u8], now: i64) -> Result<BTreeMap<String, MonsterDrops>> {
    #[derive(Deserialize)]
    struct File {
        schema: u32,
        monsters: Vec<Monster>,
    }
    #[derive(Deserialize)]
    struct Monster {
        name: String,
        drops: Vec<Listed>,
    }
    #[derive(Deserialize)]
    struct Listed {
        name: String,
        #[serde(default)]
        quest: bool,
    }
    let file: File = serde_json::from_slice(body)
        .map_err(|err| Error::Download(format!("Leafline's drop list can't be read: {err}")))?;
    if file.schema != SCHEMA {
        return Err(Error::Download(format!(
            "Leafline's drop list is in format {}; this version reads format {SCHEMA}. Update Leafline.",
            file.schema
        )));
    }
    let mut out: BTreeMap<String, MonsterDrops> = BTreeMap::new();
    for monster in file.monsters {
        let name = monster.name.trim().to_string();
        if name.is_empty() {
            continue;
        }
        let entry = out.entry(name.clone()).or_insert_with(|| MonsterDrops {
            monster: name,
            sample_kills: None,
            game_version: None,
            fetched_at: now,
            drops: vec![],
        });
        for listed in monster.drops {
            let item = listed.name.trim().to_string();
            if item.is_empty() || entry.drops.iter().any(|d| d.item == item) {
                continue;
            }
            entry.drops.push(Drop {
                item,
                category: Some("etc".into()),
                rate: None,
                rate_low: None,
                rate_high: None,
                min_quantity: None,
                max_quantity: None,
                sample_kills: None,
                sample_drops: None,
                game_version: None,
                quest: listed.quest,
            });
        }
    }
    out.retain(|_, m| !m.drops.is_empty());
    if out.is_empty() {
        return Err(Error::Download("Leafline's drop list names no drops".into()));
    }
    Ok(out)
}

/// The monsters players saw drop one item.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemSources {
    /// The name as read, folded (see [`item_key`]).
    pub key: String,
    /// The wiki's name for the item, when it knows one close to what was read.
    pub item: Option<String>,
    pub fetched_at: i64,
    /// Highest observed rate first.
    pub droppers: Vec<Dropper>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Dropper {
    pub monster: String,
    pub rate: Option<f64>,
    pub rate_low: Option<f64>,
    pub rate_high: Option<f64>,
    pub sample_kills: Option<u32>,
    pub game_version: Option<String>,
    pub quest: bool,
}

/// Below this similarity a name in the table is not taken for the item that was read: high
/// enough that `Orange Mushroom Cap` is not read as `Green Mushroom Cap`.
const MIN_ITEM_SCORE: f64 = 0.8;

/// An OCR'd item name reduced to lowercase words, so small misreads share a cache entry:
/// `Evil Eye. Tail` and `Evil Eye Tail` both become `evil eye tail`.
pub fn item_key(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Who drops each of `items` (names as read), from the downloaded table. Takes no [`Source`]:
/// what was read stays on this machine. Empty until the table has been downloaded.
pub fn sources_for(store: &Store, items: &[String]) -> Result<Vec<ItemSources>> {
    let Some(fetched_at) = store.drops_fetched_at()? else { return Ok(Vec::new()) };
    let mut keys: Vec<String> = Vec::new();
    for key in items.iter().map(|i| item_key(i)) {
        if !keys.contains(&key) {
            keys.push(key);
        }
    }
    let names = store.dropped_items()?;
    let mut found = Vec::with_capacity(keys.len());
    for key in keys {
        let item = closest_item(&names, &key);
        let droppers = match &item {
            Some(item) => droppers_of(store, item)?,
            None => Vec::new(),
        };
        found.push(ItemSources { key, item, fetched_at, droppers });
    }
    Ok(found)
}

fn closest_item(names: &[String], key: &str) -> Option<String> {
    names
        .iter()
        .map(|name| (strsim::normalized_levenshtein(&item_key(name), key), name))
        .filter(|(score, _)| *score >= MIN_ITEM_SCORE)
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, name)| name.clone())
}

fn droppers_of(store: &Store, item: &str) -> Result<Vec<Dropper>> {
    let mut droppers: Vec<Dropper> = Vec::new();
    for dropper in store.droppers(item)? {
        if !droppers.iter().any(|d| d.monster == dropper.monster) {
            droppers.push(dropper);
        }
    }
    droppers.sort_by(|a, b| b.rate.unwrap_or(-1.0).total_cmp(&a.rate.unwrap_or(-1.0)));
    Ok(droppers)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mapdata::fetch::tests::FakeSource;

    // A made-up list in the data repository's shape; none of it is real game data.
    const LIST: &str = r#"{"schema": 1, "about": "test", "monsters": [
        {"id": 1, "name": "Pebble Snail", "drops": [
            {"item": 10, "name": "Pebble", "sources": ["client"]},
            {"item": 11, "name": "Moss Ore", "sources": ["player"]},
            {"item": 12, "name": "Omok Piece: Snail", "sources": ["player"]},
            {"item": 13, "name": "Omok Piece: Snail", "sources": ["player"]}]},
        {"id": 2, "name": "Pebble Snail", "drops": [{"item": 14, "name": "Shell Dust", "sources": ["client"]}]},
        {"id": 3, "name": "Elder Gloom Bat", "drops": [{"item": 20, "name": "Gloom Bat Wing", "sources": ["player"], "quest": true}]},
        {"id": 4, "name": "Gloom Bat", "drops": [{"item": 21, "name": "Gloom Bat Wing Fragment", "sources": ["client"]}]},
        {"id": 5, "name": "Cave Owl", "drops": [{"item": 20, "name": "Gloom Bat Wing", "sources": ["strategywiki"]}]},
        {"id": 6, "name": "Spore", "drops": [{"item": 30, "name": "Green Mushroom Cap", "sources": ["client"]}]},
        {"id": 7, "name": "Empty Thing", "drops": []}
    ]}"#;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    fn source(body: &str) -> FakeSource {
        let mut source = FakeSource::default();
        source.files.insert(DROPS_URL.to_string(), body.as_bytes().to_vec());
        source
    }

    /// A store holding `LIST`, downloaded at time 1000.
    fn store_with_list() -> Store {
        let mut store = Store::open_in_memory().unwrap();
        refresh(&mut store, &source(LIST), 1000).unwrap();
        store
    }

    #[test]
    fn lists_known_drops_without_rates() {
        let store = store_with_list();
        let snail = &store.drops(&names(&["Pebble Snail"])).unwrap()[0];
        let items: Vec<&str> = snail.drops.iter().map(|d| d.item.as_str()).collect();
        assert_eq!(items.len(), 4, "two areas' drops together, the Omok piece once: {items:?}");
        assert!(items.contains(&"Shell Dust") && items.contains(&"Omok Piece: Snail"));
        assert!(snail.drops.iter().all(|d| d.rate.is_none() && d.category.as_deref() == Some("etc")));
        assert_eq!(snail.fetched_at, 1000);
        assert!(store.drops(&names(&["Elder Gloom Bat"])).unwrap()[0].drops[0].quest);
    }

    #[test]
    fn asks_once_for_the_whole_file_and_names_nothing() {
        let mut store = Store::open_in_memory().unwrap();
        let list = source(LIST);
        refresh(&mut store, &list, 1000).unwrap();
        assert_eq!(*list.requests.borrow(), [DROPS_URL]);
        assert!(!DROPS_URL.contains('?'));
    }

    #[test]
    fn finds_who_drops_a_misread_item() {
        let store = store_with_list();
        assert_eq!(item_key("Gloom Bat. Wing"), "gloom bat wing");

        let found = sources_for(&store, &names(&["Gloom Bat. Wing", "Gloom Bat Wing"])).unwrap();
        assert_eq!(found.len(), 1, "both readings share one lookup");
        let wing = &found[0];
        assert_eq!(wing.item.as_deref(), Some("Gloom Bat Wing"));
        assert_eq!(wing.fetched_at, 1000);
        let mut droppers: Vec<&str> = wing.droppers.iter().map(|d| d.monster.as_str()).collect();
        droppers.sort_unstable();
        assert_eq!(droppers, ["Cave Owl", "Elder Gloom Bat"]);

        let dropped_letters = sources_for(&store, &names(&["Glom Bat Wng"])).unwrap();
        assert_eq!(dropped_letters[0].item.as_deref(), Some("Gloom Bat Wing"));
    }

    #[test]
    fn a_similar_item_is_not_taken_for_the_one_read() {
        let found = sources_for(&store_with_list(), &names(&["Orange Mushroom Cap"])).unwrap();
        assert_eq!((found[0].item.as_ref(), found[0].droppers.len()), (None, 0));
    }

    #[test]
    fn an_item_not_in_the_list_counts_as_looked_up() {
        let found = sources_for(&store_with_list(), &names(&["Moon Rock"])).unwrap();
        assert_eq!((found[0].key.as_str(), found[0].item.as_ref(), found[0].droppers.len()), ("moon rock", None, 0));
    }

    #[test]
    fn nothing_is_looked_up_before_the_download() {
        let store = Store::open_in_memory().unwrap();
        assert!(sources_for(&store, &names(&["Gloom Bat Wing"])).unwrap().is_empty());
    }

    #[test]
    fn downloads_at_most_daily_and_replaces_the_old_list() {
        let mut store = Store::open_in_memory().unwrap();
        let first = source(LIST);
        refresh(&mut store, &first, 1000).unwrap();
        refresh(&mut store, &first, 1000 + MAX_AGE - 1).unwrap();
        assert_eq!(first.requests.borrow().len(), 1, "served from the copy");

        let later = source(r#"{"schema": 1, "monsters": [{"name": "Gloom Bat", "drops": [{"name": "Wing"}]}]}"#);
        refresh(&mut store, &later, 1000 + MAX_AGE).unwrap();
        assert!(store.drops(&names(&["Pebble Snail"])).unwrap()[0].drops.is_empty());
        assert_eq!(store.drops(&names(&["Gloom Bat"])).unwrap()[0].drops[0].item, "Wing");
    }

    #[test]
    fn keeps_the_old_list_when_offline_or_sent_nothing() {
        let mut store = store_with_list();
        refresh(&mut store, &FakeSource::default(), 1000 + MAX_AGE).unwrap();
        refresh(&mut store, &source(r#"{"schema": 1, "monsters": []}"#), 1000 + MAX_AGE + RETRY_AFTER).unwrap();
        assert_eq!(store.drops(&names(&["Pebble Snail"])).unwrap()[0].drops.len(), 4);
        assert_eq!(store.drops_fetched_at().unwrap(), Some(1000));
    }

    #[test]
    fn a_first_download_can_fail() {
        let mut store = Store::open_in_memory().unwrap();
        assert!(refresh(&mut store, &FakeSource::default(), 1000).is_err());
        assert_eq!(store.drops_fetched_at().unwrap(), None);
    }

    #[test]
    fn refuses_a_format_it_does_not_know() {
        let mut store = Store::open_in_memory().unwrap();
        let err = refresh(&mut store, &source(r#"{"schema": 2, "monsters": []}"#), 1000).unwrap_err();
        assert!(err.to_string().contains("Update Leafline"), "{err}");
    }

    #[test]
    fn a_failure_is_not_asked_again_for_a_day() {
        let mut store = store_with_list();
        let offline = FakeSource::default();
        let stale = 1000 + MAX_AGE;
        refresh(&mut store, &offline, stale).unwrap();
        refresh(&mut store, &offline, stale + RETRY_AFTER - 1).unwrap();
        assert_eq!(offline.requests.borrow().len(), 1, "lookups answered from the old copy");

        refresh(&mut store, &offline, stale + RETRY_AFTER).unwrap();
        assert_eq!(offline.requests.borrow().len(), 2, "asked again a day later");
    }

    #[test]
    fn without_a_copy_the_failure_is_repeated_not_reasked() {
        let mut store = Store::open_in_memory().unwrap();
        let offline = FakeSource::default();
        let first = refresh(&mut store, &offline, 1000).unwrap_err().to_string();
        let again = refresh(&mut store, &offline, 1001).unwrap_err().to_string();
        assert_eq!(first, again);
        assert_eq!(offline.requests.borrow().len(), 1);
    }

    #[test]
    fn forgetting_the_failure_asks_at_once_and_success_clears_it() {
        let mut store = Store::open_in_memory().unwrap();
        refresh(&mut store, &FakeSource::default(), 1000).unwrap_err();
        store.set_failure(DROPS_FAILED, None).unwrap();
        refresh(&mut store, &source(LIST), 1001).unwrap();
        assert_eq!(store.drops_fetched_at().unwrap(), Some(1001));
        assert_eq!(store.failure(DROPS_FAILED).unwrap(), None);
    }
}
