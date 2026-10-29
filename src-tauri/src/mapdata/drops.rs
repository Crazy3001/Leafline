//! Player-observed drops from mapleclassic.wiki's Cargo API (`monster_drops`).
//!
//! The client has no drop tables, so these are counts players logged: each rate comes with
//! the number of kills it was measured over and the game version. They are fetched only when
//! the user opens a map's drops, one request for all of that map's monsters, kept for a week.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::fetch::Source;
use super::store::Store;
use crate::error::{Error, Result};

pub const WIKI_API: &str = "https://mapleclassic.wiki/api.php";

/// How long fetched drops are reused before the wiki is asked again.
pub const MAX_AGE: i64 = 7 * 24 * 60 * 60;

/// Monsters per request, keeping the query string short.
const BATCH: usize = 20;

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

/// Drops for each of `monsters` (by name), from the cache when it is under a week old and
/// from the wiki otherwise. Monsters the wiki has nothing on come back with no drops.
pub fn drops_for(store: &mut Store, source: &dyn Source, monsters: &[String], now: i64) -> Result<Vec<MonsterDrops>> {
    let (mut found, missing) = store.cached_drops(monsters, now - MAX_AGE)?;
    let missing: Vec<String> = missing.into_iter().filter(|m| is_queryable(m)).collect();
    for batch in missing.chunks(BATCH) {
        let body = source.get(&query_url(batch))?;
        let fetched = parse(&body, batch, now)?;
        store.save_drops(&fetched)?;
        found.extend(fetched.into_values());
    }
    found.sort_by_key(|d| monsters.iter().position(|m| *m == d.monster));
    Ok(found)
}

/// Names go into the query in double quotes, so a name holding one is never sent.
fn is_queryable(name: &str) -> bool {
    !name.is_empty() && !name.contains(['"', '\\'])
}

fn query_url(monsters: &[String]) -> String {
    let names: Vec<String> = monsters.iter().map(|m| format!("\"{m}\"")).collect();
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
        ("where", format!("monster_id IN ({})", names.join(","))),
        ("limit", "500".to_string()),
    ];
    let encoded: Vec<String> = query.iter().map(|(k, v)| format!("{k}={}", encode(v))).collect();
    format!("{WIKI_API}?{}", encoded.join("&"))
}

/// Percent-encodes everything but unreserved characters.
fn encode(text: &str) -> String {
    text.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// Cargo returns every value as a string or null, under the aliases chosen in [`query_url`].
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

fn parse(body: &[u8], asked: &[String], now: i64) -> Result<BTreeMap<String, MonsterDrops>> {
    #[derive(Deserialize)]
    struct Response {
        cargoquery: Option<Vec<Entry>>,
        error: Option<serde_json::Value>,
    }
    #[derive(Deserialize)]
    struct Entry {
        title: Row,
    }
    let response: Response = serde_json::from_slice(body)
        .map_err(|err| Error::Download(format!("mapleclassic.wiki sent an unexpected reply: {err}")))?;
    if let Some(error) = response.error {
        return Err(Error::Download(format!(
            "mapleclassic.wiki: {}",
            error["info"].as_str().unwrap_or("query failed")
        )));
    }
    let mut out: BTreeMap<String, MonsterDrops> = asked
        .iter()
        .map(|m| {
            let empty = MonsterDrops {
                monster: m.clone(),
                sample_kills: None,
                game_version: None,
                fetched_at: now,
                drops: vec![],
            };
            (m.clone(), empty)
        })
        .collect();
    for row in response.cargoquery.unwrap_or_default().into_iter().map(|e| e.title) {
        let Some(entry) = text(&row.monster).and_then(|m| out.get_mut(&m)) else { continue };
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
    Ok(out)
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

    #[test]
    fn parses_observed_rates() {
        let drops = parse(REPLY.as_bytes(), &names(&["Pebble Snail", "Gloom Bat"]), 5).unwrap();
        let snail = &drops["Pebble Snail"];
        assert_eq!(snail.sample_kills, Some(120));
        assert_eq!(snail.game_version.as_deref(), Some("0.1.0"));
        let items: Vec<&str> = snail.drops.iter().map(|d| d.item.as_str()).collect();
        assert_eq!(items, ["Meso", "Pebble", "Moss Ore"]);
        assert_eq!((snail.drops[0].min_quantity, snail.drops[0].max_quantity), (Some(3), Some(5)));
        assert_eq!(snail.drops[1].rate_low, Some(39.1));
        assert!(drops["Gloom Bat"].drops.is_empty());
    }

    #[test]
    fn unescapes_names() {
        assert_eq!(unescape("Gloom&#039;s Wing &amp; Tail"), "Gloom's Wing & Tail");
        assert_eq!(unescape("&quot;Odd&#x27;s&quot; &lt;3&gt;"), "\"Odd's\" <3>");
        assert_eq!(unescape("Fish & Chips &bogus; &"), "Fish & Chips &bogus; &");

        let reply = r#"{"cargoquery": [{"title": {"monster": "Gloom&#039;s Bat", "item": "Bat&#039;s Wing",
            "category": "etc", "rate": "12.5", "kills": "40"}}]}"#;
        let drops = parse(reply.as_bytes(), &names(&["Gloom's Bat"]), 5).unwrap();
        assert_eq!(drops["Gloom's Bat"].drops[0].item, "Bat's Wing");
    }

    #[test]
    fn builds_one_escaped_query() {
        let url = query_url(&names(&["Pebble Snail", "Gloom's Bat"]));
        assert!(
            url.starts_with("https://mapleclassic.wiki/api.php?action=cargoquery&format=json&tables=monster_drops")
        );
        assert!(url.contains("where=monster_id%20IN%20%28%22Pebble%20Snail%22%2C%22Gloom%27s%20Bat%22%29"), "{url}");
        assert!(!is_queryable("Bad \" name"));
    }

    #[test]
    fn caches_for_a_week() {
        let mut store = Store::open_in_memory().unwrap();
        let wanted = names(&["Pebble Snail", "Gloom Bat"]);
        let mut source = FakeSource::default();
        source.files.insert(query_url(&wanted), REPLY.as_bytes().to_vec());

        let first = drops_for(&mut store, &source, &wanted, 1000).unwrap();
        assert_eq!(first.len(), 2);
        assert_eq!(first[0].monster, "Pebble Snail");
        assert_eq!(first[0].drops.len(), 3);

        let again = drops_for(&mut store, &source, &wanted, 1000 + MAX_AGE - 1).unwrap();
        assert_eq!(again, first);
        assert_eq!(source.requests.borrow().len(), 1, "served from the cache");

        drops_for(&mut store, &source, &wanted, 1000 + MAX_AGE + 1).unwrap();
        assert_eq!(source.requests.borrow().len(), 2, "asked again once stale");
    }
}
