//! Sell or keep: naming what an inventory tab holds by its pictures, and the facts that say what
//! to keep.
//!
//! The game draws each item's picture in its slot pixel for pixel (two window pixels to one of
//! the picture's at 1440p), so laying the picture OSMS publishes over an empty slot's ground
//! gives back the slot exactly. Each picture stands at its own place in the slot, so it is lined
//! up by its top edge and the right end of its top rows, which the count in the bottom left
//! never covers, then tried a little either way.
//!
//! Some items share a picture outright, such as a tier of hat scrolls; those come back together
//! and are shown as one of them, never guessed. Pictures are downloaded at runtime from OSMS
//! and kept on the player's PC, like every other picture Leafline shows.

use std::collections::{BTreeSet, HashMap, HashSet};

use image::RgbaImage;

use super::atlas::Atlas;
use super::osms::ItemKind;
use crate::build::catalog::Catalog;
use crate::readers::inventory::{GROUND, SLOT_SIZE, SlotPicture, Tab, off_ground};

/// A match whose pixels differ by less than this on average is the item (exact ones come out
/// near 0; one a pixel off, or another item, 25 and up).
const ACCEPT: f32 = 8.0;
/// Matches this close to the best are the same picture.
const SAME_PICTURE: f32 = 2.0;
/// How far either way from the lined-up place to try.
const SHIFT: i32 = 2;
/// Pictures tried at every place when lining up finds none.
const RETRIES: usize = 5;
/// Rows from the top of a picture whose right end lines it up.
const TOP_ROWS: u32 = 8;
/// Monster levels whose drops are likely, around the character's level.
const BELOW: u32 = 20;
const ABOVE: u32 = 10;
/// A picture's pixel counts as drawn at this opacity and up.
const OPAQUE: u8 = 32;

/// The item kinds a tab holds. Cash and Deco hold none Leafline knows.
pub fn kinds(tab: Tab) -> &'static [ItemKind] {
    match tab {
        Tab::Equip => &[ItemKind::Equip],
        Tab::Use => &[ItemKind::Use, ItemKind::Scroll],
        Tab::Setup => &[ItemKind::Setup],
        Tab::Etc => &[ItemKind::Etc],
        Tab::Cash | Tab::Deco => &[],
    }
}

/// Items a character at `level` most likely carries in `tab`: what monsters near their level
/// drop, what shops sell, what quests near their level ask for or give, and `extra` (tracked
/// items, the build's gear, items named before). Every item of the tab's kinds when `level` is
/// `None` and nothing else narrows it.
pub fn likely(
    atlas: &Atlas,
    catalog: &Catalog,
    dropped: &[(String, String)],
    level: Option<u32>,
    tab: Tab,
    extra: &[u32],
) -> Vec<u32> {
    let kinds = kinds(tab);
    let near = |l: u32| level.is_none_or(|level| l + BELOW >= level && l <= level + ABOVE);
    let mob_levels: HashMap<String, u32> =
        atlas.monsters.values().map(|m| (fold(&m.name), m.level)).collect();
    let by_name: HashMap<String, u32> = catalog
        .items()
        .iter()
        .filter(|i| kinds.contains(&i.kind))
        .map(|i| (fold(&i.name), i.id))
        .collect();
    let mut ids: BTreeSet<u32> = BTreeSet::new();
    for (monster, item) in dropped {
        let level = mob_levels.get(&fold(monster)).copied();
        if level.is_none_or(near)
            && let Some(&id) = by_name.get(&fold(item))
        {
            ids.insert(id);
        }
    }
    for shop in &atlas.shops {
        ids.extend(shop.item_ids.iter().flatten());
    }
    for quest in atlas.quests.values().filter(|q| q.level_min.is_none_or(near)) {
        ids.extend(quest.items.iter().map(|need| need.item_id));
        ids.extend(quest.reward_items.iter().map(|reward| reward.item_id));
    }
    ids.extend(extra);
    // Repeated items look the same: every copy goes in, so a match can say which it may be.
    let with_copies: BTreeSet<u32> = ids.iter().flat_map(|&id| catalog.copies_of(id)).collect();
    with_copies.into_iter().filter(|&id| catalog.item(id).is_some_and(|i| kinds.contains(&i.kind))).collect()
}

/// Every item of the tab's kinds.
pub fn every(catalog: &Catalog, tab: Tab) -> Vec<u32> {
    let kinds = kinds(tab);
    catalog.items().iter().filter(|i| kinds.contains(&i.kind)).map(|i| i.id).collect()
}

fn fold(name: &str) -> String {
    name.trim().to_lowercase()
}

/// What a slot's picture matched: every item drawn the same, best first.
#[derive(Debug, Clone, PartialEq)]
pub struct Named {
    pub ids: Vec<u32>,
    /// How far the best is from the slot, on average per pixel.
    pub score: f32,
}

/// Names the item in a slot from the pictures given, or `None` when none fits.
pub fn name(slot: &SlotPicture, pictures: &[(u32, &RgbaImage)]) -> Option<Named> {
    let (top, right) = slot_corner(slot)?;
    let mut scored: Vec<(f32, u32)> = pictures
        .iter()
        .filter_map(|&(id, picture)| {
            let (ptop, pright) = picture_corner(picture)?;
            let (dx, dy) = (right as i32 - pright as i32, top as i32 - ptop as i32);
            let best = (-SHIFT..=SHIFT)
                .flat_map(|sx| (-SHIFT..=SHIFT).map(move |sy| (sx, sy)))
                .map(|(sx, sy)| difference(slot, picture, dx + sx, dy + sy))
                .fold(f32::INFINITY, f32::min);
            Some((best, id))
        })
        .collect();
    scored.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    // A covered edge (a picked slot's frame, a long count) can throw the lining up off: the best
    // few are tried everywhere in the slot.
    if scored.first().is_some_and(|(s, _)| *s > ACCEPT) {
        for entry in scored.iter_mut().take(RETRIES) {
            let Some(&(_, picture)) = pictures.iter().find(|(id, _)| *id == entry.1) else { continue };
            let (w, h) = picture.dimensions();
            let span = |size: u32| -(SHIFT + 1)..=(SLOT_SIZE as i32 - size as i32 + SHIFT + 1);
            for dx in span(w) {
                for dy in span(h) {
                    entry.0 = entry.0.min(difference(slot, picture, dx, dy));
                }
            }
        }
        scored.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    }
    let &(best, _) = scored.first()?;
    if best > ACCEPT {
        return None;
    }
    let ids = scored.iter().take_while(|(s, _)| *s <= best + SAME_PICTURE).map(|(_, id)| *id).collect();
    Some(Named { ids, score: best })
}

/// Whether a slot pixel is part of what is compared: inside the slot's frame (a picked slot is
/// outlined) and clear of the count.
fn compared(slot: &SlotPicture, x: u32, y: u32) -> bool {
    x > 0 && y > 0 && x + 1 < SLOT_SIZE && y + 1 < SLOT_SIZE && !slot.is_hidden(x, y)
}

/// The picture's top row in the slot, and the right end of its top rows.
fn slot_corner(slot: &SlotPicture) -> Option<(u32, u32)> {
    let drawn = |x: u32, y: u32| compared(slot, x, y) && off_ground(slot.get(x, y));
    let top = (0..SLOT_SIZE).find(|&y| (0..SLOT_SIZE).any(|x| drawn(x, y)))?;
    let right = (top..(top + TOP_ROWS).min(SLOT_SIZE)).flat_map(|y| (0..SLOT_SIZE).filter(move |&x| drawn(x, y))).max()?;
    Some((top, right))
}

fn picture_corner(picture: &RgbaImage) -> Option<(u32, u32)> {
    let drawn = |x: u32, y: u32| picture.get_pixel(x, y).0[3] >= OPAQUE;
    let (w, h) = picture.dimensions();
    let top = (0..h).find(|&y| (0..w).any(|x| drawn(x, y)))?;
    let right = (top..(top + TOP_ROWS).min(h)).flat_map(|y| (0..w).filter(move |&x| drawn(x, y))).max()?;
    Some((top, right))
}

/// The average difference, per pixel and channel summed, between the slot and the picture laid
/// on an empty slot's ground at `(dx, dy)`, over the pixels either of them draws on.
fn difference(slot: &SlotPicture, picture: &RgbaImage, dx: i32, dy: i32) -> f32 {
    let (w, h) = picture.dimensions();
    let (mut total, mut count) = (0u32, 0u32);
    for y in 0..SLOT_SIZE {
        for x in 0..SLOT_SIZE {
            if !compared(slot, x, y) {
                continue;
            }
            let (px, py) = (x as i32 - dx, y as i32 - dy);
            let laid = if px >= 0 && py >= 0 && (px as u32) < w && (py as u32) < h {
                let [r, g, b, a] = picture.get_pixel(px as u32, py as u32).0;
                let blend = |c: u8, ground: u8| {
                    ((u32::from(c) * u32::from(a) + u32::from(ground) * (255 - u32::from(a)) + 127) / 255) as u8
                };
                [blend(r, GROUND[0]), blend(g, GROUND[1]), blend(b, GROUND[2])]
            } else {
                GROUND
            };
            let seen = slot.get(x, y);
            if !off_ground(seen) && !off_ground(laid) {
                continue;
            }
            total += seen.iter().zip(laid).map(|(&a, b)| u32::from(a.abs_diff(b))).sum::<u32>();
            count += 1;
        }
    }
    if count == 0 { f32::INFINITY } else { total as f32 / count as f32 }
}

/// What says to keep an item, from the data and the player's own plan.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct KeepFacts {
    /// Items crafting recipes use.
    pub in_recipes: HashSet<u32>,
    /// Gear the build will wear.
    pub planned: HashSet<u32>,
    /// Item → the quests that ask for it, as (id, name).
    pub wanted_by: HashMap<u32, Vec<(u32, String)>>,
}

impl KeepFacts {
    pub fn new(atlas: &Atlas, catalog: &Catalog, planned: HashSet<u32>) -> Self {
        let in_recipes = catalog.recipe_list().iter().flat_map(|r| r.ingredients.iter().map(|i| i.item_id)).collect();
        let mut wanted_by: HashMap<u32, Vec<(u32, String)>> = HashMap::new();
        for quest in atlas.quests.values() {
            for need in &quest.items {
                wanted_by.entry(need.item_id).or_default().push((quest.id, quest.name.clone()));
            }
        }
        Self { in_recipes, planned, wanted_by }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    /// A made-up picture: a ring of one colour with a dot of another, `w` by `h`, the rest clear.
    fn picture(w: u32, h: u32, ring: [u8; 3], dot: [u8; 3]) -> RgbaImage {
        RgbaImage::from_fn(w, h, |x, y| {
            let edge = x == 2 || y == 1 || x == w - 3 || y == h - 2;
            let middle = x.abs_diff(w / 2) <= 1 && y.abs_diff(h / 2) <= 1;
            match (edge, middle) {
                (true, _) => Rgba([ring[0], ring[1], ring[2], 255]),
                (_, true) => Rgba([dot[0], dot[1], dot[2], 255]),
                _ => Rgba([0, 0, 0, 0]),
            }
        })
    }

    /// A slot with `picture` laid at `(dx, dy)`, its bottom-left corner hidden as a count would be.
    fn slot(picture: &RgbaImage, dx: u32, dy: u32) -> SlotPicture {
        let mut out = SlotPicture { pixels: vec![GROUND; (SLOT_SIZE * SLOT_SIZE) as usize], hidden: vec![false; (SLOT_SIZE * SLOT_SIZE) as usize] };
        for (x, y, px) in picture.enumerate_pixels() {
            if px.0[3] > 0 && x + dx < SLOT_SIZE && y + dy < SLOT_SIZE {
                out.pixels[((y + dy) * SLOT_SIZE + x + dx) as usize] = [px.0[0], px.0[1], px.0[2]];
            }
        }
        for y in 22..SLOT_SIZE {
            for x in 0..12 {
                out.hidden[(y * SLOT_SIZE + x) as usize] = true;
                out.pixels[(y * SLOT_SIZE + x) as usize] = [255, 255, 255];
            }
        }
        out
    }

    #[test]
    fn a_picture_is_named_wherever_it_stands_in_the_slot() {
        let red = picture(27, 30, [200, 40, 40], [250, 250, 90]);
        let blue = picture(27, 30, [40, 40, 200], [250, 250, 90]);
        let small = picture(20, 18, [200, 40, 40], [250, 250, 90]);
        let pictures = [(1, &red), (2, &blue), (3, &small)];
        for (dx, dy) in [(2, 1), (0, 0), (4, 1)] {
            let named = name(&slot(&red, dx, dy), &pictures).expect("named");
            assert_eq!(named.ids, [1], "at {dx},{dy}");
            assert!(named.score < 1.0);
        }
        assert_eq!(name(&slot(&small, 6, 8), &pictures).unwrap().ids, [3]);
    }

    #[test]
    fn items_drawn_alike_come_back_together_and_strangers_not_at_all() {
        let hat = picture(30, 29, [230, 200, 120], [120, 80, 40]);
        let same = hat.clone();
        let other = picture(30, 29, [90, 160, 90], [120, 80, 40]);
        let named = name(&slot(&hat, 0, 2), &[(10, &hat), (11, &same)]).unwrap();
        assert_eq!(named.ids, [10, 11]);
        assert_eq!(name(&slot(&hat, 0, 2), &[(12, &other)]), None);
    }
}
