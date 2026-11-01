//! Moves a build's starting point to where the character really is, from its windows. The plan
//! then lays out what is left from there, so points spent early, late or elsewhere are simply
//! taken into account.

use serde::Serialize;

use super::catalog::{Catalog, path_of};
use super::plan::{SkillLevel, Start, Stats};
use super::rules::{Branch, Rules};
use crate::readers::character::CharacterReading;

/// How closely a skill name read from the window must match one in the data.
const MIN_SKILL_SCORE: f64 = 0.8;

/// What changed, for the player to see.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatchUp {
    pub changes: Vec<String>,
    /// Skills in the window that aren't on this build's job path, or didn't read.
    pub unknown: Vec<String>,
}

fn note(changes: &mut Vec<String>, label: &str, from: u32, to: u32) {
    if from != to {
        changes.push(format!("{label} {from} → {to}"));
    }
}

/// Brings `start` up to `level` and what the windows show. A window that wasn't read leaves its
/// side as it was, plus the points the levels gained since should have brought.
pub fn catch_up(
    start: &mut Start,
    level: Option<u32>,
    seen: &CharacterReading,
    job: u32,
    rules: &Rules,
    catalog: &Catalog,
) -> CatchUp {
    let mut result = CatchUp::default();
    let level = seen.stats.map(|s| s.level).or(level).unwrap_or(start.level);
    let earned_ap = level.saturating_sub(start.level) * rules.ap_per_level;
    let earned_sp = rules.sp_earned(start.level, level, Branch::of_job(job).is_some());
    note(&mut result.changes, "Lv", start.level, level);
    start.level = level;

    match seen.stats {
        Some(window) => {
            let now = Stats { str: window.str, dex: window.dex, int: window.int, luk: window.luk };
            let was = start.stats.unwrap_or_default();
            for (label, from, to) in
                [("STR", was.str, now.str), ("DEX", was.dex, now.dex), ("INT", was.int, now.int), ("LUK", was.luk, now.luk)]
            {
                note(&mut result.changes, label, from, to);
            }
            start.stats = Some(now);
            note(&mut result.changes, "Unspent AP", start.ap, window.ability_points);
            start.ap = window.ability_points;
        }
        None => start.ap += earned_ap,
    }

    match &seen.skills {
        Some(window) => {
            for row in &window.rows {
                let Some(skill) = skill_called(catalog, &row.name, job) else {
                    result.unknown.push(row.name.clone());
                    continue;
                };
                let have = start.skills.iter().find(|s| s.skill_id == skill.0).map_or(0, |s| s.level);
                note(&mut result.changes, &skill.1, have, row.level);
                start.skills.retain(|s| s.skill_id != skill.0);
                if row.level > 0 {
                    start.skills.push(SkillLevel { skill_id: skill.0, level: row.level });
                }
            }
            match window.skill_points {
                Some(sp) => {
                    note(&mut result.changes, "Unspent SP", start.sp, sp);
                    start.sp = sp;
                }
                None => start.sp += earned_sp,
            }
        }
        None => start.sp += earned_sp,
    }
    result
}

/// The skill of this job path, Beginner skills included, whose name is closest to `name`.
fn skill_called(catalog: &Catalog, name: &str, job: u32) -> Option<(u32, String)> {
    let mut path = path_of(job);
    path.push(0);
    catalog
        .skills()
        .iter()
        .filter(|s| path.contains(&s.job_id))
        .map(|s| (strsim::normalized_levenshtein(&s.name.to_lowercase(), &name.to_lowercase()), s))
        .filter(|(score, _)| *score >= MIN_SKILL_SCORE)
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, s)| (s.id, s.name.clone()))
}

#[cfg(test)]
mod tests {
    use super::super::catalog::tests::skill;
    use super::*;
    use crate::readers::character::{SkillRow, SkillWindow, StatWindow};

    const SNAILS: u32 = 1;
    const STAB: u32 = 2;

    fn catalog() -> Catalog {
        Catalog::new(
            vec![skill(SNAILS, "Three Snails", 0, 3, &[]), skill(STAB, "Fern Stab", 400, 20, &[]), skill(3, "Thin Mist", 200, 20, &[])],
            Vec::new(),
        )
    }

    fn stats(level: u32, luk: u32, ability_points: u32) -> StatWindow {
        StatWindow { level, str: 4, dex: 5, int: 4, luk, ability_points }
    }

    fn start() -> Start {
        Start { level: 1, stats: Some(Stats { str: 4, dex: 5, int: 4, luk: 12 }), ..Start::default() }
    }

    fn skills(rows: &[(&str, u32)], skill_points: Option<u32>) -> SkillWindow {
        let rows = rows.iter().map(|(name, level)| SkillRow { name: (*name).into(), level: *level }).collect();
        SkillWindow { rows, skill_points }
    }

    fn caught_up(start: &mut Start, level: Option<u32>, stats: Option<StatWindow>, skills: Option<SkillWindow>) -> CatchUp {
        let seen = CharacterReading { stats, skills };
        catch_up(start, level, &seen, 400, &Rules::default(), &catalog())
    }

    #[test]
    fn the_windows_replace_what_the_build_assumed() {
        let mut start = start();
        let done = caught_up(&mut start, None, Some(stats(2, 17, 0)), Some(skills(&[("Three Snails", 1)], Some(0))));
        assert_eq!((start.level, start.ap, start.sp), (2, 0, 0));
        assert_eq!(start.stats.unwrap().luk, 17);
        assert_eq!(start.skills, [SkillLevel { skill_id: SNAILS, level: 1 }]);
        assert_eq!(done.changes, ["Lv 1 → 2", "LUK 12 → 17", "Three Snails 0 → 1"]);
    }

    #[test]
    fn a_window_that_wasnt_open_keeps_its_side_and_gains_the_levels_points() {
        let mut start = Start { ap: 1, sp: 1, ..start() };
        caught_up(&mut start, None, Some(stats(3, 22, 0)), None);
        assert_eq!((start.ap, start.sp), (0, 3), "2 levels of Beginner SP, on top of the 1 waiting");
        let mut start = Start { level: 3, ..self::start() };
        caught_up(&mut start, Some(5), None, Some(skills(&[("Three Snails", 3)], Some(0))));
        assert_eq!((start.level, start.ap, start.sp), (5, 10, 0));
        assert_eq!(start.stats.unwrap().luk, 12);
    }

    #[test]
    fn a_skill_back_at_zero_is_taken_off_and_a_misread_name_still_matches() {
        let mut start = Start { skills: vec![SkillLevel { skill_id: STAB, level: 4 }], ..start() };
        caught_up(&mut start, None, None, Some(skills(&[("Fern Stab", 0), ("Three Snaiis", 2)], None)));
        assert_eq!(start.skills, [SkillLevel { skill_id: SNAILS, level: 2 }]);
    }

    #[test]
    fn other_jobs_skills_are_reported_not_stored() {
        let mut start = start();
        let done = caught_up(&mut start, None, None, Some(skills(&[("Thin Mist", 5)], None)));
        assert!(start.skills.is_empty());
        assert_eq!(done.unknown, ["Thin Mist"]);
    }

    #[test]
    fn nothing_read_changes_nothing() {
        let mut start = start();
        let done = caught_up(&mut start, None, None, None);
        assert_eq!(start, self::start());
        assert!(done.changes.is_empty());
    }
}
