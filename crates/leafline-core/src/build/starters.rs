//! Ready-made plans, Lv 1 to 70. A plan with a `source` follows one written guide and says so with
//! a credit. The rest were written for Leafline from the downloaded skill and item data: what each
//! skill does at each level and what each weapon asks for. They may link a guide as further
//! reading, but nothing in them is taken from it. Nobody has played any of them yet, so they are
//! places to start and the app says so.
//!
//! The file holds names and levels only. Names become ids from the downloaded data when a plan is
//! offered, so no game data ships, and a name the data no longer has is reported.

use std::collections::HashMap;
use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

use super::catalog::{Catalog, path_of};
use super::plan::{Plan, SkillLevel, Stat, StatGoal, Stats};
use super::rules::Rules;

/// The plans run to here.
const PLAN_TO: u32 = 70;
/// What a secondary stat is taken to start at: the lowest seen, so a step is never late.
const LOWEST_START: u32 = 4;
/// The most a stat can start at: a new character splits 25 points, 4 to 12 each.
#[cfg(test)]
const HIGHEST_START: u32 = 12;

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
    /// Why to pick the path and what it gives up, for a plan written for Leafline.
    #[serde(default)]
    note: String,
    /// The roll the plan counts from at Lv 1: STR, DEX, INT and LUK, 25 points with each 4 to 12.
    start: [u32; 4],
    /// The 1st-job skills, by key in `openings`.
    opening: String,
    /// The 2nd-job skills.
    skills: Goals,
    primary: Stat,
    secondary: Stat,
    /// What `secondary` must reach, and by which level, for the weapons in `gear`.
    #[serde(default)]
    steps: Vec<(u32, u32)>,
    /// Stat steps a guide gives outright, replacing `steps`: the stat, what it reaches, and the
    /// level it starts at. Whatever AP they leave goes to `primary`.
    #[serde(default)]
    ap: Vec<(Stat, u32, u32)>,
    gear: Vec<String>,
    /// The guide this plan follows, if it follows one.
    #[serde(default)]
    source: Option<Source>,
    /// A guide worth reading beside a plan written for Leafline, which it doesn't follow.
    #[serde(default)]
    reading: Option<Source>,
}

/// Where a plan comes from, to credit and link.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Source {
    pub name: String,
    /// The exact page the plan was taken from.
    pub url: String,
    pub credit: String,
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
    /// Why to pick it and what it gives up. Empty for a guide's plan, whose guide says.
    pub note: String,
    /// The Lv 1 stats the plan counts from. A different roll changes when each step lands.
    pub start: Stats,
    pub plan: Plan,
    /// Names the data doesn't have, left out of the plan.
    pub missing: Vec<String>,
    pub source: Option<Source>,
    pub reading: Option<Source>,
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
    let [str, dex, int, luk] = raw.start;
    Starter {
        id: raw.id.clone(),
        job: raw.job,
        name: raw.name.clone(),
        summary: raw.summary.clone(),
        note: raw.note.clone(),
        start: Stats { str, dex, int, luk },
        plan,
        missing,
        source: raw.source.clone(),
        reading: raw.reading.clone(),
    }
}

/// The secondary stat in time for each step, starting just early enough to reach it, and
/// everything else into the primary stat.
fn stat_goals(raw: &Raw, rules: &Rules) -> Vec<StatGoal> {
    let per_level = rules.ap_per_level.max(1);
    let mut have = LOWEST_START;
    let mut goals = Vec::with_capacity(raw.steps.len() + raw.ap.len() + 1);
    for &(stat, target, from) in &raw.ap {
        goals.push(StatGoal { stat, target: Some(target), from_level: Some(from) });
    }
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
        jobs.dedup();
        assert_eq!(jobs, [110, 120, 130, 210, 220, 230, 310, 320, 410, 420]);
        // The first plan of a job is its default; the rest are choices, such as another weapon.
        for job in jobs {
            let first = FILE.plans.iter().find(|p| p.job == job).unwrap();
            assert!(!first.id.contains("-sword-") && !first.id.contains("-polearm-") && !first.id.contains("-doubleshot-"));
            assert!(!first.id.contains("-energybolt-") && !first.id.contains("-mesosaver-"), "{}", first.id);
        }
    }

    #[test]
    fn only_guide_plans_carry_a_credit_and_it_links_the_exact_page() {
        for raw in &FILE.plans {
            let guide = raw.id.ends_with("-niadb");
            assert_eq!(raw.source.is_some(), guide, "{}", raw.name);
            if let Some(source) = &raw.source {
                assert!(source.url.starts_with("https://meowdb.com/msclassic/guides/"), "{}", raw.name);
                assert!(!source.url.ends_with("/guides/") && !source.url.ends_with("/guides"), "not the index page");
            }
        }
    }

    /// What a guide's tables say a character has after some levels, from `starters_check.json`.
    #[derive(Deserialize)]
    struct Checks {
        /// Base STR, DEX, INT and LUK at Lv 1.
        start: [u32; 4],
        /// Skill levels after a level, for the skills it names.
        sp: Vec<(u32, HashMap<String, u32>)>,
        /// Base STR, DEX, INT and LUK at the end of a level.
        stats: Vec<(u32, [u32; 4])>,
    }

    /// The downloaded skills and items, when `LEAFLINE_OSMS` names a folder holding them, so the
    /// plans are checked against the real names and prerequisites.
    fn real_catalog() -> Option<Catalog> {
        use crate::mapdata::osms::Records;
        let dir = std::path::PathBuf::from(std::env::var_os("LEAFLINE_OSMS")?);
        let read = |name: &str| std::fs::read(dir.join(name)).ok();
        let Records::Skills(skills) = Records::parse("skills.json", &read("skills.json")?).ok()? else { return None };
        let Records::Items(items) = Records::parse("items.json", &read("items.json")?).ok()? else { return None };
        Some(Catalog::new(skills, items))
    }

    /// Every skill a plan names, with no prerequisites, filed under the job it belongs to.
    fn made_up_catalog(raw: &Raw) -> Catalog {
        use super::super::catalog::tests::skill;
        let mut seen: Vec<(String, u32)> = Vec::new();
        let opening = FILE.openings.get(&raw.opening).map_or(&[][..], Vec::as_slice);
        let groups = [(0, &FILE.beginner[..]), (raw.job / 100 * 100, opening), (raw.job, &raw.skills[..])];
        let mut skills = Vec::new();
        for (job, goals) in groups {
            for (name, _) in goals {
                if !seen.iter().any(|(n, j)| n == name && *j == job) {
                    seen.push((name.clone(), job));
                    let top = goals.iter().filter(|(n, _)| n == name).map(|g| g.1).max().unwrap_or(1);
                    skills.push(skill(seen.len() as u32, name, job, top, &[]));
                }
            }
        }
        Catalog::new(skills, Vec::new())
    }

    #[test]
    fn every_guide_plan_follows_its_guides_tables() {
        use super::super::plan::{Build, Start, Stats, expand};

        let checks: HashMap<String, Checks> = serde_json::from_str(include_str!("starters_check.json")).unwrap();
        let rules = Rules::default();
        let real = real_catalog();
        let plans: Vec<&Raw> = FILE.plans.iter().filter(|p| checks.contains_key(&p.id)).collect();
        assert_eq!(plans.len(), checks.len(), "every checked plan exists");
        assert!(plans.len() >= 9);

        for raw in plans {
            let made_up;
            let catalog = match &real {
                Some(catalog) => catalog,
                None => {
                    made_up = made_up_catalog(raw);
                    &made_up
                }
            };
            let starter = resolve(raw, catalog, &rules);
            if real.is_some() {
                assert!(starter.missing.is_empty(), "{}: not in the data: {:?}", raw.id, starter.missing);
            }
            let want = &checks[&raw.id];
            let [str, dex, int, luk] = want.start;
            let start = Start { level: 1, stats: Some(Stats { str, dex, int, luk }), ..Start::default() };
            let e = expand(&Build { start, plan: starter.plan, ..Build::default() }, &rules, catalog);

            for (level, skills) in &want.sp {
                let mut have: HashMap<&str, u32> = HashMap::new();
                for step in e.steps.iter().filter(|s| s.level <= *level) {
                    for gain in &step.sp {
                        have.insert(gain.name.as_str(), gain.to);
                    }
                }
                for (name, expected) in skills {
                    assert_eq!(have.get(name.as_str()).copied().unwrap_or(0), *expected, "{}: {name} after Lv {level}", raw.id);
                }
            }
            for (level, expected) in &want.stats {
                let s = e.steps.iter().find(|s| s.level == *level).unwrap().stats;
                assert_eq!([s.str, s.dex, s.int, s.luk], *expected, "{}: base stats after Lv {level}", raw.id);
            }
        }
    }

    /// The guide's own tables for Lv 30-70, for the plan that follows it: SP by level, then the
    /// base stats at the end of some levels.
    #[test]
    fn the_niadb_crossbowman_plan_matches_the_guides_tables() {
        use super::super::catalog::tests::skill;
        use super::super::plan::{Build, Start, Stats, expand};

        const MASTERY: u32 = 3_200_000;
        const JUDGEMENT: u32 = 3_200_001;
        const FINAL: u32 = 3_201_000;
        const BOOSTER: u32 = 3_201_001;
        const SOUL: u32 = 3_201_003;
        const IRON: u32 = 3_201_004;
        const CRITICAL: u32 = 3_000_000;
        const EYE: u32 = 3_000_001;
        const FOCUS: u32 = 3_001_000;
        const BLOW: u32 = 3_001_001;
        const KNOCKBACK: u32 = 3_001_003;
        let catalog = Catalog::new(
            vec![
                skill(CRITICAL, "Critical Shot", 300, 15, &[]),
                skill(EYE, "The Eye of Amazon", 300, 15, &[]),
                skill(FOCUS, "Focus", 300, 20, &[(EYE, 3)]),
                skill(BLOW, "Arrow Blow", 300, 20, &[]),
                skill(KNOCKBACK, "Power Knockback", 300, 15, &[]),
                skill(MASTERY, "Crossbow Mastery", 320, 20, &[]),
                skill(JUDGEMENT, "Amazon's Judgement", 320, 20, &[(MASTERY, 5)]),
                skill(FINAL, "Final Attack: Crossbow", 320, 30, &[(MASTERY, 3)]),
                skill(BOOSTER, "Crossbow Booster", 320, 20, &[(MASTERY, 5)]),
                skill(SOUL, "Soul Arrow: Crossbow", 320, 20, &[(BOOSTER, 5)]),
                skill(IRON, "Iron Arrow", 320, 30, &[]),
            ],
            Vec::new(),
        );
        let rules = Rules::default();
        let starter = all(&catalog, &rules).into_iter().find(|s| s.id == "crossbowman-niadb").unwrap();
        assert!(starter.source.is_some());
        let skill_names = ["Arrow Blow", "Critical Shot", "The Eye of Amazon", "Focus", "Power Knockback", "Iron Arrow"];
        let lost: Vec<_> = starter.missing.iter().filter(|n| skill_names.contains(&n.as_str()) || n.contains("Crossbow Mastery")).collect();
        assert!(lost.is_empty(), "skills the test catalog has were not found: {lost:?}");

        // A new character, as the Bowman guide starts it: 5 STR, 12 DEX, 4 INT, 4 LUK at Lv 1.
        let start = Start { level: 1, stats: Some(Stats { str: 5, dex: 12, int: 4, luk: 4 }), ..Start::default() };
        let build = Build { start, plan: starter.plan, ..Build::default() };
        let e = expand(&build, &rules, &catalog);

        let have = |at: u32, ids: &[u32]| -> Vec<u32> {
            let mut have = std::collections::HashMap::new();
            for step in e.steps.iter().filter(|s| s.level <= at) {
                for gain in &step.sp {
                    have.insert(gain.skill_id, gain.to);
                }
            }
            ids.iter().map(|id| have.get(id).copied().unwrap_or(0)).collect()
        };

        // The Bowman guide's first-job table, Lv 10-30: Arrow Blow, Critical Shot, Eye, Focus,
        // Power Knockback.
        let first = |at: u32| have(at, &[BLOW, CRITICAL, EYE, FOCUS, KNOCKBACK]);
        assert_eq!(first(10), [1, 0, 0, 0, 0]);
        assert_eq!(first(11), [4, 0, 0, 0, 0]);
        assert_eq!(first(12), [6, 0, 1, 0, 0]);
        assert_eq!(first(14), [12, 0, 1, 0, 0]);
        assert_eq!(first(15), [13, 0, 3, 0, 0]);
        assert_eq!(first(17), [19, 0, 3, 0, 0]);
        assert_eq!(first(18), [20, 2, 3, 0, 0]);
        assert_eq!(first(22), [20, 14, 3, 0, 0]);
        assert_eq!(first(23), [20, 15, 5, 0, 0]);
        assert_eq!(first(26), [20, 15, 14, 0, 0]);
        assert_eq!(first(27), [20, 15, 15, 2, 0]);
        assert_eq!(first(29), [20, 15, 15, 8, 0]);
        assert_eq!(first(30), [20, 15, 15, 10, 1]);

        // Skill levels after each level of the Crossbowman guide's SP table: Iron Arrow, Mastery,
        // Booster, Final Attack, Soul Arrow, Judgement.
        let levels = |at: u32| -> [u32; 6] {
            have(at, &[IRON, MASTERY, BOOSTER, FINAL, SOUL, JUDGEMENT]).try_into().unwrap()
        };
        assert_eq!(levels(30), [1, 0, 0, 0, 0, 0]);
        assert_eq!(levels(31), [1, 3, 0, 0, 0, 0]);
        assert_eq!(levels(32), [1, 5, 1, 0, 0, 0]);
        assert_eq!(levels(33), [1, 5, 3, 1, 0, 0]);
        assert_eq!(levels(34), [1, 5, 5, 1, 1, 0]);
        assert_eq!(levels(38), [1, 17, 5, 1, 1, 0]);
        assert_eq!(levels(39), [1, 20, 5, 1, 1, 0]);
        assert_eq!(levels(48), [28, 20, 5, 1, 1, 0]);
        assert_eq!(levels(49), [30, 20, 5, 1, 1, 1]);
        assert_eq!(levels(54), [30, 20, 20, 1, 1, 1]);
        assert_eq!(levels(61), [30, 20, 20, 3, 20, 1]);
        assert_eq!(levels(70), [30, 20, 20, 30, 20, 1]);

        // Base STR and DEX at the end of each level the guide's AP table names.
        let stats = |at: u32| {
            let s = e.steps.iter().find(|s| s.level == at).unwrap().stats;
            (s.str, s.dex)
        };
        for (level, expected) in [
            (10, (5, 57)),
            (13, (5, 72)),
            (19, (5, 102)),
            (20, (9, 103)),
            (24, (9, 123)),
            (25, (14, 123)),
            (29, (14, 143)),
            (30, (18, 144)),
            (38, (18, 184)),
            (40, (28, 184)),
            (48, (28, 224)),
            (50, (38, 224)),
            (65, (38, 299)),
            (66, (40, 302)),
            (70, (60, 302)),
        ] {
            assert_eq!(stats(level), expected, "Lv {level}");
        }
    }

    #[test]
    fn plans_written_for_leafline_say_why_and_link_a_guide_only_as_reading() {
        for raw in FILE.plans.iter().filter(|p| p.source.is_none()) {
            assert!(!raw.note.is_empty() && !raw.summary.is_empty(), "{}", raw.id);
            assert!(raw.ap.is_empty(), "{}: a plan of our own steps its secondary stat to its gear", raw.id);
            if let Some(reading) = &raw.reading {
                assert!(reading.url.starts_with("https://meowdb.com/msclassic/guides/"), "{}", raw.id);
                assert!(!reading.url.ends_with("/guides/") && !reading.url.ends_with("/guides"), "not the index page");
            }
        }
        for raw in FILE.plans.iter().filter(|p| p.source.is_some()) {
            assert!(raw.reading.is_none() && raw.note.is_empty(), "{}: its guide says why", raw.id);
        }
    }

    /// A new character splits 25 points with each stat from 4 to 12, and the guides' rolls agree.
    #[test]
    fn every_plan_starts_from_a_lv_1_roll_and_guide_plans_from_their_guides() {
        let checks: HashMap<String, Checks> = serde_json::from_str(include_str!("starters_check.json")).unwrap();
        for raw in &FILE.plans {
            assert_eq!(raw.start.iter().sum::<u32>(), 25, "{}", raw.id);
            assert!(raw.start.iter().all(|&stat| (LOWEST_START..=HIGHEST_START).contains(&stat)), "{}", raw.id);
            if let Some(want) = checks.get(&raw.id) {
                assert_eq!(raw.start, want.start, "{}: the guide's own roll", raw.id);
            }
        }
    }

    /// SP a character has had by the end of `level`, all jobs together.
    fn sp_by(level: u32, rules: &Rules) -> u32 {
        let beginner = level.min(rules.first_job_level).saturating_sub(1) * rules.beginner_sp_per_level;
        let job = |from: u32, to: u32| {
            if level < from { 0 } else { rules.advancement_sp + (level.min(to) - from) * rules.sp_per_level }
        };
        beginner + job(rules.first_job_level, rules.second_job_level) + job(rules.second_job_level, PLAN_TO)
    }

    /// The plans written for Leafline, played from their own roll: skills never take more SP than
    /// there is, each stat step lands by its level, nothing is out of reach and nothing is left
    /// over. With `LEAFLINE_OSMS` set, against the real data: every name resolves, skill caps and
    /// prerequisites hold, and every gear goal's STR, DEX and LUK are met by the plan's own points
    /// at the level the item asks for, before any worn gear helps.
    #[test]
    fn each_plan_written_for_leafline_works_out_level_by_level() {
        use super::super::plan::{Build, Severity, Start, expand};

        let rules = Rules::default();
        let real = real_catalog();
        assert!(real.is_some() || std::env::var_os("LEAFLINE_OSMS").is_none(), "LEAFLINE_OSMS is set but its skills or items didn't load");
        let plans: Vec<&Raw> = FILE.plans.iter().filter(|p| p.source.is_none()).collect();
        assert!(plans.len() >= 3);
        for raw in plans {
            let made_up;
            let catalog = match &real {
                Some(catalog) => catalog,
                None => {
                    made_up = made_up_catalog(raw);
                    &made_up
                }
            };
            let starter = resolve(raw, catalog, &rules);
            if real.is_some() {
                assert!(starter.missing.is_empty(), "{}: not in the data: {:?}", raw.id, starter.missing);
                assert!(!starter.plan.gear.is_empty());
            }
            let start = Start { level: 1, stats: Some(starter.start), ..Start::default() };
            let e = expand(&Build { start, plan: starter.plan, ..Build::default() }, &rules, catalog);
            let warnings: Vec<&str> =
                e.problems.iter().filter(|p| p.severity == Severity::Warning).map(|p| p.message.as_str()).collect();
            assert!(warnings.is_empty(), "{}: {warnings:?}", raw.id);

            let mut spent = 0;
            for step in &e.steps {
                spent += step.sp.iter().map(|gain| gain.to - gain.from).sum::<u32>();
                assert!(spent <= sp_by(step.level, &rules), "{}: {spent} SP spent by Lv {}", raw.id, step.level);
            }
            assert_eq!(spent, sp_by(PLAN_TO, &rules), "{}: every point spent", raw.id);
            assert_eq!(e.steps.last().map(|s| (s.ap_left, s.sp_left)), Some((0, 0)), "{}", raw.id);

            let stats_at = |level: u32| e.steps.iter().find(|s| s.level == level).unwrap().stats;
            for &(target, by) in &raw.steps {
                assert!(stats_at(by).get(raw.secondary) >= target, "{}: {} {target} by Lv {by}", raw.id, raw.secondary.label());
            }
            for check in &e.gear {
                let item = &check.item;
                let s = stats_at(item.req_level.max(1));
                let need = [(s.str, item.req_str), (s.dex, item.req_dex), (s.int, item.req_int), (s.luk, item.req_luk)];
                assert!(need.iter().all(|(have, need)| have >= need), "{}: {} at Lv {}: {s:?}", raw.id, item.name, item.req_level);
                assert_eq!(check.level, Some(item.req_level.max(1)), "{}: {} the level it asks for", raw.id, item.name);
                // Each weapon is one to get, so the level list shows it then.
                if item.slot == "Weapon" {
                    let step = e.steps.iter().find(|s| s.level == item.req_level.max(1)).unwrap();
                    assert!(step.gear.contains(&item.name), "{}: {} not listed at Lv {}: {:?}", raw.id, item.name, step.level, step.gear);
                }
            }
        }
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
            note: String::new(),
            start: [4, 4, 4, 13],
            opening: String::new(),
            skills: Vec::new(),
            primary: Stat::Luk,
            secondary: Stat::Dex,
            steps,
            ap: Vec::new(),
            gear: Vec::new(),
            source: None,
            reading: None,
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
        assert_eq!(found.len(), FILE.plans.len());
        assert!(found[0].plan.skills.is_empty());
        assert!(found[0].missing.contains(&"Three Snails".to_string()));
        assert!(found[0].missing.contains(&"Metal Axe".to_string()));
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
