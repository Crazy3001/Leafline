//! The skills and equipment a build can use, from the downloaded data. Jobs are worked out from
//! the skills, so nothing about them is written into Leafline.

use std::collections::HashMap;

use serde::Serialize;

use super::plan::Stat;
use super::rules::Branch;
use crate::mapdata::osms::{ItemRecord, SkillRecord};

/// The most gear a search returns.
const SEARCH_LIMIT: usize = 40;

/// A job a player can choose.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobChoice {
    pub id: u32,
    pub name: String,
    /// 1 for a first job, 2 for a second.
    pub tier: u32,
    /// The first job a second job comes from.
    pub parent: Option<u32>,
    /// The stat a new build starts by putting AP into. A place to start, not advice.
    pub primary: Option<Stat>,
}

/// 0 for Beginner, 1 for a first job such as 400, 2 for a second such as 420.
pub fn tier_of(job: u32) -> u32 {
    match job {
        0 => 0,
        j if j.is_multiple_of(100) => 1,
        _ => 2,
    }
}

/// The first job and, if one is chosen, the second job of `job`.
pub fn path_of(job: u32) -> Vec<u32> {
    match tier_of(job) {
        0 => Vec::new(),
        1 => vec![job],
        _ => vec![job / 100 * 100, job],
    }
}

#[derive(Debug, Default)]
pub struct Catalog {
    skills: Vec<SkillRecord>,
    items: Vec<ItemRecord>,
    skill_index: HashMap<u32, usize>,
    item_index: HashMap<u32, usize>,
}

impl Catalog {
    pub fn new(skills: Vec<SkillRecord>, items: Vec<ItemRecord>) -> Self {
        let skill_index = skills.iter().enumerate().map(|(i, s)| (s.id, i)).collect();
        let item_index = items.iter().enumerate().map(|(i, s)| (s.id, i)).collect();
        Self { skills, items, skill_index, item_index }
    }

    pub fn skill(&self, id: u32) -> Option<&SkillRecord> {
        self.skills.get(*self.skill_index.get(&id)?)
    }

    pub fn item(&self, id: u32) -> Option<&ItemRecord> {
        self.items.get(*self.item_index.get(&id)?)
    }

    pub fn skills(&self) -> &[SkillRecord] {
        &self.skills
    }

    pub fn job_name(&self, job: u32) -> Option<&str> {
        self.skills.iter().find(|s| s.job_id == job).map(|s| s.class_name.as_str())
    }

    /// Every job with skills, first jobs before the second jobs that come from them.
    pub fn jobs(&self) -> Vec<JobChoice> {
        let mut jobs: Vec<JobChoice> = Vec::new();
        for skill in self.skills.iter().filter(|s| tier_of(s.job_id) > 0) {
            if !jobs.iter().any(|j| j.id == skill.job_id) {
                let tier = tier_of(skill.job_id);
                let parent = (tier == 2).then_some(skill.job_id / 100 * 100);
                let primary = Branch::of_job(skill.job_id).map(Branch::primary);
                jobs.push(JobChoice { id: skill.job_id, name: skill.class_name.clone(), tier, parent, primary });
            }
        }
        jobs.sort_by_key(|j| j.id);
        jobs
    }

    /// Equipment whose name holds `text`, lowest level first. With a branch, only what it can use.
    pub fn search_gear(&self, text: &str, branch: Option<Branch>) -> Vec<ItemRecord> {
        let text = text.trim().to_lowercase();
        let mut found: Vec<&ItemRecord> = self
            .items
            .iter()
            .filter(|i| i.name.to_lowercase().contains(&text))
            .filter(|i| branch.is_none_or(|b| usable_by(i, b)))
            .collect();
        found.sort_by(|a, b| (a.req_level, &a.name).cmp(&(b.req_level, &b.name)));
        found.into_iter().take(SEARCH_LIMIT).cloned().collect()
    }
}

/// Whether `branch` can use the item, going by the data.
pub fn usable_by(item: &ItemRecord, branch: Branch) -> bool {
    item.classes.is_empty() || item.classes.iter().any(|c| c.eq_ignore_ascii_case(branch.name()))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::mapdata::osms::SkillNeed;

    /// A made-up skill.
    pub fn skill(id: u32, name: &str, job_id: u32, max_level: u32, needs: &[(u32, u32)]) -> SkillRecord {
        SkillRecord {
            id,
            name: name.into(),
            job_id,
            class_name: format!("Job {job_id}"),
            job_tier: String::new(),
            max_level,
            passive: false,
            description: None,
            required_skills: needs.iter().map(|&(skill_id, level)| SkillNeed { skill_id, level }).collect(),
            level_descriptions: Vec::new(),
        }
    }

    /// A made-up piece of equipment.
    pub fn item(id: u32, name: &str, req_level: u32, classes: &[&str]) -> ItemRecord {
        ItemRecord {
            id,
            name: name.into(),
            category: "Equipment".into(),
            slot: "Hat".into(),
            gender: None,
            classes: classes.iter().map(|c| c.to_string()).collect(),
            weapon_type: None,
            req_level,
            req_str: 0,
            req_dex: 0,
            req_int: 0,
            req_luk: 0,
            stats: Vec::new(),
        }
    }

    #[test]
    fn jobs_come_from_the_skills() {
        let catalog = Catalog::new(
            vec![skill(1, "A", 0, 3, &[]), skill(2, "B", 410, 5, &[]), skill(3, "C", 400, 5, &[]), skill(4, "D", 410, 5, &[])],
            Vec::new(),
        );
        let jobs = catalog.jobs();
        assert_eq!(jobs.iter().map(|j| j.id).collect::<Vec<_>>(), [400, 410], "no Beginner, no repeats");
        assert_eq!((jobs[0].tier, jobs[0].parent), (1, None));
        assert_eq!((jobs[1].tier, jobs[1].parent), (2, Some(400)));
    }

    #[test]
    fn a_job_path_starts_at_the_first_job() {
        assert!(path_of(0).is_empty());
        assert_eq!(path_of(400), [400]);
        assert_eq!(path_of(420), [400, 420]);
    }

    #[test]
    fn gear_search_keeps_to_what_the_branch_can_use() {
        let catalog = Catalog::new(
            Vec::new(),
            vec![item(1, "Fern Hat", 20, &[]), item(2, "Fern Dagger", 10, &["Thief"]), item(3, "Fern Wand", 5, &["Magician"])],
        );
        let names = |found: Vec<ItemRecord>| found.into_iter().map(|i| i.name).collect::<Vec<_>>();
        assert_eq!(names(catalog.search_gear("fern", Some(Branch::Thief))), ["Fern Dagger", "Fern Hat"]);
        assert_eq!(names(catalog.search_gear("  WAND ", None)), ["Fern Wand"]);
    }
}
