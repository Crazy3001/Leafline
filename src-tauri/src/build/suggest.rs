//! Gear worth looking for at a level, going by the build itself: what its job can use and its
//! stats can wear, ranked by the stats it raises. It knows nothing of prices or what is easy to
//! get, and nobody has checked its picks in game, so it is a list to look through, not advice.

use std::collections::HashMap;

use serde::Serialize;

use super::catalog::{Catalog, usable_by};
use super::fit::ARMOUR;
use super::plan::{Build, Expansion, Stat, meets};
use super::rules::{Branch, Rules};
use crate::mapdata::osms::{ItemKind, ItemRecord};

/// Items shown for each slot.
const PICKS: usize = 3;
/// How much a point of each counts: attack most, then the stat the plan puts the rest of its AP
/// into, then stats it only raises to a target.
const ATTACK_WEIGHT: i64 = 3;
const MAIN_WEIGHT: i64 = 2;
const SIDE_WEIGHT: i64 = 1;
/// The data prices staff outfits and novelty items at a meso or less.
const NOVELTY_PRICE: i64 = 1;
/// Two-handed weapons have ids from 1400000 up.
const TWO_HANDED_BLOCK: u32 = 14;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GearPick {
    pub item: ItemRecord,
    /// Already one of the build's gear goals.
    pub goal: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SlotPicks {
    pub slot: String,
    pub picks: Vec<GearPick>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GearSuggestions {
    pub level: u32,
    /// What the ranking counted, the heaviest first.
    pub ranked_by: Vec<String>,
    /// Slots with nothing to suggest are left out.
    pub slots: Vec<SlotPicks>,
}

/// Whether a build can wear something at one level.
pub struct Wearer<'a> {
    build: &'a Build,
    expansion: &'a Expansion,
    rules: &'a Rules,
    level: u32,
    weapons: Vec<String>,
}

impl<'a> Wearer<'a> {
    pub fn new(build: &'a Build, expansion: &'a Expansion, rules: &'a Rules, catalog: &Catalog, level: u32) -> Self {
        Self { build, expansion, rules, level, weapons: catalog.weapons_of(build.plan.job) }
    }

    pub fn can_wear(&self, item: &ItemRecord) -> bool {
        item.kind == ItemKind::Equip && self.right_gender(item) && self.right_job(item) && self.strong_enough(item)
    }

    fn right_gender(&self, item: &ItemRecord) -> bool {
        match (&item.gender, &self.build.start.gender) {
            (Some(made_for), Some(gender)) => made_for == gender,
            _ => true,
        }
    }

    /// A Beginner wears only what anyone can. After that, gear for the job's class, and of
    /// weapons only those the build's job trains in.
    fn right_job(&self, item: &ItemRecord) -> bool {
        let branch = Branch::of_job(self.build.plan.job).filter(|_| self.level >= self.rules.first_job_level);
        let Some(branch) = branch else { return item.classes.is_empty() };
        let trained = self.weapons.is_empty() || item.weapon_type.as_ref().is_none_or(|w| self.weapons.contains(w));
        usable_by(item, branch) && trained
    }

    /// Whether every weapon for this job takes both hands, leaving none for a shield. The data has
    /// no field for it, but the game's item ids keep those weapons in one block.
    fn both_hands_full(&self, catalog: &Catalog) -> bool {
        let mut weapons = catalog.equipment().filter(|i| i.weapon_type.is_some() && self.right_job(i)).peekable();
        weapons.peek().is_some() && weapons.all(|i| i.id / 100_000 == TWO_HANDED_BLOCK)
    }

    /// Other gear worn by then counts, as it does for a hovered item, but not what this replaces.
    fn strong_enough(&self, item: &ItemRecord) -> bool {
        let Some(step) = self.expansion.step(self.level) else { return false };
        let others = step.worn.iter().filter(|w| w.slot != item.slot);
        let stats = others.fold(step.stats, |total, w| total.plus(w.bonus));
        meets(item, self.level, &stats, self.build.start.stats.is_some())
    }
}

/// The stats that count toward a pick and how much. With no stat goals, the job's main stat.
fn weighted_stats(build: &Build) -> Vec<(Stat, i64)> {
    let mut weights: Vec<(Stat, i64)> = Vec::new();
    for goal in &build.plan.stats {
        let weight = if goal.target.is_none() { MAIN_WEIGHT } else { SIDE_WEIGHT };
        match weights.iter_mut().find(|(stat, _)| *stat == goal.stat) {
            Some(known) => known.1 = known.1.max(weight),
            None => weights.push((goal.stat, weight)),
        }
    }
    if weights.is_empty() {
        weights.extend(Branch::of_job(build.plan.job).map(|branch| (branch.primary(), MAIN_WEIGHT)));
    }
    weights.sort_by_key(|(_, weight)| std::cmp::Reverse(*weight));
    weights
}

fn stat(item: &ItemRecord, label: &str) -> i64 {
    item.stats.iter().find(|s| s.label == label).map_or(0, |s| s.value)
}

/// Ranks what `build` can wear at `level`, slot by slot. Items at or under a meso are left out:
/// the data has no other mark for gear that players can't get.
pub fn suggest(build: &Build, expansion: &Expansion, rules: &Rules, catalog: &Catalog, level: u32) -> GearSuggestions {
    let wearer = Wearer::new(build, expansion, rules, catalog, level);
    let magician = Branch::of_job(build.plan.job) == Some(Branch::Magician);
    let (attack, defence) = if magician { ("Magic", "Magic Def.") } else { ("Attack", "Weapon Def.") };
    let weights = weighted_stats(build);
    let score = |item: &ItemRecord| {
        let stats: i64 = weights.iter().map(|(s, weight)| stat(item, s.label()) * weight).sum();
        (stat(item, attack) * ATTACK_WEIGHT + stats, stat(item, defence), item.req_level)
    };

    let mut slots = Vec::new();
    for slot in std::iter::once("Weapon").chain(ARMOUR) {
        if slot == "Shield" && wearer.both_hands_full(catalog) {
            continue;
        }
        let mut fits: Vec<&ItemRecord> = catalog
            .equipment()
            .filter(|i| i.slot == slot && i.price.is_none_or(|p| p > NOVELTY_PRICE) && wearer.can_wear(i))
            .collect();
        fits.sort_by(|a, b| score(b).cmp(&score(a)).then_with(|| a.name.cmp(&b.name)));
        // Each kind of weapon gets its turn, so one kind's strongest don't fill the list.
        let mut kinds: HashMap<&Option<String>, usize> = HashMap::new();
        fits.sort_by_cached_key(|i| {
            let turn = kinds.entry(&i.weapon_type).or_default();
            *turn += 1;
            i.weapon_type.as_ref().map_or(0, |_| *turn)
        });
        let picks: Vec<GearPick> = fits
            .into_iter()
            .take(PICKS)
            .map(|item| GearPick { item: item.clone(), goal: build.plan.gear.contains(&item.id) })
            .collect();
        if !picks.is_empty() {
            slots.push(SlotPicks { slot: slot.to_string(), picks });
        }
    }
    let ranked_by = std::iter::once(attack.to_string())
        .chain(weights.iter().map(|(s, _)| s.label().to_string()))
        .chain(std::iter::once(defence.to_string()))
        .collect();
    GearSuggestions { level, ranked_by, slots }
}

#[cfg(test)]
mod tests {
    use super::super::catalog::tests::{item, skill};
    use super::super::plan::{Plan, Start, StatGoal, Stats, expand};
    use super::*;
    use crate::mapdata::osms::ItemStat;

    fn with(mut item: ItemRecord, stats: &[(&str, i64)]) -> ItemRecord {
        item.stats = stats.iter().map(|(label, value)| ItemStat { label: (*label).into(), value: *value }).collect();
        item
    }

    fn in_slot(slot: &str, item: ItemRecord) -> ItemRecord {
        ItemRecord { slot: slot.into(), ..item }
    }

    fn weapon(id: u32, name: &str, kind: &str, level: u32, attack: i64) -> ItemRecord {
        let base = ItemRecord { weapon_type: Some(kind.into()), ..in_slot("Weapon", item(id, name, level, &["Thief"])) };
        with(base, &[("Attack", attack)])
    }

    /// A claw thief from Lv 20: all AP into LUK, with DEX raised to 60 first.
    fn thief() -> Build {
        Build {
            start: Start { level: 20, stats: Some(Stats { str: 4, dex: 40, int: 4, luk: 40 }), ..Start::default() },
            plan: Plan {
                job: 410,
                stats: vec![
                    StatGoal { stat: Stat::Dex, target: Some(60), from_level: None },
                    StatGoal { stat: Stat::Luk, target: None, from_level: None },
                ],
                plan_to: 40,
                ..Plan::default()
            },
            ..Build::default()
        }
    }

    fn catalog(items: Vec<ItemRecord>) -> Catalog {
        Catalog::new(vec![skill(1, "Claw Mastery", 410, 20, &[])], items)
    }

    fn at(build: &Build, catalog: &Catalog, level: u32) -> GearSuggestions {
        let rules = Rules::default();
        suggest(build, &expand(build, &rules, catalog), &rules, catalog, level)
    }

    fn names(picks: &GearSuggestions, slot: &str) -> Vec<String> {
        let found = picks.slots.iter().find(|s| s.slot == slot);
        found.into_iter().flat_map(|s| &s.picks).map(|p| p.item.name.clone()).collect()
    }

    #[test]
    fn ranks_by_the_stats_the_plan_raises() {
        let catalog = catalog(vec![
            with(item(1, "Plain Hat", 15, &[]), &[("Weapon Def.", 30)]),
            with(item(2, "Dex Hat", 15, &[]), &[("DEX", 3)]),
            with(item(3, "Luk Hat", 15, &[]), &[("LUK", 2)]),
            with(item(4, "Str Hat", 15, &[]), &[("STR", 9)]),
        ]);
        let picks = at(&thief(), &catalog, 20);
        assert_eq!(names(&picks, "Hat"), ["Luk Hat", "Dex Hat", "Plain Hat"], "LUK counts double, STR not at all");
        assert_eq!(picks.ranked_by, ["Attack", "LUK", "DEX", "Weapon Def."]);
    }

    #[test]
    fn leaves_out_what_the_build_cant_wear_yet() {
        let mut too_much_luk = with(item(1, "Big Hat", 15, &[]), &[("LUK", 9)]);
        too_much_luk.req_luk = 80;
        let catalog = catalog(vec![
            too_much_luk,
            with(item(2, "Late Hat", 30, &[]), &[("LUK", 9)]),
            with(item(3, "Wizard Hat", 15, &["Magician"]), &[("LUK", 9)]),
            with(item(4, "Early Hat", 15, &[]), &[("LUK", 1)]),
        ]);
        let build = thief();
        assert_eq!(names(&at(&build, &catalog, 20), "Hat"), ["Early Hat"]);
        // LUK takes 5 AP a level once DEX is at 60, from Lv 24, so it reaches 80 at Lv 32.
        assert_eq!(names(&at(&build, &catalog, 32), "Hat"), ["Late Hat", "Big Hat", "Early Hat"]);
    }

    #[test]
    fn weapons_are_the_jobs_own_each_kind_in_turn_and_attack_counts_most() {
        let catalog = catalog(vec![
            weapon(1, "Thorn Claw", "Claw", 15, 20),
            weapon(2, "Old Claw", "Claw", 15, 10),
            weapon(3, "Fern Dagger", "Dagger", 15, 40),
        ]);
        assert_eq!(names(&at(&thief(), &catalog, 20), "Weapon"), ["Thorn Claw", "Old Claw"]);
        let first_job = Build { plan: Plan { job: 400, ..thief().plan }, ..thief() };
        assert_eq!(names(&at(&first_job, &catalog, 20), "Weapon"), ["Fern Dagger", "Thorn Claw", "Old Claw"], "no Mastery yet");
    }

    #[test]
    fn weapons_fill_the_list_when_the_job_has_one_kind() {
        let catalog = catalog(vec![weapon(1, "Thorn Claw", "Claw", 15, 20), weapon(2, "Old Claw", "Claw", 15, 10)]);
        assert_eq!(names(&at(&thief(), &catalog, 20), "Weapon"), ["Thorn Claw", "Old Claw"]);
    }

    #[test]
    fn no_shields_for_a_job_whose_weapons_take_both_hands() {
        let shield = in_slot("Shield", item(5, "Pan Lid", 10, &[]));
        let catalog = catalog(vec![shield, weapon(1_470_001, "Thorn Claw", "Claw", 15, 20), weapon(1_330_001, "Fern Dagger", "Dagger", 15, 20)]);
        assert!(names(&at(&thief(), &catalog, 20), "Shield").is_empty());
        let first_job = Build { plan: Plan { job: 400, ..thief().plan }, ..thief() };
        assert_eq!(names(&at(&first_job, &catalog, 20), "Shield"), ["Pan Lid"], "a dagger leaves a hand free");
    }

    #[test]
    fn a_beginner_only_gets_gear_for_anyone() {
        let catalog = catalog(vec![item(1, "Fern Hat", 1, &[]), item(2, "Thief Hat", 1, &["Thief"])]);
        let build = Build { start: Start { level: 5, ..thief().start }, ..thief() };
        assert_eq!(names(&at(&build, &catalog, 5), "Hat"), ["Fern Hat"]);
        assert_eq!(names(&at(&build, &catalog, 10), "Hat"), ["Fern Hat", "Thief Hat"]);
    }

    #[test]
    fn gender_counts_once_the_player_has_given_it() {
        let made_for = |id, name, gender: &str| ItemRecord { gender: Some(gender.into()), ..item(id, name, 15, &[]) };
        let catalog = catalog(vec![made_for(1, "His Hat", "male"), made_for(2, "Her Hat", "female"), item(3, "Any Hat", 15, &[])]);
        assert_eq!(names(&at(&thief(), &catalog, 20), "Hat").len(), 3);
        let build = Build { start: Start { gender: Some("female".into()), ..thief().start }, ..thief() };
        assert_eq!(names(&at(&build, &catalog, 20), "Hat"), ["Any Hat", "Her Hat"]);
    }

    #[test]
    fn staff_and_novelty_gear_is_left_out() {
        let priced = |id, name, price| ItemRecord { price, ..item(id, name, 15, &[]) };
        let catalog = catalog(vec![priced(1, "Staff Hat", Some(1)), priced(2, "Shop Hat", Some(500)), priced(3, "Starter Hat", None)]);
        assert_eq!(names(&at(&thief(), &catalog, 20), "Hat"), ["Shop Hat", "Starter Hat"]);
    }

    #[test]
    fn marks_the_builds_own_goals_and_skips_empty_slots() {
        let catalog = catalog(vec![item(1, "Fern Hat", 15, &[]), item(2, "Moss Hat", 15, &[])]);
        let mut build = thief();
        build.plan.gear = vec![2];
        let picks = at(&build, &catalog, 20);
        assert_eq!(picks.slots.len(), 1);
        let goals: Vec<_> = picks.slots[0].picks.iter().map(|p| (p.item.name.as_str(), p.goal)).collect();
        assert_eq!(goals, [("Fern Hat", false), ("Moss Hat", true)]);
    }

    #[test]
    fn with_no_stat_goals_the_jobs_main_stat_counts() {
        let build = Build { plan: Plan { job: 200, ..Plan::default() }, ..Build::default() };
        assert_eq!(weighted_stats(&build), [(Stat::Int, MAIN_WEIGHT)]);
    }
}
