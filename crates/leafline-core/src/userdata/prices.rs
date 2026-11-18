//! The price log: prices the player noted for items, looked up when they hover the item again.
//! There is no market data here, only the player's own records.

use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use strsim::normalized_levenshtein;

use super::UserData;
use crate::error::{Error, Result};
use crate::mapdata::drops::item_key;

/// How closely a read name must match a logged one to count as the same item.
pub(super) const SAME_ITEM: f64 = 0.85;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PriceKind {
    /// The item changed hands at this price.
    Traded,
    /// Someone asked this price (a shop or a chat post).
    Asking,
}

impl PriceKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Traded => "traded",
            Self::Asking => "asking",
        }
    }

    fn parse(text: &str) -> Self {
        if text == "traded" { Self::Traded } else { Self::Asking }
    }
}

/// A price to record, with the item as its tooltip was read.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewPrice {
    pub item: String,
    /// The `(+N)` upgrade count.
    pub upgrades: Option<u32>,
    /// Remaining enhancement slots.
    pub slots: Option<u32>,
    /// The stat lines, for telling copies of the item apart, such as `DEX +2, Weapon Def. +21`.
    #[serde(default)]
    pub stats: String,
    pub price: u64,
    pub kind: PriceKind,
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceEntry {
    pub id: i64,
    pub item: String,
    pub upgrades: Option<u32>,
    pub slots: Option<u32>,
    pub stats: String,
    pub price: u64,
    pub kind: PriceKind,
    pub note: Option<String>,
    /// Unix seconds.
    pub recorded_at: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceStats {
    pub count: u32,
    /// How many of them were trades rather than asking prices.
    pub traded: u32,
    pub median: u64,
    pub low: u64,
    pub high: u64,
    pub first_at: i64,
    pub last_at: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceLog {
    /// The item's name as first logged, when it has records.
    pub item: Option<String>,
    /// Newest first.
    pub entries: Vec<PriceEntry>,
    pub all: Option<PriceStats>,
    /// Records with the same upgrade count and slots as the item asked about.
    pub like_this: Option<PriceStats>,
}

impl UserData {
    pub fn record_price(&mut self, price: &NewPrice, now: i64) -> Result<i64> {
        let name = price.item.trim();
        let key = item_key(name);
        if key.is_empty() {
            return Err(Error::Invalid("The item's name wasn't read, so the price can't be logged.".into()));
        }
        if price.price == 0 {
            return Err(Error::Invalid("Enter a price above zero.".into()));
        }
        // A misread name is filed with the item it was close to.
        let key = self.closest_key(&key)?.unwrap_or(key);
        let note = price.note.as_deref().map(str::trim).filter(|n| !n.is_empty());
        self.conn.execute(
            "INSERT INTO prices (item, item_key, upgrades, slots, stats, price, kind, note, recorded_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                name,
                key,
                price.upgrades,
                price.slots,
                price.stats.trim(),
                price.price as i64,
                price.kind.as_str(),
                note,
                now
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn delete_price(&mut self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM prices WHERE id = ?1", [id])?;
        Ok(())
    }

    /// Everything logged for the item named `item`, and how the prices spread, overall and for
    /// copies with the same upgrades and slots.
    pub fn price_log(&self, item: &str, upgrades: Option<u32>, slots: Option<u32>) -> Result<PriceLog> {
        let Some(key) = self.closest_key(&item_key(item))? else {
            return Ok(PriceLog { item: None, entries: Vec::new(), all: None, like_this: None });
        };
        let entries = self.entries("WHERE item_key = ?1", [&key])?;
        let like: Vec<&PriceEntry> =
            entries.iter().filter(|e| e.upgrades.unwrap_or(0) == upgrades.unwrap_or(0) && e.slots == slots).collect();
        Ok(PriceLog {
            item: entries.last().map(|e| e.item.clone()),
            all: stats(&entries.iter().collect::<Vec<_>>()),
            like_this: stats(&like),
            entries,
        })
    }

    /// Every record, newest first, for export.
    pub fn all_prices(&self) -> Result<Vec<PriceEntry>> {
        self.entries("", [])
    }

    fn entries<P: rusqlite::Params>(&self, filter: &str, params: P) -> Result<Vec<PriceEntry>> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT id, item, upgrades, slots, stats, price, kind, note, recorded_at FROM prices {filter}
             ORDER BY recorded_at DESC, id DESC"
        ))?;
        let rows = stmt.query_map(params, |row| {
            Ok(PriceEntry {
                id: row.get(0)?,
                item: row.get(1)?,
                upgrades: row.get(2)?,
                slots: row.get(3)?,
                stats: row.get(4)?,
                price: row.get::<_, i64>(5)? as u64,
                kind: PriceKind::parse(&row.get::<_, String>(6)?),
                note: row.get(7)?,
                recorded_at: row.get(8)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// The logged item key that `key` names: itself, or the closest one when OCR misread it.
    fn closest_key(&self, key: &str) -> Result<Option<String>> {
        if key.is_empty() {
            return Ok(None);
        }
        let exact: Option<String> = self
            .conn
            .query_row("SELECT item_key FROM prices WHERE item_key = ?1 LIMIT 1", [key], |row| row.get(0))
            .optional()?;
        if exact.is_some() {
            return Ok(exact);
        }
        let mut stmt = self.conn.prepare("SELECT DISTINCT item_key FROM prices")?;
        let keys = stmt.query_map([], |row| row.get::<_, String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(keys
            .into_iter()
            .map(|k| (normalized_levenshtein(&k, key), k))
            .filter(|(score, _)| *score >= SAME_ITEM)
            .max_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, k)| k))
    }
}

fn stats(entries: &[&PriceEntry]) -> Option<PriceStats> {
    let mut prices: Vec<u64> = entries.iter().map(|e| e.price).collect();
    prices.sort_unstable();
    let n = prices.len();
    if n == 0 {
        return None;
    }
    let median = if n % 2 == 1 { prices[n / 2] } else { (prices[n / 2 - 1] + prices[n / 2]).div_ceil(2) };
    Some(PriceStats {
        count: n as u32,
        traded: entries.iter().filter(|e| e.kind == PriceKind::Traded).count() as u32,
        median,
        low: prices[0],
        high: prices[n - 1],
        first_at: entries.iter().map(|e| e.recorded_at).min()?,
        last_at: entries.iter().map(|e| e.recorded_at).max()?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Made-up items and prices.
    fn price(item: &str, upgrades: Option<u32>, slots: Option<u32>, price: u64, kind: PriceKind) -> NewPrice {
        NewPrice { item: item.into(), upgrades, slots, stats: String::new(), price, kind, note: None }
    }

    #[test]
    fn summarises_an_items_prices() {
        let mut db = UserData::open_in_memory().unwrap();
        db.record_price(&price("Fernleaf Cap", None, Some(7), 40_000, PriceKind::Asking), 100).unwrap();
        db.record_price(&price("Fernleaf Cap", None, Some(7), 30_000, PriceKind::Traded), 200).unwrap();
        db.record_price(&price("Fernleaf Cap", Some(1), Some(6), 90_000, PriceKind::Traded), 300).unwrap();
        db.record_price(&price("Fernleaf Cap", None, Some(7), 35_000, PriceKind::Traded), 400).unwrap();
        db.record_price(&price("Moss Boots", None, Some(5), 1_000, PriceKind::Asking), 500).unwrap();

        let log = db.price_log("Fernleaf Cap", None, Some(7)).unwrap();
        assert_eq!(log.entries.len(), 4);
        assert_eq!(log.entries[0].price, 35_000, "newest first");
        let all = log.all.unwrap();
        assert_eq!((all.count, all.traded, all.low, all.high, all.median), (4, 3, 30_000, 90_000, 37_500));
        assert_eq!((all.first_at, all.last_at), (100, 400));
        let like = log.like_this.unwrap();
        assert_eq!((like.count, like.median), (3, 35_000));

        let scrolled = db.price_log("Fernleaf Cap", Some(1), Some(6)).unwrap();
        assert_eq!(scrolled.like_this.unwrap().median, 90_000);
    }

    #[test]
    fn misread_names_find_the_logged_item() {
        let mut db = UserData::open_in_memory().unwrap();
        db.record_price(&price("Fernleaf Cap", None, None, 10, PriceKind::Asking), 1).unwrap();
        // OCR dropped a letter: filed under the logged item, and found from a misread too.
        db.record_price(&price("Fernlea Cap", None, None, 20, PriceKind::Asking), 2).unwrap();
        let log = db.price_log("Fernleaf C4p", None, None).unwrap();
        assert_eq!(log.item.as_deref(), Some("Fernleaf Cap"));
        assert_eq!(log.entries.len(), 2);
        assert!(db.price_log("Moss Boots", None, None).unwrap().entries.is_empty());
    }

    #[test]
    fn deletes_and_rejects_bad_records() {
        let mut db = UserData::open_in_memory().unwrap();
        let id = db.record_price(&price("Moss Boots", None, None, 500, PriceKind::Traded), 1).unwrap();
        assert!(db.record_price(&price("  ", None, None, 500, PriceKind::Traded), 1).is_err());
        assert!(db.record_price(&price("Moss Boots", None, None, 0, PriceKind::Traded), 1).is_err());
        db.delete_price(id).unwrap();
        assert!(db.all_prices().unwrap().is_empty());
    }

    #[test]
    fn upgrades_keep_existing_records() {
        let dir = std::env::temp_dir().join(format!("leafline-prices-{}", std::process::id()));
        let path = dir.join("leafline.sqlite3");
        {
            let mut db = UserData::open(&path).unwrap();
            db.record_price(&price("Moss Boots", None, None, 500, PriceKind::Traded), 1).unwrap();
        }
        let db = UserData::open(&path).unwrap();
        assert_eq!(db.all_prices().unwrap().len(), 1);
        drop(db);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
