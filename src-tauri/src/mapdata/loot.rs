//! Naming what the Pickup Log says was picked up, from the item data.
//!
//! The log words an item as the game names it, with the inventory tab it went to. OCR slips a
//! letter now and then, so a name is matched to the closest in the data, but only when it is
//! clearly closer to that item than to any other of the tab's: a name that could be two items is
//! left unnamed, and an unnamed item is never tallied. A long name the game cut short with `...`
//! is matched by its start.

use super::bag;
use super::drops::item_key;
use super::osms::ItemRecord;
use crate::build::catalog::Catalog;
use crate::readers::inventory::Tab;

/// The least likeness, 0 to 1, for a misread name to be an item's.
const LIKE: f64 = 0.8;
/// How much closer than the next item's name it must be.
const LEAD: f64 = 0.08;
/// A cut-short name shorter than this, in letters, says too little.
const SHORTEST_START: usize = 8;

/// The item a name read off the Pickup Log is. `tab` narrows it when it was read.
pub fn name_item<'a>(catalog: &'a Catalog, read: &str, tab: Option<Tab>, cut_short: bool) -> Option<&'a ItemRecord> {
    let key = item_key(read);
    if key.is_empty() {
        return None;
    }
    let in_tab = |item: &&ItemRecord| tab.is_none_or(|tab| bag::kinds(tab).contains(&item.kind));
    // An exact name stands whatever the tab read as.
    if let Some(item) = catalog.items().iter().filter(in_tab).find(|i| item_key(&i.name) == key) {
        return Some(item);
    }
    if !cut_short && let Some(item) = catalog.items().iter().find(|i| item_key(&i.name) == key) {
        return Some(item);
    }
    let candidates: Vec<&ItemRecord> = catalog.items().iter().filter(in_tab).collect();
    if cut_short {
        return by_start(&candidates, &key);
    }
    closest(&candidates, &key)
}

/// The one name in `items` that starts with `start`, a letter or so off. Items that share the
/// name count as one.
fn by_start<'a>(items: &[&'a ItemRecord], start: &str) -> Option<&'a ItemRecord> {
    if start.chars().filter(char::is_ascii_alphabetic).count() < SHORTEST_START {
        return None;
    }
    let fits: Vec<&ItemRecord> = items
        .iter()
        .copied()
        .filter(|item| {
            let name = item_key(&item.name);
            name.len() > start.len() && strsim::normalized_levenshtein(&name[..start.len()], start) >= LIKE + 0.1
        })
        .collect();
    one_name(fits)
}

/// The item whose name is like `key` and clearly closer to it than any other name.
fn closest<'a>(items: &[&'a ItemRecord], key: &str) -> Option<&'a ItemRecord> {
    let mut scored: Vec<(f64, &ItemRecord)> =
        items.iter().map(|item| (strsim::normalized_levenshtein(&item_key(&item.name), key), *item)).collect();
    scored.sort_by(|a, b| b.0.total_cmp(&a.0));
    let (best, item) = *scored.first()?;
    if best < LIKE {
        return None;
    }
    let runner_up = scored.iter().find(|(_, other)| other.name != item.name).map_or(0.0, |(score, _)| *score);
    (best - runner_up >= LEAD).then_some(item)
}

/// The item when every one of `items` has the same name, else none.
fn one_name(items: Vec<&ItemRecord>) -> Option<&ItemRecord> {
    let first = *items.first()?;
    items.iter().all(|i| i.name == first.name).then_some(first)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::catalog::tests::item;
    use crate::mapdata::osms::ItemKind;

    fn of(id: u32, name: &str, kind: ItemKind) -> ItemRecord {
        ItemRecord { kind, ..item(id, name, 0, &[]) }
    }

    fn catalog() -> Catalog {
        Catalog::new(
            Vec::new(),
            vec![
                of(1, "Snail Shell", ItemKind::Etc),
                of(2, "Blue Snail Shell", ItemKind::Etc),
                of(3, "Red Snail Shell", ItemKind::Etc),
                of(4, "Leaf", ItemKind::Etc),
                of(5, "Red Potion", ItemKind::Use),
                of(6, "Scroll for One-Handed Sword for ATT", ItemKind::Scroll),
                of(7, "Scroll for One-Handed Sword for Accuracy", ItemKind::Scroll),
                of(8, "Scroll for Two-Handed Axe for ATT", ItemKind::Scroll),
                of(9, "Leaf", ItemKind::Use),
                of(10, "Pig's Ribbon", ItemKind::Etc),
                of(11, "Pig's Ribbons", ItemKind::Etc),
            ],
        )
    }

    fn named(read: &str, tab: Option<Tab>, cut: bool) -> Option<u32> {
        name_item(&catalog(), read, tab, cut).map(|i| i.id)
    }

    #[test]
    fn exact_names_stand() {
        assert_eq!(named("Snail Shell", Some(Tab::Etc), false), Some(1));
        assert_eq!(named("Red Potion", Some(Tab::Etc), false), Some(5), "an exact name stands over a misread tab");
        assert_eq!(named("Leaf", Some(Tab::Use), false), Some(9), "the tab picks between two of a name");
    }

    #[test]
    fn a_letter_wrong_is_the_item_it_is_clearly_closest_to() {
        assert_eq!(named("Blue Snail Shel", Some(Tab::Etc), false), Some(2));
        assert_eq!(named("Red Potlon", Some(Tab::Use), false), Some(5));
    }

    #[test]
    fn a_name_that_could_be_two_items_is_left_unnamed() {
        assert_eq!(named("Bed Snail Shell", Some(Tab::Etc), false), Some(3), "far nearer Red than Blue");
        assert_eq!(named("Pig's Ribbonz", Some(Tab::Etc), false), None, "a letter from each of two names");
        assert_eq!(named("Mushroom Cap", Some(Tab::Etc), false), None, "not in the data");
    }

    #[test]
    fn a_cut_short_name_is_matched_by_its_start() {
        assert_eq!(named("Scroll for Two-Handed A", Some(Tab::Use), true), Some(8));
        assert_eq!(named("Scroll for One-Handed Sw", Some(Tab::Use), true), None, "two scrolls start so");
        assert_eq!(named("Red", Some(Tab::Etc), true), None, "too short");
    }
}
