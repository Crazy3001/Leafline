//! The character's gear slot by slot: what they wear, the gear finder's pick when it beats that,
//! and what comes next: the plan's next goal for the slot, or else the first better piece the
//! plan can wear at a later level. It reads like the game's equip window, so a player can see at
//! a glance what to look for now and what to look forward to.

use serde::Serialize;

use super::catalog::{Catalog, novelty};
use super::fit::ARMOUR;
use super::plan::{Build, Expansion, GearCheck, MAX_LEVEL, Piece};
use super::rules::Rules;
use super::suggest::{Scorer, Wearer, suggest};
use crate::mapdata::osms::ItemRecord;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SlotGear {
    /// As the item data names it.
    pub slot: String,
    pub wearing: Option<Piece>,
    /// The best the character can wear now, when it beats what they wear and isn't a planned `next`.
    pub suggested: Option<ItemRecord>,
    /// The plan's next goal for this slot, or else the first piece that beats what they wear and
    /// `suggested`, at the level the plan can wear it.
    pub next: Option<GearCheck>,
    /// `next` is one of the plan's goals, not the gear finder's.
    pub planned: bool,
    /// More goals for this slot after a planned `next`.
    pub later: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Outfit {
    pub level: u32,
    /// In the equip window's order. An overall or a shield only when worn or planned.
    pub slots: Vec<SlotGear>,
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
    let end = expansion.steps.last().map_or(level, |s| s.level);
    let later_wearers: Vec<(u32, Wearer)> =
        (level + 1..=end).map(|at| (at, Wearer::new(build, expansion, rules, catalog, at))).collect();
    let trained = catalog.weapons_of_branch(build.plan.job);

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
        let mut next = ahead.get(at).map(|g| (*g).clone());
        let planned = next.is_some();
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

        // With no goal for the slot, the first piece a later level can wear that beats both what
        // is worn and the suggestion.
        if next.is_none() {
            let floor = [worn_score, suggested.as_ref().map(|s| scorer.of(s))].into_iter().flatten().max();
            next = coming_up(slot, floor, &later_wearers, &trained, &scorer, catalog)
                .map(|(item, at)| GearCheck { item: item.clone(), level: Some(at), usable: true });
        }

        // An overall, or a shield, is only worth a row when it is worn or planned.
        let optional = slot == "Overall" || slot == "Shield";
        if optional && worn.is_none() && !planned {
            continue;
        }
        outfit.slots.push(SlotGear { slot: slot.to_string(), wearing: worn, suggested, next, planned, later });
    }
    outfit
}

/// The first piece for `slot` that a later level can wear and that scores above `floor`, with
/// that level. Of several at the same level, the best. Weapons only of the kinds the job trains in.
fn coming_up<'a>(
    slot: &str,
    floor: Option<(i64, i64)>,
    wearers: &[(u32, Wearer)],
    trained: &[String],
    scorer: &Scorer,
    catalog: &'a Catalog,
) -> Option<(&'a ItemRecord, u32)> {
    let better: Vec<&ItemRecord> = catalog
        .equipment()
        .filter(|i| i.slot == slot && !novelty(i))
        .filter(|i| trained.is_empty() || i.weapon_type.as_ref().is_none_or(|w| trained.contains(w)))
        .filter(|i| floor.is_none_or(|f| scorer.of(i) > f))
        .collect();
    wearers.iter().find_map(|(at, wearer)| {
        let wearable = better.iter().filter(|i| wearer.can_wear(i));
        wearable.max_by_key(|i| (scorer.of(i), std::cmp::Reverse(i.req_level))).map(|i| (*i, *at))
    })
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
        assert!(weapon.planned);
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
    fn every_slot_has_a_row_but_an_overall_or_shield_only_when_worn_or_planned() {
        let catalog = catalog();
        let outfit = at(&bandit(10, vec![], vec![]), &catalog, 10);
        let rows: Vec<&str> = outfit.slots.iter().map(|s| s.slot.as_str()).collect();
        assert_eq!(rows, ["Weapon", "Hat", "Top", "Bottom", "Shoes", "Glove", "Cape", "Earring"]);
        assert_eq!(slot(&outfit, "Glove").suggested.as_ref().map(|i| i.name.as_str()), Some("Work Gloves"));
    }

    #[test]
    fn with_no_goal_next_is_the_first_better_piece_a_later_level_can_wear() {
        let catalog = catalog();
        // Wearing the Lv 10 dagger, with no goals: the next better dagger comes at Lv 15.
        let build = bandit(10, vec![], vec![worn(1, &catalog)]);
        let weapon = slot(&at(&build, &catalog, 10), "Weapon").clone();
        assert_eq!(weapon.suggested, None, "nothing better to wear yet");
        assert_eq!(weapon.next.as_ref().map(|n| (n.item.name.as_str(), n.level)), Some(("Field Dagger", Some(15))));
        assert!(!weapon.planned);

        // At Lv 22 the Coconut Knife can be worn: it is suggested, and Next is the one after it.
        let weapon = slot(&at(&build, &catalog, 22), "Weapon").clone();
        assert_eq!(weapon.suggested.map(|i| i.name), Some("Coconut Knife".into()));
        assert_eq!(weapon.next.map(|n| (n.item.name, n.level)), Some(("Iron Dagger".into(), Some(25))));
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
