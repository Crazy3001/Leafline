//! The character's gear slot by slot: what they wear, the plan's next goal for the slot, and the
//! gear finder's pick when it beats what they wear. It reads like the game's equip window, so a
//! player can see at a glance what to look for next.

use serde::Serialize;

use super::catalog::Catalog;
use super::fit::ARMOUR;
use super::plan::{Build, Expansion, GearCheck, MAX_LEVEL, Piece};
use super::rules::Rules;
use super::suggest::{Scorer, suggest};
use crate::mapdata::osms::ItemRecord;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SlotGear {
    /// As the item data names it.
    pub slot: String,
    pub wearing: Option<Piece>,
    /// The plan's next goal for this slot.
    pub next: Option<GearCheck>,
    /// More goals for this slot after `next`.
    pub later: usize,
    /// The best the character can wear now, when it beats what they wear and isn't `next`.
    pub suggested: Option<ItemRecord>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Outfit {
    pub level: u32,
    /// Slots with something to show, in the equip window's order.
    pub slots: Vec<SlotGear>,
    /// Slots with nothing worn, planned or suggested.
    pub empty: Vec<String>,
}

/// The slots in order. An overall takes the top's and bottom's place.
fn slot_order() -> impl Iterator<Item = &'static str> {
    std::iter::once("Weapon").chain(ARMOUR)
}

/// `build`'s gear at `level`, slot by slot.
pub fn outfit(build: &Build, expansion: &Expansion, rules: &Rules, catalog: &Catalog, level: u32) -> Outfit {
    let level = level.clamp(1, MAX_LEVEL);
    let picks = suggest(build, expansion, rules, catalog, level);
    let scorer = Scorer::new(build);
    let wearing = |slot: &str| build.start.pieces.iter().rev().find(|p| p.slot(catalog) == slot);
    let overall = wearing("Overall").is_some();

    let mut outfit = Outfit { level, ..Outfit::default() };
    for slot in slot_order() {
        if overall && (slot == "Top" || slot == "Bottom") {
            continue;
        }
        let worn = wearing(slot).cloned();
        let worn_item = worn.as_ref().and_then(|p| p.item);
        let worn_level = worn.as_ref().map_or(0, |p| p.req_level(catalog));
        let worn_score = worn.as_ref().map(|p| scorer.score(&p.data_stats(catalog)));

        // Goals still ahead: not what is worn, nor older than it. Of those the plan can wear by
        // now, the best; else the first to come.
        let mut ahead: Vec<&GearCheck> = expansion
            .gear
            .iter()
            .filter(|g| g.item.slot == slot && g.usable && Some(g.item.id) != worn_item && g.item.req_level > worn_level)
            .collect();
        ahead.sort_by_key(|g| (g.level.unwrap_or(u32::MAX), g.item.req_level));
        let ready = ahead.iter().take_while(|g| g.level.is_some_and(|at| at <= level)).count();
        let at = ready.saturating_sub(1);
        let next = ahead.get(at).map(|g| (*g).clone());
        let later = ahead.len().saturating_sub(at + 1);

        let suggested = picks
            .slots
            .iter()
            .find(|s| s.slot == slot)
            .and_then(|s| s.picks.first())
            .map(|p| &p.item)
            .filter(|item| Some(item.id) != worn_item && next.as_ref().is_none_or(|n| n.item.id != item.id))
            .filter(|item| worn_score.is_none_or(|have| scorer.of(item) > have))
            .cloned();

        // An overall, or a shield beside a two-handed weapon, is only worth a row when it is planned or worn.
        let optional = slot == "Overall" || slot == "Shield";
        let suggested = if optional && worn.is_none() && next.is_none() { None } else { suggested };
        if worn.is_none() && next.is_none() && suggested.is_none() {
            if !optional {
                outfit.empty.push(slot.to_string());
            }
            continue;
        }
        outfit.slots.push(SlotGear { slot: slot.to_string(), wearing: worn, next, later, suggested });
    }
    outfit
}

#[cfg(test)]
mod tests {
    use super::super::catalog::tests::{item, skill};
    use super::super::plan::{Plan, PieceStat, Start, StatGoal, Stat, Stats, expand};
    use super::*;
    use crate::mapdata::osms::ItemStat;

    fn with(mut item: ItemRecord, slot: &str, stats: &[(&str, i64)]) -> ItemRecord {
        item.slot = slot.into();
        item.stats = stats.iter().map(|(label, value)| ItemStat { label: (*label).into(), value: *value }).collect();
        item
    }

    fn dagger(id: u32, name: &str, level: u32, attack: i64) -> ItemRecord {
        let mut d = with(item(id, name, level, &["Thief"]), "Weapon", &[("Attack", attack)]);
        d.weapon_type = Some("Dagger".into());
        d
    }

    fn catalog() -> Catalog {
        Catalog::new(
            vec![skill(1, "Double Stab", 400, 20, &[])],
            vec![
                dagger(1, "Old Dagger", 10, 17),
                dagger(2, "Field Dagger", 15, 25),
                dagger(3, "Coconut Knife", 20, 30),
                dagger(4, "Iron Dagger", 25, 35),
                with(item(10, "Plain Hat", 10, &[]), "Hat", &[("Weapon Def.", 5)]),
                with(item(11, "Luk Hat", 10, &[]), "Hat", &[("LUK", 3)]),
                with(item(20, "Work Gloves", 10, &[]), "Glove", &[("Weapon Def.", 2)]),
            ],
        )
    }

    fn bandit(level: u32, gear: Vec<u32>, pieces: Vec<Piece>) -> Build {
        Build {
            start: Start { level, stats: Some(Stats { str: 4, dex: 25, int: 4, luk: 40 }), pieces, ..Start::default() },
            plan: Plan {
                job: 400,
                stats: vec![StatGoal { stat: Stat::Luk, target: None, from_level: None }],
                gear,
                plan_to: 40,
                ..Plan::default()
            },
            ..Build::default()
        }
    }

    fn worn(id: u32, catalog: &Catalog) -> Piece {
        Piece { id: i64::from(id), ..Piece::of_item(catalog.item(id).unwrap()) }
    }

    fn at(build: &Build, catalog: &Catalog, level: u32) -> Outfit {
        let rules = Rules::default();
        outfit(build, &expand(build, &rules, catalog), &rules, catalog, level)
    }

    fn slot<'a>(outfit: &'a Outfit, name: &str) -> &'a SlotGear {
        outfit.slots.iter().find(|s| s.slot == name).unwrap_or_else(|| panic!("no {name} row"))
    }

    #[test]
    fn the_next_goal_is_the_first_after_what_is_worn() {
        let catalog = catalog();
        let build = bandit(10, vec![2, 3, 4], vec![worn(1, &catalog)]);
        let outfit = at(&build, &catalog, 10);
        let weapon = slot(&outfit, "Weapon");
        assert_eq!(weapon.wearing.as_ref().map(|p| p.name.as_str()), Some("Old Dagger"));
        assert_eq!(weapon.next.as_ref().map(|n| n.item.name.as_str()), Some("Field Dagger"));
        assert_eq!(weapon.later, 2);
    }

    #[test]
    fn of_goals_already_in_reach_the_best_is_next() {
        let catalog = catalog();
        let build = bandit(22, vec![2, 3, 4], vec![worn(1, &catalog)]);
        let weapon = slot(&at(&build, &catalog, 22), "Weapon").clone();
        assert_eq!(weapon.next.map(|n| n.item.name), Some("Coconut Knife".into()));
        assert_eq!(weapon.later, 1);
    }

    #[test]
    fn a_suggestion_shows_only_when_it_beats_what_is_worn() {
        let catalog = catalog();
        let mut hat = worn(10, &catalog);
        let build = bandit(10, vec![], vec![hat.clone()]);
        let outfit = at(&build, &catalog, 10);
        assert_eq!(slot(&outfit, "Hat").suggested.as_ref().map(|i| i.name.as_str()), Some("Luk Hat"));

        // Scrolled to 4 LUK, the worn hat is as good: nothing to suggest.
        hat.stats.push(PieceStat { label: "LUK".into(), value: 4 });
        let build = bandit(10, vec![], vec![hat]);
        assert_eq!(slot(&at(&build, &catalog, 10), "Hat").suggested, None);
    }

    #[test]
    fn empty_slots_with_nothing_to_say_are_listed_apart() {
        let catalog = catalog();
        let build = bandit(10, vec![], vec![]);
        let outfit = at(&build, &catalog, 10);
        assert_eq!(slot(&outfit, "Glove").suggested.as_ref().map(|i| i.name.as_str()), Some("Work Gloves"));
        assert!(outfit.empty.contains(&"Cape".to_string()));
        assert!(!outfit.empty.contains(&"Overall".to_string()), "an overall is optional");
    }

    #[test]
    fn worn_gear_is_not_a_goal_to_look_for() {
        let catalog = catalog();
        let build = bandit(10, vec![1, 2], vec![worn(1, &catalog)]);
        let e = expand(&build, &Rules::default(), &catalog);
        assert!(e.gear.iter().all(|g| g.item.id != 1), "what is worn isn't new gear");
        assert!(e.steps.iter().all(|s| !s.gear.contains(&"Old Dagger".to_string())));
    }

    #[test]
    fn a_piece_from_the_data_reads_like_a_tooltip() {
        let catalog = catalog();
        let piece = Piece::of_item(catalog.item(2).unwrap());
        assert_eq!(piece.stats, [PieceStat { label: "Weapon Attack".into(), value: 25 }]);
        assert_eq!(piece.data_stats(&catalog), [("Attack".to_string(), 25)]);
        assert_eq!(piece.slot(&catalog), "Weapon");
        assert_eq!(piece.item_type.as_deref(), Some("Dagger"));
    }
}
