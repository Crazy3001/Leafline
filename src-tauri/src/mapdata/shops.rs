//! NPC shops: who sells what, where and for how much. The map data has no shop stock, so
//! Leafline keeps its own file, `shops/shops.json` in the Leafline-data repository. It is
//! downloaded like the other reference data and kept on the player's PC, so prices can be
//! corrected without a new release. Every entry carries the date it was last seen, because
//! prices can change after launch.
//!
//! The file names NPCs, maps and items as the game shows them. They are matched to the map
//! data's ids when the atlas is built; an entry the data can't place still shows by name.

use serde::{Deserialize, Serialize};

use super::atlas::{Atlas, MapRef, fold, similarity};
use super::fetch::Source;
use super::store::Store;
use crate::error::{Error, Result};

/// Where the latest shop file lives. Nothing about the player goes with the request.
pub const SHOPS_URL: &str = "https://raw.githubusercontent.com/Crazy3001/Leafline-data/main/shops/shops.json";
/// The only `schema` this reader understands.
pub const SCHEMA: u32 = 1;
/// The file is checked for changes at most this often.
const MAX_AGE: i64 = 24 * 60 * 60;

/// The shop file as written.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShopFile {
    pub schema: u32,
    #[serde(default)]
    pub about: String,
    pub shops: Vec<ShopRecord>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShopRecord {
    /// The NPC's name, as the game shows it.
    pub npc: String,
    /// What the NPC is, such as "Grocer".
    #[serde(default)]
    pub role: Option<String>,
    /// The part of the world, as the list groups shops.
    #[serde(default)]
    pub region: Option<String>,
    /// The map's name, as its minimap header shows it.
    pub map: String,
    #[serde(default)]
    pub street: Option<String>,
    pub items: Vec<ShopRecordItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShopRecordItem {
    pub item: String,
    /// In mesos.
    pub price: u64,
    /// A town standing the shop asks for before it sells this, as the game words it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub needs: Option<String>,
    /// The day this price was last seen in the game, `YYYY-MM-DD`.
    pub seen: String,
}

/// Reads a downloaded shop file, refusing one this version can't read or that looks damaged.
pub fn parse(body: &[u8]) -> Result<ShopFile> {
    let file: ShopFile =
        serde_json::from_slice(body).map_err(|err| Error::Invalid(format!("The shop file can't be read: {err}")))?;
    if file.schema != SCHEMA {
        return Err(Error::Invalid(format!(
            "The shop file is in format {}; this version of Leafline reads format {SCHEMA}. Update Leafline.",
            file.schema
        )));
    }
    for shop in &file.shops {
        if shop.npc.trim().is_empty() || shop.map.trim().is_empty() {
            return Err(Error::Invalid("The shop file has a shop with no NPC or map.".into()));
        }
        for item in &shop.items {
            if item.item.trim().is_empty() || !is_day(&item.seen) {
                return Err(Error::Invalid(format!("The shop file has an entry at {} with no item or date.", shop.npc)));
            }
        }
    }
    Ok(file)
}

/// `YYYY-MM-DD`.
fn is_day(text: &str) -> bool {
    let parts: Vec<&str> = text.split('-').collect();
    parts.len() == 3
        && parts.iter().zip([4, 2, 2]).all(|(p, n)| p.len() == n && p.chars().all(|c| c.is_ascii_digit()))
}

/// Downloads the shop file when the cached one is a day old, and keeps the old one if that fails.
/// True when a new copy was stored.
pub fn refresh(store: &mut Store, source: &dyn Source, now: i64) -> Result<bool> {
    let fetched_at = store.shops_fetched_at()?;
    if fetched_at.is_some_and(|at| now - at < MAX_AGE) {
        return Ok(false);
    }
    match source.get(SHOPS_URL).and_then(|body| parse(&body).map(|_| body)) {
        Ok(body) => {
            let changed = store.shop_file()?.as_deref() != Some(body.as_slice());
            store.replace_shop_file(&body, now)?;
            Ok(changed)
        }
        Err(err) if fetched_at.is_some() => {
            eprintln!("shop file update failed: {err}");
            Ok(false)
        }
        Err(err) => Err(err),
    }
}

/// A shop as the app shows it, placed in the map data where it could be.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Shop {
    pub npc: String,
    pub npc_id: Option<u32>,
    pub role: Option<String>,
    /// The map, when the map data has it.
    pub map: Option<MapRef>,
    /// The map as the file names it.
    pub map_name: String,
    pub items: Vec<ShopItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShopItem {
    pub item: String,
    /// The item in the map data, when its name matched one.
    pub item_id: Option<u32>,
    pub price: u64,
    pub needs: Option<String>,
    pub seen: String,
}

/// One shop selling an item, for the item page.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShopOffer {
    pub npc: String,
    pub npc_id: Option<u32>,
    pub role: Option<String>,
    pub map: Option<MapRef>,
    pub map_name: String,
    /// Portals from where the player is, when known.
    pub hops: Option<u32>,
    pub price: u64,
    pub needs: Option<String>,
    pub seen: String,
}

/// A shop with its NPC, map and items matched to the map data.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct PlacedShop {
    pub record: ShopRecord,
    pub npc_id: Option<u32>,
    pub map_id: Option<u32>,
    /// Each item's id, in the record's order.
    pub item_ids: Vec<Option<u32>>,
}

/// How alike a hand-written item name and the data's must be to count as the same item, and how
/// far ahead of the next best it must be.
const ITEM_MATCH: f64 = 0.85;
const ITEM_MARGIN: f64 = 0.03;

impl Atlas {
    /// Matches each shop's map by name and street, then its NPC among those standing there,
    /// or anywhere by name when the map isn't known.
    pub(super) fn place_shops(&self, shops: Vec<ShopRecord>) -> Vec<PlacedShop> {
        shops
            .into_iter()
            .map(|record| {
                let map_id = self
                    .find(&record.map, record.street.as_deref())
                    .filter(|m| m.score >= 0.9)
                    .map(|m| m.card.map.id);
                let name = fold(&record.npc);
                let named = |id: &u32| self.npc_names.get(id).is_some_and(|n| fold(n) == name);
                let npc_id = map_id
                    .and_then(|map| self.npcs_on.get(&map)?.iter().copied().find(named))
                    .or_else(|| {
                        let mut ids: Vec<u32> = self.npc_names.keys().copied().filter(named).collect();
                        ids.sort_unstable();
                        ids.first().copied()
                    });
                PlacedShop { item_ids: vec![None; record.items.len()], record, npc_id, map_id }
            })
            .collect()
    }

    /// Matches every shop item to the data's items again, after the item names changed.
    pub(super) fn match_shop_items(&mut self) {
        let ids: Vec<Vec<Option<u32>>> = self
            .shops
            .iter()
            .map(|shop| shop.record.items.iter().map(|i| self.shop_item_id(&i.item)).collect())
            .collect();
        for (shop, ids) in self.shops.iter_mut().zip(ids) {
            shop.item_ids = ids;
        }
    }

    /// The data's item id for an item a shop names: the same name folded ("(F)" reads as gear
    /// for women), or failing that one clearly closest spelling ("Jeans" for "Jean").
    fn shop_item_id(&self, name: &str) -> Option<u32> {
        let key = fold(name);
        if let Some(id) = self.item_keys.get(&key) {
            return Some(*id);
        }
        let plain = key.strip_suffix(" m").or_else(|| key.strip_suffix(" f")).unwrap_or(&key);
        if let Some(id) = self.item_keys.get(plain) {
            return Some(*id);
        }
        let mut scored: Vec<(f64, u32)> = self.item_keys.iter().map(|(k, id)| (similarity(&key, k), *id)).collect();
        scored.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
        let best = *scored.first()?;
        let next = scored.iter().find(|(_, id)| *id != best.1).map_or(0.0, |s| s.0);
        (best.0 >= ITEM_MATCH && best.0 - next >= ITEM_MARGIN).then_some(best.1)
    }

    fn shop_view(&self, shop: &PlacedShop) -> Shop {
        Shop {
            npc: shop.record.npc.clone(),
            npc_id: shop.npc_id,
            role: shop.record.role.clone(),
            map: shop.map_id.map(|id| self.map_ref(id)),
            map_name: shop.record.map.clone(),
            items: shop
                .record
                .items
                .iter()
                .zip(&shop.item_ids)
                .map(|(i, id)| ShopItem {
                    item: i.item.clone(),
                    item_id: *id,
                    price: i.price,
                    needs: i.needs.clone(),
                    seen: i.seen.clone(),
                })
                .collect(),
        }
    }

    /// The shops on a map.
    pub fn shops_on(&self, map_id: u32) -> Vec<Shop> {
        self.shops.iter().filter(|s| s.map_id == Some(map_id)).map(|s| self.shop_view(s)).collect()
    }

    /// The shop an NPC keeps, if any.
    pub fn shop_of(&self, npc_id: u32) -> Option<Shop> {
        self.shops.iter().find(|s| s.npc_id == Some(npc_id)).map(|s| self.shop_view(s))
    }

    /// Every shop selling an item, by any of its ids or by name, the cheapest first. `hops` gives
    /// portals from where the player is.
    pub fn sold_by(&self, ids: &[u32], name: &str, hops: impl Fn(u32) -> Option<u32>) -> Vec<ShopOffer> {
        let key = fold(name);
        let mut offers: Vec<ShopOffer> = Vec::new();
        for shop in &self.shops {
            // A shop can list an item twice (two looks of it); one offer each at its lowest price.
            let best = shop
                .record
                .items
                .iter()
                .zip(&shop.item_ids)
                .filter(|(i, id)| id.is_some_and(|id| ids.contains(&id)) || (id.is_none() && fold(&i.item) == key))
                .map(|(i, _)| i)
                .min_by_key(|i| i.price);
            if let Some(item) = best {
                offers.push(ShopOffer {
                    npc: shop.record.npc.clone(),
                    npc_id: shop.npc_id,
                    role: shop.record.role.clone(),
                    map: shop.map_id.map(|id| self.map_ref(id)),
                    map_name: shop.record.map.clone(),
                    hops: shop.map_id.and_then(&hops),
                    price: item.price,
                    needs: item.needs.clone(),
                    seen: item.seen.clone(),
                });
            }
        }
        offers.sort_by(|a, b| a.price.cmp(&b.price).then(a.hops.unwrap_or(u32::MAX).cmp(&b.hops.unwrap_or(u32::MAX))));
        offers
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mapdata::atlas::tests::world;
    use crate::mapdata::fetch::tests::FakeSource;

    // A made-up shop file over the made-up test world.
    const FILE: &str = r#"{"schema": 1, "about": "test", "shops": [
        {"npc": "Warden Ilse", "role": "Grocer", "map": "Mossbank", "street": "Fern Road", "items": [
            {"item": "Twig Wand", "price": 400, "seen": "2026-11-10"},
            {"item": "Moss Scroll", "price": 9000, "needs": "Town Resident", "seen": "2026-11-11"},
            {"item": "Moss scroll", "price": 8000, "seen": "2026-11-11"}]},
        {"npc": "Innkeeper Bo", "map": "Mossbank Inn", "street": "Fern Road", "items": [
            {"item": "Twig Wand", "price": 350, "seen": "2026-11-09"},
            {"item": "Mystery Bun", "price": 5, "seen": "2026-11-09"}]},
        {"npc": "Wandering Pip", "map": "Nowhere At All", "items": [
            {"item": "Twig Wand", "price": 999, "seen": "2026-11-09"}]}]}"#;

    #[test]
    fn names_written_by_hand_find_the_data_items() {
        let atlas = world().with_item_genders(
            [
                (10, "Blue Jean Shorts", Some("male")),
                (20, "Green Bennis Chainmail", Some("male")),
                (21, "Green Bennis Chainmail", Some("female")),
                (30, "Red Headband", None),
                (31, "Black Headband", None),
            ]
            .into_iter(),
        );
        assert_eq!(atlas.shop_item_id("Green Bennis Chainmail (F)"), Some(21));
        assert_eq!(atlas.shop_item_id("Green Bennis Chainmail (M)"), Some(20));
        assert_eq!(atlas.shop_item_id("Blue Jeans Shorts"), Some(10), "a close spelling");
        assert_eq!(atlas.shop_item_id("red headband"), Some(30));
        assert_eq!(atlas.shop_item_id("Blue Headband"), None, "near two items is a guess, not a match");
    }

    fn stocked() -> Atlas {
        let file = parse(FILE.as_bytes()).unwrap();
        world()
            .with_item_names([(90, "Twig Wand".to_string()), (91, "Moss Scroll".to_string())].into())
            .with_shops(file.shops)
    }

    #[test]
    fn shops_are_placed_on_their_maps_and_npcs() {
        let atlas = stocked();
        let shops = atlas.shops_on(1);
        assert_eq!(shops.len(), 1);
        assert_eq!((shops[0].npc_id, shops[0].role.as_deref()), (Some(50), Some("Grocer")));
        assert_eq!(shops[0].items[0].item_id, Some(90));
        assert_eq!(shops[0].items[1].needs.as_deref(), Some("Town Resident"));
        assert_eq!(atlas.shop_of(51).unwrap().items[1].item_id, None, "an item the data lacks stays by name");
        let lost = atlas.sold_by(&[90], "Twig Wand", |_| None).into_iter().find(|o| o.npc == "Wandering Pip").unwrap();
        assert_eq!((lost.map, lost.npc_id), (None, None), "a shop the data can't place still shows");
    }

    #[test]
    fn an_item_lists_every_shop_cheapest_first() {
        let atlas = stocked();
        let offers = atlas.sold_by(&[90], "Twig Wand", |map| (map == 5).then_some(2));
        let rows: Vec<_> = offers.iter().map(|o| (o.npc.as_str(), o.price, o.hops)).collect();
        assert_eq!(rows, [("Innkeeper Bo", 350, Some(2)), ("Warden Ilse", 400, None), ("Wandering Pip", 999, None)]);
        // Listed twice in one shop: one offer, at the lower price.
        let scroll = atlas.sold_by(&[91], "Moss Scroll", |_| None);
        assert_eq!((scroll.len(), scroll[0].price, scroll[0].needs.clone()), (1, 8000, None));
    }

    #[test]
    fn a_file_in_another_format_or_without_dates_is_refused() {
        assert!(parse(br#"{"schema": 2, "shops": []}"#).is_err());
        let undated = r#"{"schema": 1, "shops": [{"npc": "A", "map": "B", "items": [{"item": "C", "price": 1, "seen": "soon"}]}]}"#;
        assert!(parse(undated.as_bytes()).is_err());
    }

    #[test]
    fn refresh_keeps_the_last_good_copy() {
        let mut store = Store::open_in_memory().unwrap();
        let mut source = FakeSource::default();
        assert!(store.shops().unwrap().is_empty(), "no shops before the first download");
        assert!(refresh(&mut store, &source, 1000).is_err(), "nothing cached and nothing to download");
        source.files.insert(SHOPS_URL.to_string(), FILE.as_bytes().to_vec());
        assert!(refresh(&mut store, &source, 1000).unwrap());
        assert!(!refresh(&mut store, &source, 2000).unwrap(), "checked at most once a day");
        source.files.insert(SHOPS_URL.to_string(), b"not json".to_vec());
        assert!(!refresh(&mut store, &source, 1000 + MAX_AGE).unwrap(), "a bad download keeps the old file");
        assert_eq!(store.shop_file().unwrap().as_deref(), Some(FILE.as_bytes()));
        assert_eq!(store.shops().unwrap().len(), 3);
    }
}
