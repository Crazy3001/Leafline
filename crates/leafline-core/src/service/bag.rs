//! Sell or keep: an Item Inventory tab as read from the game, each item named by its picture and
//! given what says to keep it. The program checks on a hotkey, and both it and the website once the
//! tab has been open and held still. Pictures come from OSMS through whatever cache the caller keeps.
//!
//! A check tries the items a character near this level most likely carries, plus every item of
//! the tab whose picture is kept already; "Look further" tries every item of the tab, which may
//! download many pictures, so the player asks for it.

use std::collections::{BTreeSet, HashSet};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::{Deserialize, Serialize};

use super::builds::planned_gear;
use super::host::Host;
use crate::clock::now;
use crate::error::{Error, Result};
use crate::mapdata::bag;
use crate::mapdata::osms::ItemKind;
use crate::readers::inventory::{BagSlot, Inventory, SLOT_SIZE, SlotPicture, Tab};
use crate::vision::Rect;

/// Emitted with each check's result, and with `busy` while one runs.
pub const BAG_EVENT: &str = "bag";

/// Where item pictures come from: the program's picture cache, or the website's downloads.
pub trait Pictures {
    /// Whether the picture at `url` is kept already, so trying it costs nothing.
    fn has(&self, url: &str) -> bool;
    /// The pictures at these addresses, decoded, downloaded first when not kept. One that can't be
    /// had is left out.
    fn load(&self, wanted: Vec<(u32, String)>) -> Vec<(u32, image::RgbaImage)>;
}

/// A tab read from the game, and whose it is.
#[derive(Debug, Clone)]
pub struct BagRead {
    pub inventory: Inventory,
    pub character: Option<String>,
    pub level: Option<u32>,
    /// Read from a video (`inventory::read_video`): items are named by `bag::name_video`.
    pub video: bool,
}

/// What a check found.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BagCheck {
    /// A check is under way; the rest is from the one before.
    pub busy: bool,
    /// Unix seconds.
    pub at: i64,
    pub tab: Option<Tab>,
    pub character: Option<String>,
    pub items: Vec<BagItem>,
    /// Slots holding something no picture matched.
    pub unknown: Vec<u32>,
    /// Every item of the tab was tried.
    pub wide: bool,
    /// Pictures "Look further" would still download.
    pub more: usize,
    pub problem: Option<String>,
}

impl BagCheck {
    pub fn failed(problem: String) -> Self {
        Self { at: now(), problem: Some(problem), ..Self::default() }
    }
}

/// One slot's item.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BagItem {
    pub slot: u32,
    pub count: u32,
    /// Every item drawn the same, best first. More than one when the picture can't tell them apart.
    pub ids: Vec<u32>,
    pub name: String,
    /// What an NPC pays for one, from the item data, when the items drawn alike agree. What the
    /// data's price means in Classic isn't confirmed.
    pub price: Option<i64>,
    /// The lowest and highest when they don't agree.
    pub price_range: Option<(i64, i64)>,
    /// Can't be traded or dropped: an NPC may still buy it.
    pub untradable: bool,
    /// What says to keep it, from the data and the build.
    pub keep: Vec<Keep>,
    /// Quests that ask for it, for the window to check against the ones being done.
    pub wanted_by: Vec<QuestAsk>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestAsk {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Keep {
    /// A crafting recipe uses it.
    Recipe,
    /// Gear the build will wear.
    Plan,
    /// The data marks it a quest item.
    Quest,
    /// A scroll: worth more to players than to an NPC, as a rule.
    Scroll,
}

/// The pictures a check of `read` tries, by item: what a character of its level likely carries
/// (every item of the tab with `wide`), and every item of the tab whose picture is kept already.
/// The website fetches them all at once before the check, which then finds them in the browser's
/// cache.
pub fn wanted(h: &dyn Host, pictures: &dyn Pictures, read: &BagRead, wide: bool) -> Result<Vec<(u32, String)>> {
    let (tab, _, catalog) = ready(h, read)?;
    let data = h.map();
    let index = data.pictures();
    let url = |id: u32| index.resolve(&format!("/item/{id}")).map(str::to_string);
    let every = bag::every(&catalog, tab);
    let mut ids: BTreeSet<u32> = if wide {
        every.iter().copied().collect()
    } else {
        let Some(atlas) = data.atlas() else { return Ok(Vec::new()) };
        let planned = read.level.map_or_else(HashSet::new, |level| planned_gear(h, read.character.as_deref(), level));
        let extra: Vec<u32> = planned.iter().copied().collect();
        let dropped = data.dropped_pairs().unwrap_or_default();
        bag::likely(&atlas, &catalog, &dropped, read.level, tab, &extra).into_iter().collect()
    };
    ids.extend(every.iter().copied().filter(|&id| url(id).is_some_and(|u| pictures.has(&u))));
    Ok(ids.into_iter().filter_map(|id| Some((id, url(id)?))).collect())
}

/// The tab, the atlas and the item data a check needs, or why it can't be made.
fn ready(h: &dyn Host, read: &BagRead) -> Result<(Tab, std::sync::Arc<crate::mapdata::Atlas>, std::sync::Arc<crate::build::catalog::Catalog>)> {
    if !h.settings().show_pictures {
        return Err(Error::Invalid(
            "Sell or keep names items by their pictures. Turn on Show game pictures in Settings → Data.".into(),
        ));
    }
    let Some(tab) = read.inventory.tab else {
        return Err(Error::Invalid("Leafline couldn't tell which tab is open.".into()));
    };
    if bag::kinds(tab).is_empty() {
        return Err(Error::Invalid("Cash and Deco items aren't in the item data. Open Equip, Use, Set-up or Etc.".into()));
    }
    let data = h.map();
    let (Some(atlas), Some(catalog)) = (data.atlas(), data.catalog()) else {
        return Err(Error::Invalid("Download the map data first: items are named from it.".into()));
    };
    Ok((tab, atlas, catalog))
}

/// Names the items of the tab in `read` by their pictures. `wide` tries every item of the tab.
pub fn name_items(h: &dyn Host, pictures: &dyn Pictures, read: &BagRead, wide: bool) -> Result<BagCheck> {
    let (tab, atlas, catalog) = ready(h, read)?;
    let index = h.map().pictures();
    let url = |id: u32| index.resolve(&format!("/item/{id}")).map(str::to_string);
    let planned = read.level.map_or_else(HashSet::new, |level| planned_gear(h, read.character.as_deref(), level));
    let every = bag::every(&catalog, tab);
    let images = pictures.load(wanted(h, pictures, read, wide)?);
    let candidates: Vec<(u32, &image::RgbaImage)> = images.iter().map(|(id, image)| (*id, image)).collect();
    let more = every.iter().filter(|&&id| url(id).is_some_and(|u| !pictures.has(&u))).count();

    let facts = bag::KeepFacts::new(&atlas, &catalog, planned);
    let mut check = BagCheck { at: now(), tab: Some(tab), character: read.character.clone(), wide, more, ..BagCheck::default() };
    for slot in &read.inventory.slots {
        // How many there are must be known too, or the slot's worth would be wrong.
        let named = if slot.count_unsure {
            None
        } else if read.video {
            bag::name_video(&slot.picture, &candidates)
        } else {
            bag::name(&slot.picture, &candidates)
        };
        let Some(named) = named else {
            check.unknown.push(slot.index);
            continue;
        };
        let records: Vec<_> = named.ids.iter().filter_map(|&id| catalog.item(id)).collect();
        let Some(first) = records.first() else { continue };
        let prices: Vec<i64> = records.iter().filter_map(|r| r.price).collect();
        let (low, high) = (prices.iter().min().copied(), prices.iter().max().copied());
        let any = |test: &dyn Fn(u32) -> bool| named.ids.iter().any(|&id| test(id));
        let mut keep = Vec::new();
        if any(&|id| facts.in_recipes.contains(&id)) {
            keep.push(Keep::Recipe);
        }
        if any(&|id| facts.planned.contains(&id)) {
            keep.push(Keep::Plan);
        }
        if records.iter().any(|r| r.quest_item) {
            keep.push(Keep::Quest);
        }
        if records.iter().all(|r| r.kind == ItemKind::Scroll) {
            keep.push(Keep::Scroll);
        }
        check.items.push(BagItem {
            slot: slot.index,
            count: slot.count.unwrap_or(1),
            ids: named.ids.clone(),
            name: first.name.clone(),
            price: (low == high).then_some(low).flatten(),
            price_range: low.zip(high).filter(|(l, h)| l != h),
            untradable: records.iter().all(|r| r.untradable),
            keep,
            wanted_by: named
                .ids
                .iter()
                .flat_map(|id| facts.wanted_by.get(id).into_iter().flatten())
                .map(|(id, name)| QuestAsk { id: *id, name: name.clone() })
                .collect(),
        });
    }
    Ok(check)
}

/// A tab as the website's live reading hands it from its worker to the one that keeps the
/// records: each slot's picture as base64, the pixels' colours and then which the count hides.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeenBag {
    pub tab: Option<Tab>,
    pub slots: Vec<SeenSlot>,
    pub character: Option<String>,
    pub level: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeenSlot {
    pub index: u32,
    pub count: Option<u32>,
    #[serde(default)]
    pub count_unsure: bool,
    pub picture: String,
}

impl SeenBag {
    pub fn new(inventory: &Inventory, character: Option<String>, level: Option<u32>) -> Self {
        let slots = inventory
            .slots
            .iter()
            .map(|slot| {
                let mut bytes: Vec<u8> = slot.picture.pixels.iter().flatten().copied().collect();
                bytes.extend(slot.picture.hidden.iter().map(|&h| u8::from(h)));
                SeenSlot { index: slot.index, count: slot.count, count_unsure: slot.count_unsure, picture: STANDARD.encode(bytes) }
            })
            .collect();
        Self { tab: inventory.tab, slots, character, level }
    }

    /// The tab back as read; `None` when a picture isn't one.
    pub fn read(&self) -> Option<BagRead> {
        let cells = (SLOT_SIZE * SLOT_SIZE) as usize;
        let slots = self
            .slots
            .iter()
            .map(|slot| {
                let bytes = STANDARD.decode(&slot.picture).ok()?;
                if bytes.len() != cells * 4 {
                    return None;
                }
                let (colours, hidden) = bytes.split_at(cells * 3);
                let picture = SlotPicture {
                    pixels: colours.chunks(3).map(|c| [c[0], c[1], c[2]]).collect(),
                    hidden: hidden.iter().map(|&h| h != 0).collect(),
                };
                Some(BagSlot { index: slot.index, count: slot.count, count_unsure: slot.count_unsure, picture })
            })
            .collect::<Option<Vec<_>>>()?;
        let inventory = Inventory { tab: self.tab, slots, window: Rect::new(0.0, 0.0, 0.0, 0.0) };
        Some(BagRead { inventory, character: self.character.clone(), level: self.level, video: true })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_seen_tab_comes_back_as_read() {
        let cells = (SLOT_SIZE * SLOT_SIZE) as usize;
        let picture = SlotPicture {
            pixels: (0..cells).map(|i| [i as u8, (i / 3) as u8, 7]).collect(),
            hidden: (0..cells).map(|i| i % 5 == 0).collect(),
        };
        let inventory = Inventory {
            tab: Some(Tab::Etc),
            slots: vec![BagSlot { index: 4, count: Some(12), count_unsure: false, picture }],
            window: Rect::new(1.0, 2.0, 3.0, 4.0),
        };
        let seen = SeenBag::new(&inventory, Some("Someone".into()), Some(31));
        let json = serde_json::to_string(&seen).unwrap();
        let back: SeenBag = serde_json::from_str(&json).unwrap();
        let read = back.read().expect("read back");
        assert_eq!((read.inventory.tab, read.level), (Some(Tab::Etc), Some(31)));
        assert_eq!(read.inventory.slots, inventory.slots);
    }
}
