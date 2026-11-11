//! Player-observed drops from mapleclassic.wiki's Cargo API (`monster_drops`).
//!
//! The client has no drop tables, so these are counts players logged: each rate comes with
//! the number of kills it was measured over and the game version. The whole table is downloaded
//! when first needed and again after a week, and every lookup is answered from that copy, so no
//! monster or item name, least of all one read from the screen, is ever sent to the wiki.
//!
//! Quest items are looked up the other way round, by item: which monsters players saw drop it.
//! The item's name comes from OCR, so the closest name in the table is kept.

use std::collections::BTreeMap;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use super::fetch::Source;
use super::store::{DROPS_FAILED, Store};
use crate::error::{Error, Result};

pub const WIKI_API: &str = "https://mapleclassic.wiki/api.php";

/// How long the downloaded table is used before the wiki is asked again.
pub const MAX_AGE: i64 = 7 * 24 * 60 * 60;

/// Rows per request, the most the wiki gives an anonymous caller.
const PAGE: usize = 500;

/// Far more pages than the table needs, so a reply that ignores `offset` can't loop forever.
const MAX_PAGES: usize = 200;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Drop {
    pub item: String,
    /// `etc`, `use`, `equip`, `meso` and so on, as the wiki files it.
    pub category: Option<String>,
    /// Observed drop rate in percent, with its confidence interval. Missing when the wiki
    /// only lists the item as a known drop.
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

/// How long after a failed download the wiki is left alone. Lookups come often, and a wiki that
/// refused once (since 2026-11-11 it refuses anonymous reads of this table) would otherwise be
/// asked again on every one. A download button the player presses tries again at once.
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

/// Downloads the whole table if there is no copy or it is over a week old. A failed download
/// keeps the old copy, so drops still work offline, and is not tried again for a day.
pub fn refresh(store: &mut Store, source: &dyn Source, now: i64) -> Result<()> {
    let fetched_at = store.drops_fetched_at()?;
    if fetched_at.is_some_and(|at| now - at < MAX_AGE) {
        return Ok(());
    }
    if let Some(reason) = recent_failure(store, DROPS_FAILED, now)? {
        return if fetched_at.is_some() { Ok(()) } else { Err(Error::Download(reason)) };
    }
    match download(source, now) {
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

fn download(source: &dyn Source, now: i64) -> Result<BTreeMap<String, MonsterDrops>> {
    let mut rows = Vec::new();
    for page in 0..MAX_PAGES {
        let batch: Vec<Row> = cargo_rows(&source.get(&page_url(page * PAGE))?)?;
        let last = batch.len() < PAGE;
        rows.extend(batch);
        if last && rows.is_empty() {
            return Err(Error::Download("mapleclassic.wiki sent an empty drop table".into()));
        }
        if last {
            return Ok(group(rows, now));
        }
    }
    Err(Error::Download("mapleclassic.wiki's drop table is longer than expected".into()))
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

/// One page of the whole table. It has no `where`: nothing the player is doing goes in the URL.
fn page_url(offset: usize) -> String {
    let query = [
        ("action", "cargoquery".to_string()),
        ("format", "json".to_string()),
        ("tables", "monster_drops".to_string()),
        (
            "fields",
            "monster_id=monster,item_id=item,category=category,drop_rate=rate,rate_low=low,rate_high=high,\
             min_quantity=qmin,max_quantity=qmax,sample_kills=kills,sample_drops=drops,game_version=version,\
             quest_flag=quest"
                .to_string(),
        ),
        ("order_by", "_ID".to_string()),
        ("limit", PAGE.to_string()),
        ("offset", offset.to_string()),
    ];
    let encoded: Vec<String> = query.iter().map(|(k, v)| format!("{k}={}", encode(v))).collect();
    format!("{WIKI_API}?{}", encoded.join("&"))
}

/// Percent-encodes everything but unreserved characters.
pub(super) fn encode(text: &str) -> String {
    text.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// Cargo returns every value as a string or null, under the aliases chosen in [`page_url`].
#[derive(Deserialize)]
struct Row {
    monster: Option<String>,
    item: Option<String>,
    category: Option<String>,
    rate: Option<String>,
    low: Option<String>,
    high: Option<String>,
    qmin: Option<String>,
    qmax: Option<String>,
    kills: Option<String>,
    drops: Option<String>,
    version: Option<String>,
    quest: Option<String>,
}

/// The rows of a Cargo reply, or the wiki's error.
pub(super) fn cargo_rows<T: DeserializeOwned>(body: &[u8]) -> Result<Vec<T>> {
    #[derive(Deserialize)]
    struct Response<T> {
        cargoquery: Option<Vec<Entry<T>>>,
        error: Option<serde_json::Value>,
    }
    #[derive(Deserialize)]
    struct Entry<T> {
        title: T,
    }
    let response: Response<T> = serde_json::from_slice(body)
        .map_err(|err| Error::Download(format!("mapleclassic.wiki sent an unexpected reply: {err}")))?;
    if let Some(error) = response.error {
        return Err(Error::Download(format!(
            "mapleclassic.wiki: {}",
            error["info"].as_str().unwrap_or("query failed")
        )));
    }
    Ok(response.cargoquery.unwrap_or_default().into_iter().map(|e| e.title).collect())
}

fn group(rows: Vec<Row>, now: i64) -> BTreeMap<String, MonsterDrops> {
    let mut out: BTreeMap<String, MonsterDrops> = BTreeMap::new();
    for row in rows {
        let Some(monster) = text(&row.monster) else { continue };
        let entry = out.entry(monster.clone()).or_insert_with(|| MonsterDrops {
            monster,
            sample_kills: None,
            game_version: None,
            fetched_at: now,
            drops: vec![],
        });
        let kills = number(&row.kills);
        entry.sample_kills = entry.sample_kills.max(kills);
        if entry.game_version.is_none() {
            entry.game_version = text(&row.version);
        }
        // The "nothing dropped" row only carries the sample size.
        let Some(item) = text(&row.item) else { continue };
        if row.category.as_deref() == Some("nothing") {
            continue;
        }
        entry.drops.push(Drop {
            item,
            category: text(&row.category),
            rate: number(&row.rate),
            rate_low: number(&row.low),
            rate_high: number(&row.high),
            min_quantity: number(&row.qmin),
            max_quantity: number(&row.qmax),
            sample_kills: kills,
            sample_drops: number(&row.drops),
            game_version: text(&row.version),
            quest: row.quest.as_deref() == Some("1"),
        });
    }
    for entry in out.values_mut() {
        // Highest rate first; listed-but-unmeasured drops last.
        entry.drops.sort_by(|a, b| b.rate.unwrap_or(-1.0).total_cmp(&a.rate.unwrap_or(-1.0)));
    }
    out
}

fn text(value: &Option<String>) -> Option<String> {
    value.as_deref().map(str::trim).filter(|v| !v.is_empty()).map(unescape)
}

/// Cargo returns values HTML-escaped: `Bubbling&#039;s Huge Bubble`.
pub fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let entity = rest.find(';').filter(|&end| end <= 10).map(|end| (&rest[1..end], end));
        let decoded = entity.and_then(|(name, end)| {
            let c = match name {
                "amp" => Some('&'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                "lt" => Some('<'),
                "gt" => Some('>'),
                _ => name
                    .strip_prefix("#x")
                    .and_then(|hex| u32::from_str_radix(hex, 16).ok())
                    .or_else(|| name.strip_prefix('#').and_then(|dec| dec.parse().ok()))
                    .and_then(char::from_u32),
            };
            c.map(|c| (c, end))
        });
        match decoded {
            Some((c, end)) => {
                out.push(c);
                rest = &rest[end + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn number<T: std::str::FromStr>(value: &Option<String>) -> Option<T> {
    value.as_deref().and_then(|v| v.trim().parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mapdata::fetch::tests::FakeSource;

    // A made-up reply in Cargo's shape.
    const REPLY: &str = r#"{"cargoquery": [
        {"title": {"monster": "Pebble Snail", "item": null, "category": "nothing", "rate": "20", "kills": "120", "version": "0.1.0"}},
        {"title": {"monster": "Pebble Snail", "item": "Pebble", "category": "etc", "rate": "48.5", "low": "39.1",
                   "high": "58", "qmin": null, "qmax": null, "kills": "120", "drops": "58", "version": "0.1.0", "quest": "0"}},
        {"title": {"monster": "Pebble Snail", "item": "Meso", "category": "meso", "rate": "70", "qmin": "3", "qmax": "5",
                   "kills": "120", "drops": "84", "version": "0.1.0", "quest": "0"}},
        {"title": {"monster": "Pebble Snail", "item": "Moss Ore", "category": "etc", "rate": null}}
    ]}"#;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    /// A store holding `reply` as the whole table, downloaded at time 1000.
    fn store_with(reply: &str) -> Store {
        let mut store = Store::open_in_memory().unwrap();
        let mut source = FakeSource::default();
        source.files.insert(page_url(0), reply.as_bytes().to_vec());
        refresh(&mut store, &source, 1000).unwrap();
        store
    }

    #[test]
    fn parses_observed_rates() {
        let store = store_with(REPLY);
        let drops = store.drops(&names(&["Pebble Snail", "Gloom Bat"])).unwrap();
        let snail = &drops[0];
        assert_eq!(snail.sample_kills, Some(120));
        assert_eq!(snail.game_version.as_deref(), Some("0.1.0"));
        let items: Vec<&str> = snail.drops.iter().map(|d| d.item.as_str()).collect();
        assert_eq!(items, ["Meso", "Pebble", "Moss Ore"]);
        assert_eq!((snail.drops[0].min_quantity, snail.drops[0].max_quantity), (Some(3), Some(5)));
        assert_eq!(snail.drops[1].rate_low, Some(39.1));
        assert_eq!((drops[1].monster.as_str(), drops[1].drops.len()), ("Gloom Bat", 0));
    }

    #[test]
    fn unescapes_names() {
        assert_eq!(unescape("Gloom&#039;s Wing &amp; Tail"), "Gloom's Wing & Tail");
        assert_eq!(unescape("&quot;Odd&#x27;s&quot; &lt;3&gt;"), "\"Odd's\" <3>");
        assert_eq!(unescape("Fish & Chips &bogus; &"), "Fish & Chips &bogus; &");

        let reply = r#"{"cargoquery": [{"title": {"monster": "Gloom&#039;s Bat", "item": "Bat&#039;s Wing",
            "category": "etc", "rate": "12.5", "kills": "40"}}]}"#;
        let drops = store_with(reply).drops(&names(&["Gloom's Bat"])).unwrap();
        assert_eq!(drops[0].drops[0].item, "Bat's Wing");
    }

    #[test]
    fn asks_for_the_whole_table_and_names_nothing() {
        let url = page_url(1000);
        assert!(
            url.starts_with("https://mapleclassic.wiki/api.php?action=cargoquery&format=json&tables=monster_drops")
        );
        assert!(url.ends_with("&order_by=_ID&limit=500&offset=1000"), "{url}");
        assert!(!url.contains("where="), "{url}");
    }

    #[test]
    fn reads_every_page() {
        let row = r#"{"title": {"monster": "Pebble Snail", "item": "Pebble", "rate": "1"}}"#;
        let full = format!(r#"{{"cargoquery": [{}]}}"#, vec![row; PAGE].join(","));
        let last = r#"{"cargoquery": [{"title": {"monster": "Gloom Bat", "item": "Wing", "rate": "2"}}]}"#;
        let mut source = FakeSource::default();
        source.files.insert(page_url(0), full.into_bytes());
        source.files.insert(page_url(PAGE), last.as_bytes().to_vec());

        let mut store = Store::open_in_memory().unwrap();
        refresh(&mut store, &source, 1000).unwrap();
        assert_eq!(source.requests.borrow().len(), 2);
        assert_eq!(store.drops(&names(&["Pebble Snail"])).unwrap()[0].drops.len(), PAGE);
        assert_eq!(store.drops(&names(&["Gloom Bat"])).unwrap()[0].drops.len(), 1);
    }

    #[test]
    fn stops_if_the_wiki_never_ends() {
        let row = r#"{"title": {"monster": "Pebble Snail", "item": "Pebble", "rate": "1"}}"#;
        let full = format!(r#"{{"cargoquery": [{}]}}"#, vec![row; PAGE].join(","));
        let mut source = FakeSource::default();
        for page in 0..MAX_PAGES {
            source.files.insert(page_url(page * PAGE), full.clone().into_bytes());
        }
        let err = refresh(&mut Store::open_in_memory().unwrap(), &source, 1000).unwrap_err();
        assert!(err.to_string().contains("longer than expected"), "{err}");
    }

    // A made-up table where a similar item sits beside the one wanted.
    const ITEM_REPLY: &str = r#"{"cargoquery": [
        {"title": {"monster": "Gloom Bat", "item": "Gloom Bat Wing", "rate": "4.5", "kills": "300", "version": "0.1.0", "quest": "1"}},
        {"title": {"monster": "Elder Gloom Bat", "item": "Gloom Bat Wing", "rate": "9", "kills": "80", "quest": "1"}},
        {"title": {"monster": "Gloom Bat", "item": "Gloom Bat Wing Fragment", "rate": "30", "kills": "300"}},
        {"title": {"monster": "Cave Owl", "item": "Gloom Bat Wing", "rate": null}}
    ]}"#;

    #[test]
    fn finds_who_drops_a_misread_item() {
        let store = store_with(ITEM_REPLY);
        assert_eq!(item_key("Gloom Bat. Wing"), "gloom bat wing");

        let found = sources_for(&store, &names(&["Gloom Bat. Wing", "Gloom Bat Wing"])).unwrap();
        assert_eq!(found.len(), 1, "both readings share one lookup");
        let wing = &found[0];
        assert_eq!(wing.item.as_deref(), Some("Gloom Bat Wing"));
        assert_eq!(wing.fetched_at, 1000);
        let droppers: Vec<(&str, Option<f64>)> = wing.droppers.iter().map(|d| (d.monster.as_str(), d.rate)).collect();
        assert_eq!(droppers, [("Elder Gloom Bat", Some(9.0)), ("Gloom Bat", Some(4.5)), ("Cave Owl", None)]);
        assert!(wing.droppers[0].quest);

        let dropped_letters = sources_for(&store, &names(&["Glom Bat Wng"])).unwrap();
        assert_eq!(dropped_letters[0].item.as_deref(), Some("Gloom Bat Wing"));
    }

    #[test]
    fn a_similar_item_is_not_taken_for_the_one_read() {
        let reply = r#"{"cargoquery": [{"title": {"monster": "Spore", "item": "Green Mushroom Cap", "rate": "5"}}]}"#;
        let found = sources_for(&store_with(reply), &names(&["Orange Mushroom Cap"])).unwrap();
        assert_eq!((found[0].item.as_ref(), found[0].droppers.len()), (None, 0));
    }

    #[test]
    fn an_item_not_in_the_table_counts_as_looked_up() {
        let found = sources_for(&store_with(REPLY), &names(&["Moon Rock"])).unwrap();
        assert_eq!((found[0].key.as_str(), found[0].item.as_ref(), found[0].droppers.len()), ("moon rock", None, 0));
    }

    #[test]
    fn nothing_is_looked_up_before_the_download() {
        let store = Store::open_in_memory().unwrap();
        assert!(sources_for(&store, &names(&["Gloom Bat Wing"])).unwrap().is_empty());
    }

    #[test]
    fn downloads_at_most_weekly() {
        let mut store = Store::open_in_memory().unwrap();
        let mut source = FakeSource::default();
        source.files.insert(page_url(0), REPLY.as_bytes().to_vec());

        refresh(&mut store, &source, 1000).unwrap();
        refresh(&mut store, &source, 1000 + MAX_AGE - 1).unwrap();
        assert_eq!(source.requests.borrow().len(), 1, "served from the copy");

        refresh(&mut store, &source, 1000 + MAX_AGE).unwrap();
        assert_eq!(source.requests.borrow().len(), 2, "asked again once stale");
    }

    #[test]
    fn a_new_download_replaces_the_old_table() {
        let mut store = store_with(REPLY);
        let reply = r#"{"cargoquery": [{"title": {"monster": "Gloom Bat", "item": "Wing", "rate": "2"}}]}"#;
        let mut source = FakeSource::default();
        source.files.insert(page_url(0), reply.as_bytes().to_vec());
        refresh(&mut store, &source, 1000 + MAX_AGE).unwrap();

        assert!(store.drops(&names(&["Pebble Snail"])).unwrap()[0].drops.is_empty());
        assert_eq!(store.drops(&names(&["Gloom Bat"])).unwrap()[0].drops[0].item, "Wing");
    }

    #[test]
    fn keeps_the_old_table_when_offline() {
        let mut store = store_with(REPLY);
        refresh(&mut store, &FakeSource::default(), 1000 + MAX_AGE).unwrap();
        assert_eq!(store.drops(&names(&["Pebble Snail"])).unwrap()[0].drops.len(), 3);
        assert_eq!(store.drops_fetched_at().unwrap(), Some(1000));
    }

    #[test]
    fn a_first_download_can_fail() {
        let mut store = Store::open_in_memory().unwrap();
        assert!(refresh(&mut store, &FakeSource::default(), 1000).is_err());
        assert_eq!(store.drops_fetched_at().unwrap(), None);
    }

    #[test]
    fn an_empty_reply_does_not_wipe_the_table() {
        let mut store = store_with(REPLY);
        let mut source = FakeSource::default();
        source.files.insert(page_url(0), br#"{"cargoquery": []}"#.to_vec());
        refresh(&mut store, &source, 1000 + MAX_AGE).unwrap();
        assert_eq!(store.drops(&names(&["Pebble Snail"])).unwrap()[0].drops.len(), 3);
    }

    // The wiki's refusal since 2026-11-11, in Cargo's error shape.
    const REFUSED: &str = r#"{"error": {"code": "cargogate-table", "info": "Log in to make this request."}}"#;

    fn refusing() -> FakeSource {
        let mut source = FakeSource::default();
        source.files.insert(page_url(0), REFUSED.as_bytes().to_vec());
        source
    }

    #[test]
    fn a_refusal_is_not_asked_again_for_a_day() {
        let mut store = store_with(REPLY);
        let source = refusing();
        let stale = 1000 + MAX_AGE;
        refresh(&mut store, &source, stale).unwrap();
        refresh(&mut store, &source, stale + RETRY_AFTER - 1).unwrap();
        assert_eq!(source.requests.borrow().len(), 1, "lookups answered from the old copy");
        assert_eq!(store.drops(&names(&["Pebble Snail"])).unwrap()[0].drops.len(), 3);

        refresh(&mut store, &source, stale + RETRY_AFTER).unwrap();
        assert_eq!(source.requests.borrow().len(), 2, "asked again a day later");
    }

    #[test]
    fn without_a_copy_the_refusal_is_repeated_not_reasked() {
        let mut store = Store::open_in_memory().unwrap();
        let source = refusing();
        let first = refresh(&mut store, &source, 1000).unwrap_err().to_string();
        let again = refresh(&mut store, &source, 1001).unwrap_err().to_string();
        assert!(first.contains("Log in to make this request"), "{first}");
        assert_eq!(first, again);
        assert_eq!(source.requests.borrow().len(), 1);
    }

    #[test]
    fn forgetting_the_failure_asks_at_once_and_success_clears_it() {
        let mut store = Store::open_in_memory().unwrap();
        refresh(&mut store, &refusing(), 1000).unwrap_err();
        store.set_failure(DROPS_FAILED, None).unwrap();
        let mut source = FakeSource::default();
        source.files.insert(page_url(0), REPLY.as_bytes().to_vec());
        refresh(&mut store, &source, 1001).unwrap();
        assert_eq!(store.drops_fetched_at().unwrap(), Some(1001));
        assert_eq!(store.failure(DROPS_FAILED).unwrap(), None);
    }

    #[test]
    fn a_download_that_works_after_the_wait_clears_the_failure() {
        let mut store = Store::open_in_memory().unwrap();
        refresh(&mut store, &refusing(), 1000).unwrap_err();
        let mut source = FakeSource::default();
        source.files.insert(page_url(0), REPLY.as_bytes().to_vec());
        refresh(&mut store, &source, 1000 + RETRY_AFTER).unwrap();
        assert_eq!(store.failure(DROPS_FAILED).unwrap(), None);
    }
}
