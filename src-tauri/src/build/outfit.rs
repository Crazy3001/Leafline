//! The character's gear slot by slot: what they wear, the best they can wear now when it beats
//! that, and the next step up, from one path per slot that climbs a tier at a time to the plan's
//! end (see [`gear_paths`]). It reads like the game's equip window, so a player can see at a
//! glance what to look for now and what to look forward to.

use serde::Serialize;

use super::catalog::{Catalog, novelty};
use super::fit::ARMOUR;
use super::plan::{Build, Expansion, GearCheck, MAX_LEVEL, Piece, Stats, piece_bonus};
use super::rules::Rules;
use super::suggest::{Scorer, Wearer};
use crate::mapdata::osms::ItemRecord;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SlotGear {
    /// As the item data names it.
    pub slot: String,
    pub wearing: Option<Piece>,
    /// The best the character can wear now, when it beats what they wear: a goal of the plan's
    /// in reach, or the gear finder's pick when that scores higher.
    pub suggested: Option<ItemRecord>,
    /// `suggested` is one of the plan's goals.
    pub suggested_planned: bool,
    /// The next step up, which only a later level can wear: the plan's next goal for this slot, or
    /// else the first piece that beats what they wear and `suggested`, at the level the plan can wear it.
    pub next: Option<GearCheck>,
    /// `next` is one of the plan's goals, not the gear finder's.
    pub planned: bool,
    /// More goals for this slot after a planned `next`.
    pub later: usize,
    /// Whether the character, as they are now, meets each piece's level and stat requirements.
    pub wearing_fit: Option<Fit>,
    pub suggested_fit: Option<Fit>,
    pub next_fit: Option<Fit>,
}

/// Whether the character meets an item's requirements now, and what falls short.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Fit {
    pub ok: bool,
    /// Each requirement not met, such as `DEX 35 (you have 28)`.
    pub short: Vec<String>,
}

/// `item`'s level and stat requirements against `level` and, when known, `stats`.
fn fit(item: &ItemRecord, level: u32, stats: Option<Stats>) -> Fit {
    let mut short = Vec::new();
    if level < item.req_level {
        short.push(format!("Lv {} (you're {level})", item.req_level));
    }
    if let Some(have) = stats {
        for (name, need, got) in
            [("STR", item.req_str, have.str), ("DEX", item.req_dex, have.dex), ("INT", item.req_int, have.int), ("LUK", item.req_luk, have.luk)]
        {
            if got < need {
                short.push(format!("{name} {need} (you have {got})"));
            }
        }
    }
    Fit { ok: short.is_empty(), short }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Outfit {
    pub level: u32,
    /// In the equip window's order. An overall or a shield only when worn or planned.
    pub slots: Vec<SlotGear>,
    /// Each slot's way up from this level, which `slots` takes its suggestion and next step from.
    pub paths: Vec<GearPath>,
}

/// The slots in order. An overall takes the top's and bottom's place.
fn slot_order() -> impl Iterator<Item = &'static str> {
    std::iter::once("Weapon").chain(ARMOUR)
}

/// What is worn in `slot`, as the data's item when its name matches one, with the level it needs.
fn worn_in(build: &Build, catalog: &Catalog, slot: &str) -> Option<Piece> {
    build.start.pieces.iter().rev().find(|p| p.slot(catalog) == slot).cloned().map(|mut p| {
        // A piece saved from a noisy read (`Brown Hard Leather Pants ossbow`), or never matched,
        // is the data's item once its name does match; the name shown is the item's own.
        if let Some(item) = p.item.and_then(|id| catalog.item(id)).or_else(|| catalog.item_like(&p.name, p.level)) {
            p.item = Some(item.id);
            p.name = item.name.clone();
        }
        if p.level.is_none() && p.item.is_some_and(|id| catalog.item(id).is_some()) {
            p.level = Some(p.req_level(catalog));
        }
        p
    })
}

/// `build`'s gear at `level`, slot by slot.
pub fn outfit(build: &Build, expansion: &Expansion, rules: &Rules, catalog: &Catalog, level: u32) -> Outfit {
    let level = level.clamp(1, MAX_LEVEL);
    // The character's stats now: as last read, or the plan's for a level reached since. Gear worn
    // counts toward a piece's requirements, as in the game, what the piece would replace included.
    let base = if level <= build.start.level { build.start.stats } else { expansion.step(level).map(|s| s.stats).or(build.start.stats) };
    let known = build.start.stats.is_some();
    let worn_bonus: Vec<(String, Stats)> = build.start.pieces.iter().map(|p| (p.slot(catalog), piece_bonus(&p.data_stats(catalog)))).collect();
    let fits = |item: &ItemRecord| {
        let stats = known.then(|| worn_bonus.iter().fold(base.unwrap_or_default(), |total, (_, b)| total.plus(*b)));
        fit(item, level, stats)
    };

    // One list per slot, which the planner's Gear tab and its level by level show too.
    let paths = gear_paths(build, expansion, rules, catalog, level);
    let mut outfit = Outfit { level, ..Outfit::default() };
    for path in &paths {
        let worn = worn_in(build, catalog, &path.slot);
        let suggested = path.now.as_ref().map(|s| s.item.clone());
        let suggested_planned = path.now.as_ref().is_some_and(|s| s.planned);
        let next = path.steps.first().map(|s| GearCheck { item: s.item.clone(), level: Some(s.level), usable: true });
        let planned = path.steps.first().is_some_and(|s| s.planned);
        let later = path.steps.len().saturating_sub(1);
        let wearing_fit = worn.as_ref().map(|p| match p.item.and_then(|id| catalog.item(id)) {
            Some(item) => fits(item),
            None => {
                let need = p.level.unwrap_or(0);
                let short: Vec<String> = (level < need).then(|| format!("Lv {need} (you're {level})")).into_iter().collect();
                Fit { ok: short.is_empty(), short }
            }
        });
        let suggested_fit = suggested.as_ref().map(fits);
        let next_fit = next.as_ref().map(|n| fits(&n.item));
        outfit.slots.push(SlotGear {
            slot: path.slot.clone(),
            wearing: worn,
            suggested,
            suggested_planned,
            next,
            planned,
            later,
            wearing_fit,
            suggested_fit,
            next_fit,
        });
    }
    outfit.paths = paths;
    outfit
}

/// One piece on a slot's way up, at the first level the plan can wear it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GearStep {
    pub item: ItemRecord,
    pub level: u32,
    /// One of the plan's own goals, not the data's pick.
    pub planned: bool,
}

/// A slot's gear from now to the end of the plan: at most one piece to get now, then one piece for
/// each higher tier of the data the plan can wear, each better than the last.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GearPath {
    pub slot: String,
    /// The best piece wearable now, when it beats what is worn.
    pub now: Option<GearStep>,
    /// Each tier after it, in level order.
    pub steps: Vec<GearStep>,
    /// The plan's goals for the slot the path passes over: outgrown, or beaten by a piece of the
    /// same tier or a higher one that comes no later. Goals out of reach or for another job are in
    /// the expansion's `gear`.
    pub passed: Vec<ItemRecord>,
}

/// Each slot's way up from `from` to the plan's end. A piece joins when the plan can first wear
/// it, if it is of a higher tier (needs a higher level) than the last one and scores above it.
/// Within a tier the plan's own goal wins over the data's pick, when it too beats the last piece.
/// What is worn is never offered again. An overall or a shield has a path only when worn or
/// planned, and then only of the plan's goals; a worn overall leaves the top and bottom out.
pub fn gear_paths(build: &Build, expansion: &Expansion, rules: &Rules, catalog: &Catalog, from: u32) -> Vec<GearPath> {
    let from = from.clamp(1, MAX_LEVEL);
    let end = expansion.steps.last().map_or(from, |s| s.level).max(from);
    let wearers: Vec<(u32, Wearer)> = (from..=end).map(|at| (at, Wearer::new(build, expansion, rules, catalog, at))).collect();
    let scorer = Scorer::new(build);
    let trained = catalog.weapons_of_branch(build.plan.job);
    let overall = worn_in(build, catalog, "Overall").is_some();

    let mut paths = Vec::new();
    for slot in slot_order() {
        if overall && (slot == "Top" || slot == "Bottom") {
            continue;
        }
        let goals: Vec<&ItemRecord> = expansion.gear.iter().filter(|g| g.usable && g.item.slot == slot).map(|g| &g.item).collect();
        let worn = worn_in(build, catalog, slot);
        let optional = slot == "Overall" || slot == "Shield";
        if optional && worn.is_none() && goals.is_empty() {
            continue;
        }
        let is_worn = |item: &ItemRecord| worn.as_ref().is_some_and(|p| p.is_copy_of(item, catalog));
        let is_goal = |item: &ItemRecord| goals.iter().any(|g| g.id == item.id);
        let mut candidates: Vec<&ItemRecord> = goals.clone();
        if !optional {
            candidates.extend(
                catalog
                    .equipment()
                    .filter(|i| i.slot == slot && !novelty(i) && !is_goal(i))
                    .filter(|i| trained.is_empty() || i.weapon_type.as_ref().is_none_or(|w| trained.contains(w))),
            );
        }
        candidates.retain(|i| !is_worn(i));

        // What the next piece has to beat, and the tier it has to pass: a piece of the worn one's
        // own tier may still beat it, as Green Jack Boots do Brown ones; after that, only higher.
        let mut floor = worn.as_ref().map(|p| scorer.score(&p.data_stats(catalog)));
        let mut tier = worn.as_ref().map(|p| p.req_level(catalog));
        let mut first = true;
        let mut chosen: Vec<GearStep> = Vec::new();
        for (at, wearer) in &wearers {
            let fitting: Vec<&ItemRecord> = candidates
                .iter()
                .copied()
                .filter(|i| {
                    wearer.can_wear(i)
                        && floor.is_none_or(|f| scorer.of(i) > f)
                        && tier.is_none_or(|t| if first { i.req_level >= t } else { i.req_level > t })
                })
                .collect();
            let rank = |i: &&ItemRecord| (scorer.of(i), std::cmp::Reverse(i.req_level));
            let Some(&best) = fitting.iter().max_by_key(|i| rank(i)) else { continue };
            let pick = fitting
                .iter()
                .filter(|i| is_goal(i) && i.req_level == best.req_level)
                .max_by_key(|i| rank(i))
                .copied()
                .unwrap_or(best);
            floor = Some(scorer.of(pick));
            tier = Some(pick.req_level);
            first = false;
            chosen.push(GearStep { item: pick.clone(), level: *at, planned: is_goal(pick) });
        }

        let now = chosen.first().filter(|s| s.level == from).cloned();
        let steps: Vec<GearStep> = chosen.iter().skip(usize::from(now.is_some())).cloned().collect();
        let reached = |item: &ItemRecord| expansion.gear.iter().any(|g| g.item.id == item.id && g.level.is_some());
        let passed = goals
            .iter()
            .filter(|g| reached(g) && !chosen.iter().any(|s| s.item.id == g.id))
            .map(|g| (*g).clone())
            .collect();
        paths.push(GearPath { slot: slot.to_string(), now, steps, passed });
    }
    paths
}

/// How far apart the levels a filled-in path looks at are.
const PATH_STEP: u32 = 10;

/// An upgrade path for each slot with no goals, as item ids in level order: starting from what is
/// worn, at `level` and then every tenth level to the plan's end, the best piece the plan can wear
/// by then, whenever it beats the one before. An overall or a shield is left alone.
pub fn fill_paths(build: &Build, expansion: &Expansion, rules: &Rules, catalog: &Catalog, level: u32) -> Vec<(String, Vec<u32>)> {
    let level = level.clamp(1, MAX_LEVEL);
    let end = expansion.steps.last().map_or(level, |s| s.level).max(level);
    let mut looks: Vec<u32> = std::iter::once(level).chain((level / PATH_STEP + 1..).map(|n| n * PATH_STEP).take_while(|&at| at < end)).collect();
    if looks.last() != Some(&end) {
        looks.push(end);
    }
    let wearers: Vec<(u32, Wearer)> = looks.iter().map(|&at| (at, Wearer::new(build, expansion, rules, catalog, at))).collect();
    let scorer = Scorer::new(build);
    let trained = catalog.weapons_of_branch(build.plan.job);

    let mut paths = Vec::new();
    for slot in slot_order().filter(|s| *s != "Overall" && *s != "Shield") {
        if expansion.gear.iter().any(|g| g.item.slot == slot) {
            continue;
        }
        let worn = build.start.pieces.iter().rev().find(|p| p.slot(catalog) == slot);
        let mut floor = worn.map(|p| scorer.score(&p.data_stats(catalog)));
        let candidates: Vec<&ItemRecord> = catalog
            .equipment()
            .filter(|i| i.slot == slot && !novelty(i) && !worn.is_some_and(|p| p.is_copy_of(i, catalog)))
            .filter(|i| trained.is_empty() || i.weapon_type.as_ref().is_none_or(|w| trained.contains(w)))
            .collect();
        let mut path = Vec::new();
        for (_, wearer) in &wearers {
            let best = candidates
                .iter()
                .filter(|i| floor.is_none_or(|f| scorer.of(i) > f) && wearer.can_wear(i))
                .max_by_key(|i| (scorer.of(i), std::cmp::Reverse(i.req_level)));
            if let Some(item) = best {
                path.push(item.id);
                floor = Some(scorer.of(item));
            }
        }
        if !path.is_empty() {
            paths.push((slot.to_string(), path));
        }
    }
    paths
}

/// The gender worn gear was made for, when the build doesn't say: pieces made for one tell.
pub fn worn_gender(build: &Build, catalog: &Catalog) -> Option<String> {
    build.start.pieces.iter().filter_map(|p| p.item).filter_map(|id| catalog.item(id)).find_map(|i| i.gender.clone())
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
        // Added without a level read: the data's is shown.
        let build = bandit(10, vec![2, 3, 4], vec![Piece { level: None, ..worn(1, &catalog) }]);
        let outfit = at(&build, &catalog, 10);
        let weapon = slot(&outfit, "Weapon");
        assert_eq!(weapon.wearing.as_ref().map(|p| (p.name.as_str(), p.level)), Some(("Old Dagger", Some(10))));
        assert_eq!(weapon.next.as_ref().map(|n| n.item.name.as_str()), Some("Field Dagger"));
        assert!(weapon.planned);
        assert_eq!(weapon.later, 2);
    }

    #[test]
    fn the_best_goal_in_reach_is_suggested_and_next_waits_for_a_level() {
        let catalog = catalog();
        let build = bandit(22, vec![2, 3, 4], vec![worn(1, &catalog)]);
        let weapon = slot(&at(&build, &catalog, 22), "Weapon").clone();
        assert_eq!(weapon.suggested.map(|i| i.name), Some("Coconut Knife".into()));
        assert!(weapon.suggested_planned);
        assert_eq!(weapon.next.map(|n| (n.item.name, n.level)), Some(("Iron Dagger".into(), Some(25))));
        assert!(weapon.planned);
        assert_eq!(weapon.later, 0);
    }

    #[test]
    fn each_piece_says_whether_the_character_meets_it_now() {
        let mut field = dagger(2, "Field Dagger", 15, 25);
        field.req_dex = 30;
        let catalog = Catalog::new(vec![skill(1, "Double Stab", 400, 20, &[])], vec![field, dagger(4, "Iron Dagger", 25, 35)]);
        // Wearing the Field Dagger at Lv 16 with 25 DEX: old enough, but 5 DEX short.
        let build = bandit(16, vec![], vec![worn(2, &catalog)]);
        let weapon = slot(&at(&build, &catalog, 16), "Weapon").clone();
        let wearing = weapon.wearing_fit.expect("a fit for what is worn");
        assert!(!wearing.ok);
        assert_eq!(wearing.short, ["DEX 30 (you have 25)"]);
        // The Iron Dagger comes at Lv 25.
        let next = weapon.next_fit.expect("a fit for what is next");
        assert_eq!(next.short, ["Lv 25 (you're 16)"]);
    }

    #[test]
    fn a_better_piece_in_reach_beats_an_older_goal() {
        let catalog = catalog();
        // The plan only aims for the Field Dagger, but at Lv 22 the Coconut Knife can be worn.
        let build = bandit(22, vec![2], vec![worn(1, &catalog)]);
        let weapon = slot(&at(&build, &catalog, 22), "Weapon").clone();
        assert_eq!(weapon.suggested.map(|i| i.name), Some("Coconut Knife".into()));
        assert!(!weapon.suggested_planned);
        assert_eq!(weapon.next.map(|n| (n.item.name, n.level)), Some(("Iron Dagger".into(), Some(25))));
        assert!(!weapon.planned);
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
        // Wearing the Lv 10 dagger, with no goals: the first better dagger comes at Lv 15, and
        // every tier after it follows in turn.
        let build = bandit(10, vec![], vec![worn(1, &catalog)]);
        let weapon = slot(&at(&build, &catalog, 10), "Weapon").clone();
        assert_eq!(weapon.suggested, None, "nothing better to wear yet");
        assert_eq!(weapon.next.as_ref().map(|n| (n.item.name.as_str(), n.level)), Some(("Field Dagger", Some(15))));
        assert!(!weapon.planned);
        assert_eq!(weapon.later, 2, "the Coconut Knife and the Iron Dagger after it");

        // At Lv 22 the Coconut Knife can be worn: it is suggested, and Next is the one after it.
        let weapon = slot(&at(&build, &catalog, 22), "Weapon").clone();
        assert_eq!(weapon.suggested.map(|i| i.name), Some("Coconut Knife".into()));
        assert_eq!(weapon.next.map(|n| (n.item.name, n.level)), Some(("Iron Dagger".into(), Some(25))));
    }

    #[test]
    fn an_empty_slot_fills_with_an_upgrade_every_ten_levels() {
        let catalog = Catalog::new(
            vec![skill(1, "Double Stab", 400, 20, &[])],
            vec![
                dagger(1, "Old Dagger", 10, 17),
                dagger(2, "Field Dagger", 15, 25),
                with(item(10, "Plain Hat", 10, &[]), "Hat", &[("Weapon Def.", 5)]),
                with(item(11, "Luk Hat", 10, &[]), "Hat", &[("LUK", 3)]),
                with(item(12, "Better Hat", 15, &[]), "Hat", &[("LUK", 5)]),
                with(item(13, "Big Hat", 25, &[]), "Hat", &[("LUK", 8)]),
                with(item(14, "Same Hat", 28, &[]), "Hat", &[("LUK", 8)]),
                with(item(20, "Work Gloves", 10, &[]), "Glove", &[("Weapon Def.", 2)]),
            ],
        );
        // Wearing the plain hat, with a weapon goal already: the weapon is left as planned.
        let build = bandit(10, vec![2], vec![worn(10, &catalog)]);
        let rules = Rules::default();
        let paths = fill_paths(&build, &expand(&build, &rules, &catalog), &rules, &catalog, 10);
        let path = |slot: &str| paths.iter().find(|(s, _)| s == slot).map(|(_, ids)| ids.clone());
        assert_eq!(path("Weapon"), None, "a slot with goals keeps them");
        assert_eq!(path("Hat"), Some(vec![11, 12, 13]), "no step for a hat only as good");
        assert_eq!(path("Glove"), Some(vec![20]));
        assert_eq!(path("Cape"), None, "nothing in the data for it");
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
    fn a_scrolled_copy_of_an_item_is_still_that_item_and_is_never_suggested_again() {
        let robin = |id| with(item(id, "Green Robin Hat", 20, &[]), "Hat", &[("DEX", 2), ("Weapon Def.", 20), ("Accuracy", 2)]);
        // The data repeats the hat under a second ID, as it does for some items.
        let catalog = Catalog::new(Vec::new(), vec![robin(30), robin(31)]);
        assert_eq!(catalog.copies_of(30), [30, 31]);

        // Worn and scrolled, but the check lost the Accuracy line, so by its stats it scores
        // below the data's copy.
        let read = vec![PieceStat { label: "DEX".into(), value: 2 }, PieceStat { label: "Weapon Def.".into(), value: 21 }];
        let scrolled = Piece { id: 1, upgrades: Some(1), stats: read, ..Piece::of_item(catalog.item(30).unwrap()) };
        // The same, never matched to the data, so only its name says what it is.
        let unmatched = Piece { item: None, ..scrolled.clone() };

        for piece in [scrolled, unmatched] {
            let build = bandit(20, vec![30, 31], vec![piece]);
            let e = expand(&build, &Rules::default(), &catalog);
            assert!(e.gear.is_empty(), "a worn goal isn't new gear, under either ID");
            let outfit = at(&build, &catalog, 20);
            let hat = slot(&outfit, "Hat");
            assert!(hat.wearing.is_some());
            assert_eq!(hat.suggested, None, "it is already worn");
            assert_eq!(hat.next, None, "and it isn't a step up from itself");
            let paths = fill_paths(&build, &e, &Rules::default(), &catalog, 20);
            assert!(paths.iter().all(|(slot, _)| slot != "Hat"), "nor a filled-in upgrade");
        }
    }

    #[test]
    fn the_piece_an_item_would_replace_still_counts_toward_its_requirements() {
        // Boots that need 40 STR, a character with 38, and worn boots that give 2: the game's
        // hover shows the requirement met, so the new boots are wearable as soon as the level is.
        let mut steel = with(item(2, "Steel Boots", 50, &[]), "Shoes", &[("Weapon Def.", 26)]);
        steel.req_str = 40;
        let catalog = Catalog::new(Vec::new(), vec![with(item(1, "Old Boots", 25, &[]), "Shoes", &[("STR", 2)]), steel]);
        let mut build = bandit(45, vec![2], vec![worn(1, &catalog)]);
        build.start.stats = Some(Stats { str: 38, dex: 25, int: 4, luk: 40 });
        build.plan.plan_to = 70;
        let e = expand(&build, &Rules::default(), &catalog);
        assert_eq!(e.gear.iter().map(|g| (g.item.name.as_str(), g.level)).collect::<Vec<_>>(), [("Steel Boots", Some(50))]);
        // With 37 base STR the 2 from the old boots falls one short of 40 until the plan raises it.
        build.start.stats = Some(Stats { str: 37, dex: 25, int: 4, luk: 40 });
        assert_eq!(expand(&build, &Rules::default(), &catalog).gear[0].level, None);
    }

    #[test]
    fn every_tier_up_to_the_plans_end_is_a_step() {
        // Shoes worn at Lv 10; a step up at Lv 40, a better one at Lv 41, a far better one at Lv 60.
        let shoes = |id, name: &str, level, def| with(item(id, name, level, &[]), "Shoes", &[("Weapon Def.", def)]);
        let catalog = Catalog::new(
            Vec::new(),
            vec![shoes(1, "Old Shoes", 10, 5), shoes(2, "Pierre", 40, 22), shoes(3, "Better", 41, 26), shoes(4, "Far Off", 60, 40)],
        );
        let mut build = bandit(10, Vec::new(), vec![worn(1, &catalog)]);
        build.plan.plan_to = 70;
        let outfit = at(&build, &catalog, 10);
        let next = slot(&outfit, "Shoes").next.as_ref().map(|n| (n.item.name.as_str(), n.level));
        assert_eq!(next, Some(("Pierre", Some(40))));
        let path = outfit.paths.iter().find(|p| p.slot == "Shoes").unwrap();
        let steps: Vec<_> = path.steps.iter().map(|s| (s.item.name.as_str(), s.level)).collect();
        assert_eq!(steps, [("Pierre", 40), ("Better", 41), ("Far Off", 60)]);
    }

    /// Crossbows as the data has them for Lv 30 to 70, with the DEX each asks.
    fn crossbows() -> Vec<ItemRecord> {
        let crossbow = |id, name: &str, level, dex, attack| {
            let mut c = with(item(id, name, level, &["Bowman"]), "Weapon", &[("Attack", attack)]);
            c.weapon_type = Some("Crossbow".into());
            c.req_dex = dex;
            c
        };
        vec![
            crossbow(30, "Eagle Crow", 30, 65, 52),
            crossbow(40, "Heckler", 40, 85, 59),
            crossbow(45, "Silver Crow", 45, 95, 67),
            crossbow(50, "Rower", 50, 105, 72),
            crossbow(60, "Golden Crow", 60, 125, 82),
            crossbow(70, "Gross Jaeger", 70, 145, 92),
        ]
    }

    #[test]
    fn a_plan_that_names_some_tiers_still_gets_every_tier_between() {
        // A Crossbowman at Lv 33 wearing the Eagle Crow, aiming for three of the five crossbows to
        // come: the Silver Crow and the Golden Crow fill the gaps, and nothing at or below Lv 33.
        let mut items = crossbows();
        items.push(with(item(90, "Green Hunter", 25, &["Bowman"]), "Hat", &[("DEX", 2), ("Weapon Def.", 24)]));
        items.push(with(item(91, "Green Hawkeye", 30, &["Bowman"]), "Hat", &[("DEX", 3), ("Weapon Def.", 27)]));
        items.push(with(item(92, "Old Cap", 10, &[]), "Hat", &[("Weapon Def.", 5)]));
        let catalog = Catalog::new(vec![skill(1, "Iron Arrow", 320, 20, &[])], items);
        let build = Build {
            start: Start { level: 33, stats: Some(Stats { str: 30, dex: 160, int: 4, luk: 4 }), pieces: vec![worn(30, &catalog), worn(92, &catalog)], ..Start::default() },
            plan: Plan { job: 320, stats: vec![StatGoal { stat: Stat::Dex, target: None, from_level: None }], gear: vec![40, 50, 70, 90, 91], plan_to: 70, ..Plan::default() },
            ..Build::default()
        };
        let outfit = at(&build, &catalog, 33);
        let path = |slot: &str| outfit.paths.iter().find(|p| p.slot == slot).unwrap().clone();
        let weapon = path("Weapon");
        assert_eq!(weapon.now, None, "nothing better than the Eagle Crow yet");
        let steps: Vec<_> = weapon.steps.iter().map(|s| (s.item.name.as_str(), s.level, s.planned)).collect();
        assert_eq!(
            steps,
            [("Heckler", 40, true), ("Silver Crow", 45, false), ("Rower", 50, true), ("Golden Crow", 60, false), ("Gross Jaeger", 70, true)]
        );
        // Two hat goals at or below Lv 33: only the better one, as the piece to get now.
        let hat = path("Hat");
        assert_eq!(hat.now.as_ref().map(|s| (s.item.name.as_str(), s.planned)), Some(("Green Hawkeye", true)));
        assert_eq!(hat.passed.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(), ["Green Hunter"]);
        // Character → Gear's next step and the level by level list say the same.
        assert_eq!(slot(&outfit, "Weapon").next.as_ref().map(|n| n.item.name.as_str()), Some("Heckler"));
        let e = expand(&build, &Rules::default(), &catalog);
        let gets: Vec<(u32, &str)> = e.steps.iter().flat_map(|s| s.gear.iter().map(move |g| (s.level, g.as_str()))).collect();
        assert_eq!(gets, [(33, "Green Hawkeye"), (40, "Heckler"), (45, "Silver Crow"), (50, "Rower"), (60, "Golden Crow"), (70, "Gross Jaeger")]);
    }

    #[test]
    fn a_goal_of_the_same_level_stays_when_it_is_better_and_goes_when_it_is_not() {
        let mut items = vec![dagger(1, "Worn Dagger", 25, 30), dagger(2, "Better Dagger", 25, 33), dagger(3, "Worse Dagger", 25, 28)];
        items.push(dagger(4, "Younger Dagger", 20, 20));
        let catalog = Catalog::new(Vec::new(), items);
        let build = bandit(25, vec![2, 3, 4], vec![worn(1, &catalog)]);
        let e = expand(&build, &Rules::default(), &catalog);
        let kept: Vec<&str> = e.gear.iter().map(|g| g.item.name.as_str()).collect();
        assert_eq!(kept, ["Better Dagger"], "the same level and better stays; worse, and a lower level, are outgrown");
        let weapon = at(&build, &catalog, 25).slots.into_iter().find(|s| s.slot == "Weapon").unwrap();
        assert_eq!(weapon.suggested.map(|i| i.name), Some("Better Dagger".into()));
    }

    #[test]
    fn earrings_are_told_apart_by_crit_and_defence_not_all_tied() {
        let earring = |id, name: &str, level, magic_def, crit| {
            with(item(id, name, level, &[]), "Earring", &[("Magic Def.", magic_def), ("Crit. Damage", crit)])
        };
        let catalog = Catalog::new(Vec::new(), vec![earring(1, "Yellow Square", 20, 22, 1), earring(2, "Emerald", 30, 27, 2), earring(3, "Cat's Eye", 35, 30, 2)]);
        // As a tooltip spelled it: the worn one's crit is `Critical Damage`, which counts all the same.
        let worn = Piece {
            id: 1,
            stats: vec![PieceStat { label: "Magic Def.".into(), value: 22 }, PieceStat { label: "Critical Damage".into(), value: 1 }],
            ..Piece::of_item(catalog.item(1).unwrap())
        };
        let build = bandit(33, Vec::new(), vec![worn]);
        let outfit = at(&build, &catalog, 33);
        let row = slot(&outfit, "Earring");
        assert_eq!(row.suggested.as_ref().map(|i| i.name.as_str()), Some("Emerald"), "wearable now and better");
        assert_eq!(row.next.as_ref().map(|n| (n.item.name.as_str(), n.level)), Some(("Cat's Eye", Some(35))));
    }

    #[test]
    fn next_is_the_nearest_step_up_even_when_the_plan_only_names_a_later_one() {
        let catalog = catalog();
        // The plan aims for the Iron Dagger (Lv 25), but better daggers than the worn Old Dagger
        // come at Lv 15 and 20: the first of them is what comes next, and the goal still follows.
        let build = bandit(10, vec![4], vec![worn(1, &catalog)]);
        let outfit = at(&build, &catalog, 10);
        let weapon = slot(&outfit, "Weapon");
        assert_eq!(weapon.next.as_ref().map(|n| (n.item.name.as_str(), n.level)), Some(("Field Dagger", Some(15))));
        assert!(!weapon.planned, "it isn't one of the plan's goals");
        assert_eq!(weapon.later, 2, "the Coconut Knife, then the planned Iron Dagger");
        // When the plan's goal is the nearest, it stays the one shown.
        let build = bandit(10, vec![2, 4], vec![worn(1, &catalog)]);
        let weapon = at(&build, &catalog, 10).slots.into_iter().find(|s| s.slot == "Weapon").unwrap();
        assert_eq!(weapon.next.as_ref().map(|n| n.item.name.as_str()), Some("Field Dagger"));
        assert!(weapon.planned);
        assert_eq!(weapon.later, 2);
    }

    #[test]
    fn the_timeline_lists_only_the_best_goal_a_slot_can_wear_at_once_and_nothing_outgrown() {
        let catalog = catalog();
        let listed = |build: &Build| -> Vec<String> {
            let e = expand(build, &Rules::default(), &catalog);
            e.steps.iter().flat_map(|s| s.gear.clone()).collect()
        };
        // At Lv 25 every dagger is within reach together: the character goes to the best, and
        // the three before it aren't "things to get". Slots with no goals get the data's best.
        assert_eq!(listed(&bandit(25, vec![1, 2, 3, 4], Vec::new())), ["Iron Dagger", "Luk Hat", "Work Gloves"]);
        // What is worn counts too: nothing at or below its level is a goal.
        let build = bandit(25, vec![1, 2, 3, 4], vec![worn(3, &catalog), worn(11, &catalog), worn(20, &catalog)]);
        assert_eq!(listed(&build), ["Iron Dagger"]);
        let e = expand(&build, &Rules::default(), &catalog);
        assert_eq!(e.gear.iter().map(|g| g.item.name.as_str()).collect::<Vec<_>>(), ["Iron Dagger"], "older goals are dropped");
        // A goal still to come stays on its level.
        let daggers: Vec<String> = listed(&bandit(10, vec![1, 2, 3, 4], Vec::new())).into_iter().filter(|g| g.contains("Dagger") || g.contains("Knife")).collect();
        assert_eq!(daggers, ["Old Dagger", "Field Dagger", "Coconut Knife", "Iron Dagger"]);
    }

    #[test]
    fn a_piece_saved_from_a_noisy_read_shows_the_items_own_name() {
        let catalog = catalog();
        let garbled = Piece { item: None, name: "Old Dagger ossbow".into(), ..worn(1, &catalog) };
        let build = bandit(10, Vec::new(), vec![garbled]);
        let outfit = at(&build, &catalog, 10);
        let weapon = slot(&outfit, "Weapon");
        let piece = weapon.wearing.as_ref().unwrap();
        assert_eq!((piece.name.as_str(), piece.item), ("Old Dagger", Some(1)));
        // A name the data has nothing like stays as the player wrote it.
        let own = Piece { item: None, name: "My Homemade Blade".into(), ..worn(1, &catalog) };
        let outfit = at(&bandit(10, Vec::new(), vec![own]), &catalog, 10);
        assert_eq!(slot(&outfit, "Weapon").wearing.as_ref().map(|p| (p.name.as_str(), p.item)), Some(("My Homemade Blade", None)));
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
