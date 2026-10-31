//! Ready-made plans written for Leafline, one for each second job, Lv 1 to 70. They come from
//! reading what each skill does and what each weapon asks for, not from anyone's guide, and
//! nobody has played them yet, so they are places to start and the app says so.
//!
//! The file holds names and levels only. Names become ids from the downloaded data when a plan is
//! offered, so no game data ships, and a name the data no longer has is reported.

use std::collections::HashMap;
use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

use super::catalog::{Catalog, path_of};
use super::plan::{Plan, SkillLevel, Stat, StatGoal};
use super::rules::Rules;

/// The plans run to here.
const PLAN_TO: u32 = 70;
/// What a secondary stat is taken to start at: the lowest seen, so a step is never late.
const LOWEST_START: u32 = 4;

type Goals = Vec<(String, u32)>;

#[derive(Deserialize)]
struct File {
    beginner: Goals,
    openings: HashMap<String, Goals>,
    plans: Vec<Raw>,
}

#[derive(Deserialize)]
struct Raw {
    id: String,
    job: u32,
    name: String,
    summary: String,
    /// The 1st-job skills, by key in `openings`.
    opening: String,
    /// The 2nd-job skills.
    skills: Goals,
    primary: Stat,
    secondary: Stat,
    /// What `secondary` must reach, and by which level, for the weapons in `gear`.
    steps: Vec<(u32, u32)>,
    gear: Vec<String>,
}

static FILE: LazyLock<File> =
    LazyLock::new(|| serde_json::from_str(include_str!("starters.json")).expect("starters.json is checked by the tests"));

/// A plan ready to use, with its names turned into this data's ids.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Starter {
    pub id: String,
    pub job: u32,
    pub name: String,
    pub summary: String,
    pub plan: Plan,
    /// Names the data doesn't have, left out of the plan.
    pub missing: Vec<String>,
}

/// Every plan, in job order.
pub fn all(catalog: &Catalog, rules: &Rules) -> Vec<Starter> {
    FILE.plans.iter().map(|raw| resolve(raw, catalog, rules)).collect()
}

fn resolve(raw: &Raw, catalog: &Catalog, rules: &Rules) -> Starter {
    let mut jobs = vec![0];
    jobs.extend(path_of(raw.job));
    let mut missing = Vec::new();

    let opening = FILE.openings.get(&raw.opening).map_or(&[][..], Vec::as_slice);
    let goals = FILE.beginner.iter().chain(opening).chain(&raw.skills);
    let mut skills = Vec::new();
    for (name, level) in goals {
        match catalog.skill_named(name, &jobs) {
            Some(skill) => skills.push(SkillLevel { skill_id: skill.id, level: *level }),
            None => missing.push(name.clone()),
        }
    }
    let mut gear = Vec::new();
    for name in &raw.gear {
        match catalog.item_named(name) {
            Some(item) => gear.push(item.id),
            None => missing.push(name.clone()),
        }
    }
    let plan = Plan { job: raw.job, skills, stats: stat_goals(raw, rules), gear, plan_to: PLAN_TO };
    Starter { id: raw.id.clone(), job: raw.job, name: raw.name.clone(), summary: raw.summary.clone(), plan, missing }
}

/// The secondary stat in time for each step, starting just early enough to reach it, and
/// everything else into the primary stat.
fn stat_goals(raw: &Raw, rules: &Rules) -> Vec<StatGoal> {
    let per_level = rules.ap_per_level.max(1);
    let mut have = LOWEST_START;
    let mut goals = Vec::with_capacity(raw.steps.len() + 1);
    for &(target, by) in &raw.steps {
        let levels = target.saturating_sub(have).div_ceil(per_level);
        let from = (by + 1).saturating_sub(levels).max(2);
        goals.push(StatGoal { stat: raw.secondary, target: Some(target), from_level: Some(from) });
        have = target;
    }
    goals.push(StatGoal { stat: raw.primary, target: None, from_level: None });
    goals
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    /// A goal list's last word on each skill. A skill can come back, so only its highest counts.
    fn spent(goals: &[(String, u32)]) -> u32 {
        let mut levels = BTreeMap::new();
        for (name, level) in goals {
            let top = levels.entry(name).or_insert(0);
            *top = (*top).max(*level);
        }
        levels.values().sum()
    }

    #[test]
    fn every_plan_spends_each_jobs_sp_exactly() {
        assert_eq!(spent(&FILE.beginner), 9);
        for (key, goals) in &FILE.openings {
            assert_eq!(spent(goals), 61, "the {key} opening: 1 at the advancement and 3 a level to Lv 30");
        }
        for raw in &FILE.plans {
            assert_eq!(spent(&raw.skills), 121, "{}: 1 at the advancement and 3 a level to Lv 70", raw.name);
            assert!(FILE.openings.contains_key(&raw.opening), "{}", raw.name);
        }
    }

    #[test]
    fn there_is_a_plan_for_each_second_job() {
        let mut jobs: Vec<u32> = FILE.plans.iter().map(|p| p.job).collect();
        jobs.sort_unstable();
        assert_eq!(jobs, [110, 120, 130, 210, 220, 230, 310, 320, 410, 420]);
    }

    #[test]
    fn steps_only_ever_rise() {
        for raw in &FILE.plans {
            assert!(raw.steps.windows(2).all(|w| w[0].0 < w[1].0 && w[0].1 < w[1].1), "{}", raw.name);
            assert_ne!(raw.primary, raw.secondary, "{}", raw.name);
        }
    }

    fn raw(steps: Vec<(u32, u32)>) -> Raw {
        Raw {
            id: "test".into(),
            job: 410,
            name: "Test".into(),
            summary: String::new(),
            opening: String::new(),
            skills: Vec::new(),
            primary: Stat::Luk,
            secondary: Stat::Dex,
            steps,
            gear: Vec::new(),
        }
    }

    #[test]
    fn a_step_starts_just_early_enough() {
        // From 4: 11 more takes three levels, so Lv 13 to 15 reach 15 by Lv 15.
        let goals = stat_goals(&raw(vec![(15, 15), (20, 20)]), &Rules::default());
        assert_eq!(goals[0].from_level, Some(13));
        assert_eq!(goals[1].from_level, Some(20));
        assert_eq!(goals[2], StatGoal { stat: Stat::Luk, target: None, from_level: None });
    }

    #[test]
    fn a_step_too_soon_starts_at_the_first_level_up() {
        let goals = stat_goals(&raw(vec![(100, 3)]), &Rules::default());
        assert_eq!(goals[0].from_level, Some(2));
    }

    #[test]
    fn names_the_data_lacks_are_reported() {
        let found = all(&Catalog::default(), &Rules::default());
        assert_eq!(found.len(), 10);
        assert!(found[0].plan.skills.is_empty());
        assert!(found[0].missing.contains(&"Three Snails".to_string()));
        assert!(found[0].missing.contains(&"Long Sword".to_string()));
    }

    #[test]
    fn the_same_skill_name_resolves_within_the_job() {
        use super::super::catalog::tests::skill;
        let catalog = Catalog::new(
            vec![skill(1, "Sword Mastery", 110, 20, &[]), skill(2, "Sword Mastery", 120, 20, &[])],
            Vec::new(),
        );
        assert_eq!(catalog.skill_named("Sword Mastery", &[0, 100, 120]).unwrap().id, 2);
        assert!(catalog.skill_named("Sword Mastery", &[0, 400]).is_none());
    }
}
