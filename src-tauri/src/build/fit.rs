//! Whether a hovered item suits the build: its requirements, as the tooltip reader read them,
//! against the plan's base stats and the goal gear worn by then, leaving out what the item
//! itself would replace.

use serde::{Deserialize, Serialize};

use super::plan::{Expansion, LevelStep, Stat, Stats};
use super::rules::Branch;
use crate::readers::tooltip::Requirements;

/// A job flag from the tooltip, as the overlay passes it back.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ItemJob {
    pub job: String,
    /// Whether the flag is highlighted, meaning the job can use the item.
    pub eligible: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemFit {
    /// The level this was judged at.
    pub level: u32,
    /// Whether the job can use it. `None` when no job flag was read for it.
    pub job_ok: Option<bool>,
    pub wearable_now: bool,
    /// The first level the plan can wear it, if any.
    pub wearable_at: Option<u32>,
    /// What is missing at `level`.
    pub short: Vec<Shortfall>,
    /// A requirement could not be read, so this may be wrong.
    pub unread: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Shortfall {
    pub label: String,
    pub amount: u32,
}

/// Armour slots as the item data names them. A tooltip type that is none of these is a weapon.
pub const ARMOUR: [&str; 9] = ["Hat", "Top", "Bottom", "Overall", "Shoes", "Glove", "Cape", "Shield", "Earring"];

/// Whether worn gear in `slot` would come off for an item of the tooltip's `item_type`.
fn replaces(slot: &str, item_type: Option<&str>) -> bool {
    let Some(item_type) = item_type else { return false };
    let lower = item_type.to_lowercase();
    match ARMOUR.iter().find(|a| lower.contains(&a.to_lowercase())) {
        Some(armour) => slot.eq_ignore_ascii_case(armour),
        None => slot.eq_ignore_ascii_case("Weapon"),
    }
}

/// The stats that count toward an item at this step.
fn stats_for(step: &LevelStep, item_type: Option<&str>) -> Stats {
    step.worn.iter().filter(|w| !replaces(&w.slot, item_type)).fold(step.stats, |total, w| total.plus(w.bonus))
}

/// Judges an item at `level`, or at the plan's start without one. `None` for a plan with no
/// levels or no stats to go on, where nothing can be said.
pub fn check(
    requirements: &Requirements,
    jobs: &[ItemJob],
    item_type: Option<&str>,
    expansion: &Expansion,
    branch: Option<Branch>,
    level: Option<u32>,
) -> Option<ItemFit> {
    let step = match level {
        Some(level) => expansion.step(level)?,
        None => expansion.steps.first()?,
    };
    let level = level.unwrap_or(step.level);
    let short = shortfalls(requirements, &stats_for(step, item_type), level);
    let wearable_at = expansion
        .steps
        .iter()
        .find(|s| shortfalls(requirements, &stats_for(s, item_type), s.level).is_empty())
        .map(|s| s.level);
    let job_ok = branch.and_then(|b| jobs.iter().find(|f| f.job.eq_ignore_ascii_case(b.name()))).map(|f| f.eligible);
    let needs = [requirements.level, requirements.str, requirements.dex, requirements.int, requirements.luk];
    Some(ItemFit {
        level,
        job_ok,
        wearable_now: short.is_empty(),
        wearable_at,
        short,
        unread: needs.iter().any(Option::is_none),
    })
}

fn shortfalls(requirements: &Requirements, stats: &Stats, level: u32) -> Vec<Shortfall> {
    let mut short = Vec::new();
    if let Some(amount) = requirements.level.and_then(|need| need.checked_sub(level)).filter(|a| *a > 0) {
        short.push(Shortfall { label: "Lv".into(), amount });
    }
    let stats_needed = [
        (Stat::Str, requirements.str),
        (Stat::Dex, requirements.dex),
        (Stat::Int, requirements.int),
        (Stat::Luk, requirements.luk),
    ];
    for (stat, need) in stats_needed {
        if let Some(amount) = need.and_then(|need| need.checked_sub(stats.get(stat))).filter(|a| *a > 0) {
            short.push(Shortfall { label: stat.label().into(), amount });
        }
    }
    short
}

#[cfg(test)]
mod tests {
    use super::super::catalog::Catalog;
    use super::super::plan::{Build, Plan, Start, StatGoal, Worn, expand};
    use super::super::rules::Rules;
    use super::*;

    /// A thief from Lv 10 with 25 DEX, all new AP into DEX.
    fn expansion() -> Expansion {
        let build = Build {
            start: Start { level: 10, stats: Some(Stats { str: 4, dex: 25, int: 4, luk: 4 }), ..Start::default() },
            plan: Plan {
                job: 400,
                stats: vec![StatGoal { stat: Stat::Dex, target: None, from_level: None }],
                plan_to: 20,
                ..Plan::default()
            },
            ..Build::default()
        };
        expand(&build, &Rules::default(), &Catalog::default())
    }

    fn needs(level: u32, dex: u32) -> Requirements {
        Requirements { level: Some(level), str: Some(0), dex: Some(dex), int: Some(0), luk: Some(0), fam: Some(0) }
    }

    fn check_at(requirements: &Requirements, level: u32) -> ItemFit {
        check(requirements, &[], None, &expansion(), Some(Branch::Thief), Some(level)).unwrap()
    }

    #[test]
    fn worn_gear_counts_unless_the_item_would_replace_it() {
        let mut plan = expansion();
        for step in &mut plan.steps {
            step.worn = vec![Worn { slot: "Hat".into(), bonus: Stats { dex: 20, ..Stats::default() } }];
        }
        let need = needs(10, 50);
        let judged = |item_type| check(&need, &[], item_type, &plan, None, Some(12)).unwrap();
        assert!(judged(Some("Claw")).wearable_now, "35 DEX and the hat's 20");
        assert!(judged(None).wearable_now);
        assert!(!judged(Some("Hat")).wearable_now, "the hat would come off");
    }

    #[test]
    fn tooltip_types_map_to_slots() {
        assert!(replaces("Glove", Some("Gloves")));
        assert!(replaces("Weapon", Some("Claw")));
        assert!(!replaces("Weapon", Some("Hat")));
        assert!(!replaces("Hat", None));
    }

    #[test]
    fn an_item_within_the_plan_fits_now() {
        let fit = check_at(&needs(10, 25), 12);
        assert!(fit.wearable_now && fit.short.is_empty() && !fit.unread);
    }

    #[test]
    fn says_what_is_short_and_when_it_comes() {
        let fit = check_at(&needs(15, 50), 12);
        assert!(!fit.wearable_now);
        let short: Vec<_> = fit.short.iter().map(|s| (s.label.as_str(), s.amount)).collect();
        assert_eq!(short, [("Lv", 3), ("DEX", 15)]);
        assert_eq!(fit.wearable_at, Some(15));
    }

    #[test]
    fn an_item_beyond_the_plan_has_no_level() {
        assert_eq!(check_at(&needs(40, 25), 12).wearable_at, None);
        assert_eq!(check_at(&needs(10, 500), 12).wearable_at, None);
    }

    #[test]
    fn the_jobs_flag_decides_whether_it_can_use_it() {
        let flag = |job: &str, eligible| ItemJob { job: job.into(), eligible };
        let flags = [flag("WARRIOR", true), flag("THIEF", false)];
        let fit = check(&needs(10, 0), &flags, None, &expansion(), Some(Branch::Thief), Some(12)).unwrap();
        assert_eq!(fit.job_ok, Some(false));
        let fit = check(&needs(10, 0), &flags, None, &expansion(), Some(Branch::Warrior), Some(12)).unwrap();
        assert_eq!(fit.job_ok, Some(true));
        let fit = check(&needs(10, 0), &[], None, &expansion(), Some(Branch::Thief), Some(12)).unwrap();
        assert_eq!(fit.job_ok, None);
    }

    #[test]
    fn an_unread_requirement_is_said_not_assumed() {
        let fit = check_at(&Requirements { dex: None, ..needs(10, 0) }, 12);
        assert!(fit.unread && fit.wearable_now);
    }

    #[test]
    fn with_no_level_read_it_judges_from_the_start_of_the_plan() {
        let fit = check(&needs(15, 25), &[], None, &expansion(), Some(Branch::Thief), None).unwrap();
        assert_eq!(fit.level, 10);
        assert_eq!(fit.wearable_at, Some(15));
    }

    #[test]
    fn a_level_past_the_plan_is_judged_on_its_last_stats() {
        let fit = check_at(&needs(30, 75), 30);
        assert!(fit.wearable_now, "{fit:?}");
    }
}
