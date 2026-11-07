//! A build and how its goals play out, level by level.
//!
//! A plan is an ordered list of goals, the way written guides put it: this skill to 20, then
//! that one; DEX to 25, the rest into LUK. [`expand`] walks the levels, gives each new point to
//! the first goal that can take it, and says what cannot work out.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::catalog::{Catalog, path_of, tier_of, usable_by};
use super::rules::{Branch, Rules};
use crate::mapdata::osms::ItemRecord;

/// The highest level a plan can run to.
pub const MAX_LEVEL: u32 = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Stat {
    Str,
    Dex,
    Int,
    Luk,
}

impl Stat {
    pub fn label(self) -> &'static str {
        match self {
            Self::Str => "STR",
            Self::Dex => "DEX",
            Self::Int => "INT",
            Self::Luk => "LUK",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Stats {
    pub str: u32,
    pub dex: u32,
    pub int: u32,
    pub luk: u32,
}

impl Stats {
    pub fn get(&self, stat: Stat) -> u32 {
        match stat {
            Stat::Str => self.str,
            Stat::Dex => self.dex,
            Stat::Int => self.int,
            Stat::Luk => self.luk,
        }
    }

    fn add(&mut self, stat: Stat, amount: u32) {
        match stat {
            Stat::Str => self.str += amount,
            Stat::Dex => self.dex += amount,
            Stat::Int => self.int += amount,
            Stat::Luk => self.luk += amount,
        }
    }

    pub fn plus(self, other: Stats) -> Stats {
        Stats {
            str: self.str + other.str,
            dex: self.dex + other.dex,
            int: self.int + other.int,
            luk: self.luk + other.luk,
        }
    }
}

/// A skill and a level: a goal, or what a character already has.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillLevel {
    pub skill_id: u32,
    pub level: u32,
}

/// Raise `stat` to `target`, or with no target put all the AP left there. Waits for `from_level`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatGoal {
    pub stat: Stat,
    pub target: Option<u32>,
    pub from_level: Option<u32>,
}

/// Where the character is now, as its windows show it. It differs by character, so the player
/// types it in; the rest of the plan counts up from here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Start {
    pub level: u32,
    /// `None` until the player has entered them.
    pub stats: Option<Stats>,
    /// Points earned but not spent yet.
    pub ap: u32,
    pub sp: u32,
    pub skills: Vec<SkillLevel>,
    /// `male` or `female`, for gear that is made for one. `None` until the player says.
    pub gender: Option<String>,
    /// Equipment the character has, each piece as an item check read it, scrolls and all. Only
    /// shown for now: the plan's requirement checks still count the data's stats.
    pub pieces: Vec<Piece>,
}

impl Default for Start {
    fn default() -> Self {
        Self { level: 1, stats: None, ap: 0, sp: 0, skills: Vec::new(), gender: None, pieces: Vec::new() }
    }
}

/// One copy of an item, as its tooltip showed it: its own stats rather than the data's.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Piece {
    /// Tells two copies of an item apart. Given when it is added.
    #[serde(default)]
    pub id: i64,
    /// The data's item, when the name matched one.
    pub item: Option<u32>,
    pub name: String,
    /// What the tooltip calls it, such as `Hat` or `One-handed Sword`.
    pub item_type: Option<String>,
    pub level: Option<u32>,
    pub stats: Vec<PieceStat>,
    /// Successful scrolls, from the name's (+N).
    pub upgrades: Option<u32>,
    pub slots_left: Option<u32>,
    /// The data's upgrade slots for the item, to say how many were used.
    pub slots: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PieceStat {
    pub label: String,
    pub value: i32,
}

/// The part of a build that can be shared: nothing in it is about one character.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Plan {
    /// The first or second job chosen; 0 before there is one.
    pub job: u32,
    pub skills: Vec<SkillLevel>,
    pub stats: Vec<StatGoal>,
    /// Equipment to aim for, by item id.
    pub gear: Vec<u32>,
    pub plan_to: u32,
}

impl Default for Plan {
    fn default() -> Self {
        Self { job: 0, skills: Vec::new(), stats: Vec::new(), gear: Vec::new(), plan_to: 70 }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Build {
    /// 0 until it has been saved.
    pub id: i64,
    pub name: String,
    /// Follows this character when the HUD shows its name.
    pub character: Option<String>,
    /// A link to the guide this build follows, typed by the player.
    pub guide: String,
    pub start: Start,
    pub plan: Plan,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatGain {
    pub stat: Stat,
    pub amount: u32,
    pub total: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillGain {
    pub skill_id: u32,
    pub name: String,
    pub from: u32,
    pub to: u32,
}

/// What happens on reaching a level.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LevelStep {
    pub level: u32,
    /// The job advancement due at this level.
    pub advance: Option<String>,
    pub ap: Vec<StatGain>,
    pub sp: Vec<SkillGain>,
    /// Goal gear this level can first wear.
    pub gear: Vec<String>,
    /// Goal gear worn by now whose stats count toward other gear's requirements.
    pub worn: Vec<Worn>,
    /// Base stats after this level's AP.
    pub stats: Stats,
    /// Points left with nothing to go on.
    pub ap_left: u32,
    pub sp_left: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Worn {
    pub slot: String,
    pub bonus: Stats,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Worth knowing, and nothing is wrong.
    Note,
    /// The plan won't work out as written.
    Warning,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Problem {
    pub severity: Severity,
    pub level: Option<u32>,
    pub message: String,
}

impl Problem {
    fn warning(level: Option<u32>, message: String) -> Self {
        Self { severity: Severity::Warning, level, message }
    }

    fn note(level: Option<u32>, message: String) -> Self {
        Self { severity: Severity::Note, level, message }
    }
}

/// When a goal piece of gear comes within reach.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GearCheck {
    pub item: ItemRecord,
    /// The first level the plan can wear it, if any.
    pub level: Option<u32>,
    pub usable: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Expansion {
    pub steps: Vec<LevelStep>,
    pub problems: Vec<Problem>,
    pub gear: Vec<GearCheck>,
    /// Skill levels when the plan ends.
    pub skills: Vec<SkillLevel>,
}

impl Expansion {
    /// The step for `level`: the last one for a level past the plan, the first for one before it.
    pub fn step(&self, level: u32) -> Option<&LevelStep> {
        self.steps.iter().rev().find(|s| s.level <= level).or(self.steps.first())
    }
}

/// A goal that is still within the catalog and the job path.
struct SkillGoal {
    id: u32,
    level: u32,
    tier: u32,
}

/// SP waiting to be spent. Each kind buys only its own tier's skills, unless the rules let
/// 2nd-job SP buy 1st-job skills too.
#[derive(Default)]
struct SkillPoints {
    beginner: u32,
    first: u32,
    second: u32,
    second_buys_first: bool,
}

impl SkillPoints {
    fn can_buy(&self, tier: u32) -> bool {
        match tier {
            0 => self.beginner > 0,
            1 => self.first > 0 || (self.second_buys_first && self.second > 0),
            _ => self.second > 0,
        }
    }

    fn spend(&mut self, tier: u32) {
        match tier {
            0 => self.beginner -= 1,
            1 if self.first > 0 => self.first -= 1,
            _ => self.second -= 1,
        }
    }

    fn left(&self) -> u32 {
        self.beginner + self.first + self.second
    }
}

/// Lays `build` out from its starting level to the end of the plan.
pub fn expand(build: &Build, rules: &Rules, catalog: &Catalog) -> Expansion {
    let mut problems = Vec::new();
    let branch = Branch::of_job(build.plan.job);
    let start = build.start.level.clamp(1, MAX_LEVEL);
    let end = build.plan.plan_to.clamp(start, MAX_LEVEL);
    let first_job = rules.first_job_level;
    let second_job = rules.second_job_level;
    let path = path_of(build.plan.job);

    let mut skills: HashMap<u32, u32> = build.start.skills.iter().map(|s| (s.skill_id, s.level)).collect();
    let goals = skill_goals(build, catalog, &path, &mut problems);
    check_order(&goals, &skills, catalog, &mut problems);
    problems.extend(start_notes(build, branch, rules));

    let mut stats = build.start.stats.unwrap_or_default();
    let mut ap = build.start.ap;
    let mut sp = SkillPoints { second_buys_first: rules.second_sp_buys_first, ..SkillPoints::default() };
    if start >= second_job {
        sp.second = build.start.sp;
    } else if start >= first_job {
        sp.first = build.start.sp;
    } else {
        sp.beginner = build.start.sp;
    }

    let mut steps = Vec::with_capacity((end - start + 1) as usize);
    for level in start..=end {
        let new_level = level > start;
        let mut advance = None;
        if new_level {
            ap += rules.ap_per_level;
            if level <= first_job {
                sp.beginner += rules.beginner_sp_per_level;
            } else if branch.is_some() && level <= second_job {
                sp.first += rules.sp_per_level;
            } else if branch.is_some() {
                sp.second += rules.sp_per_level;
            }
            if level == first_job {
                advance = Some(format!("1st job: {}", job_name(catalog, path.first().copied())));
                if branch.is_some() {
                    sp.first += rules.advancement_sp;
                }
            }
        }
        let ap_gains = spend_ap(&mut ap, &mut stats, &build.plan.stats, level);
        let mut sp_gains = Vec::new();
        spend_sp(&mut sp, &mut skills, &goals, catalog, &mut sp_gains);
        // This level's SP goes on the old job first, then the advancement leaves it behind.
        if new_level && level == second_job {
            advance = Some(format!("2nd job: {}", job_name(catalog, path.get(1).copied())));
            if sp.first > 0 {
                problems.push(Problem::warning(
                    Some(level),
                    format!("{} 1st-job SP would be lost at Lv {level}: no goal can use them.", sp.first),
                ));
                sp.first = 0;
            }
            if branch.is_some() {
                sp.second += rules.advancement_sp;
                spend_sp(&mut sp, &mut skills, &goals, catalog, &mut sp_gains);
            }
        }
        steps.push(LevelStep {
            level,
            advance,
            ap: ap_gains,
            sp: sp_gains,
            gear: Vec::new(),
            worn: Vec::new(),
            stats,
            ap_left: ap,
            sp_left: sp.left(),
        });
    }

    let gear = check_gear(build, catalog, rules, branch, &mut steps, &mut problems);
    problems.extend(end_notes(build, &goals, &skills, &stats, catalog, end, ap, sp.left()));
    let mut skills: Vec<SkillLevel> = skills.into_iter().map(|(skill_id, level)| SkillLevel { skill_id, level }).collect();
    skills.sort_by_key(|s| s.skill_id);
    Expansion { steps, problems, gear, skills }
}

fn job_name(catalog: &Catalog, job: Option<u32>) -> String {
    job.and_then(|j| catalog.job_name(j)).unwrap_or("not chosen yet").to_string()
}

/// The goals that can be followed. Anything else is reported and left out.
fn skill_goals(build: &Build, catalog: &Catalog, path: &[u32], problems: &mut Vec<Problem>) -> Vec<SkillGoal> {
    let mut goals = Vec::with_capacity(build.plan.skills.len());
    for goal in &build.plan.skills {
        let Some(skill) = catalog.skill(goal.skill_id) else {
            problems.push(Problem::warning(None, format!("A goal names a skill Leafline doesn't know ({}).", goal.skill_id)));
            continue;
        };
        if tier_of(skill.job_id) > 0 && !path.contains(&skill.job_id) {
            problems.push(Problem::warning(None, format!("{} isn't a skill of this job path.", skill.name)));
            continue;
        }
        if goal.level > skill.max_level {
            problems.push(Problem::warning(
                None,
                format!("{} stops at level {}, below the goal of {}.", skill.name, skill.max_level, goal.level),
            ));
        }
        let level = goal.level.min(skill.max_level);
        goals.push(SkillGoal { id: skill.id, level, tier: tier_of(skill.job_id) });
    }
    goals
}

/// Says which goals wait on a prerequisite that comes later in the list or not at all.
fn check_order(goals: &[SkillGoal], have: &HashMap<u32, u32>, catalog: &Catalog, problems: &mut Vec<Problem>) {
    let mut reached = have.clone();
    let mut reported: Vec<(u32, u32)> = Vec::new();
    for (i, goal) in goals.iter().enumerate() {
        let Some(skill) = catalog.skill(goal.id) else { continue };
        for need in &skill.required_skills {
            if reached.get(&need.skill_id).copied().unwrap_or(0) >= need.level || reported.contains(&(goal.id, need.skill_id)) {
                continue;
            }
            reported.push((goal.id, need.skill_id));
            let name = catalog.skill(need.skill_id).map_or("another skill", |s| s.name.as_str());
            let later = goals[i..].iter().any(|g| g.id == need.skill_id && g.level >= need.level);
            problems.push(Problem::warning(
                None,
                if later {
                    format!("{} needs {name} level {} first, which comes later in your list. It waits until then.", skill.name, need.level)
                } else {
                    format!("{} needs {name} level {}, which isn't in your plan.", skill.name, need.level)
                },
            ));
        }
        let level = reached.entry(goal.id).or_default();
        *level = (*level).max(goal.level);
    }
}

fn start_notes(build: &Build, branch: Option<Branch>, rules: &Rules) -> Vec<Problem> {
    let mut notes = Vec::new();
    if branch.is_none() {
        notes.push(Problem::note(None, "Choose a job to plan its skills.".into()));
    } else if path_of(build.plan.job).len() < 2 && build.plan.plan_to >= rules.second_job_level {
        notes.push(Problem::note(None, format!("Choose a 2nd job to plan past Lv {}.", rules.second_job_level - 1)));
    }
    if build.start.stats.is_none() {
        notes.push(Problem::warning(None, "Enter your stats from the Character Stat window, so the totals are right.".into()));
    }
    notes
}

/// Gives each point of AP to the first goal that can use it. A "the rest" goal takes all of it.
fn spend_ap(ap: &mut u32, stats: &mut Stats, goals: &[StatGoal], level: u32) -> Vec<StatGain> {
    let mut gains: Vec<StatGain> = Vec::new();
    for goal in goals {
        if *ap == 0 {
            break;
        }
        if goal.from_level.is_some_and(|from| level < from) {
            continue;
        }
        let amount = match goal.target {
            Some(target) => target.saturating_sub(stats.get(goal.stat)).min(*ap),
            None => *ap,
        };
        if amount == 0 {
            continue;
        }
        *ap -= amount;
        stats.add(goal.stat, amount);
        let total = stats.get(goal.stat);
        match gains.iter_mut().find(|g| g.stat == goal.stat) {
            Some(gain) => {
                gain.amount += amount;
                gain.total = total;
            }
            None => gains.push(StatGain { stat: goal.stat, amount, total }),
        }
    }
    gains
}

/// Spends one SP at a time on the first goal that can take it: one with points of its tier
/// left and its prerequisites met. So a goal that has to wait doesn't hold up those after it.
fn spend_sp(
    sp: &mut SkillPoints,
    skills: &mut HashMap<u32, u32>,
    goals: &[SkillGoal],
    catalog: &Catalog,
    gains: &mut Vec<SkillGain>,
) {
    loop {
        let next = goals.iter().find(|goal| {
            let have = skills.get(&goal.id).copied().unwrap_or(0);
            have < goal.level && sp.can_buy(goal.tier) && needs_met(goal.id, skills, catalog)
        });
        let Some(goal) = next else { return };
        sp.spend(goal.tier);
        let level = skills.entry(goal.id).or_default();
        *level += 1;
        let to = *level;
        match gains.iter_mut().find(|g| g.skill_id == goal.id) {
            Some(gain) => gain.to = to,
            None => {
                let name = catalog.skill(goal.id).map_or_else(String::new, |s| s.name.clone());
                gains.push(SkillGain { skill_id: goal.id, name, from: to - 1, to });
            }
        }
    }
}

fn needs_met(skill: u32, have: &HashMap<u32, u32>, catalog: &Catalog) -> bool {
    catalog.skill(skill).is_some_and(|s| {
        s.required_skills.iter().all(|n| have.get(&n.skill_id).copied().unwrap_or(0) >= n.level)
    })
}

/// What an item adds to the four base stats.
fn bonus_of(item: &ItemRecord) -> Stats {
    let mut bonus = Stats::default();
    for stat in &item.stats {
        let target = match stat.label.as_str() {
            "STR" => Stat::Str,
            "DEX" => Stat::Dex,
            "INT" => Stat::Int,
            "LUK" => Stat::Luk,
            _ => continue,
        };
        bonus.add(target, stat.value.max(0) as u32);
    }
    bonus
}

/// Marks the level each goal item comes within reach, walking up the levels so gear already
/// worn can help with the next piece. A new piece replaces what was in its slot, and an item's
/// own stats never count toward its requirements.
fn check_gear(
    build: &Build,
    catalog: &Catalog,
    rules: &Rules,
    branch: Option<Branch>,
    steps: &mut [LevelStep],
    problems: &mut Vec<Problem>,
) -> Vec<GearCheck> {
    let mut gear = Vec::with_capacity(build.plan.gear.len());
    for &id in &build.plan.gear {
        let Some(item) = catalog.item(id) else {
            problems.push(Problem::warning(None, format!("A gear goal names an item Leafline doesn't know ({id}).")));
            continue;
        };
        let usable = branch.is_none_or(|b| usable_by(item, b));
        if !usable {
            problems.push(Problem::warning(None, format!("{} isn't usable by this job.", item.name)));
        }
        gear.push(GearCheck { item: item.clone(), level: None, usable });
    }

    let known = build.start.stats.is_some();
    let mut worn: Vec<Worn> = Vec::new();
    for step in steps.iter_mut() {
        loop {
            let wearable = gear.iter().position(|check: &GearCheck| {
                let others = worn.iter().filter(|w| w.slot != check.item.slot);
                let stats = others.fold(step.stats, |total, w| total.plus(w.bonus));
                check.usable && check.level.is_none() && meets(&check.item, step.level, &stats, known)
            });
            let Some(i) = wearable else { break };
            let check = &mut gear[i];
            check.level = Some(step.level);
            step.gear.push(check.item.name.clone());
            if rules.gear_counts {
                worn.retain(|w| w.slot != check.item.slot);
                worn.push(Worn { slot: check.item.slot.clone(), bonus: bonus_of(&check.item) });
            }
        }
        step.worn = worn.clone();
    }

    let end = steps.last().map_or(0, |s| s.level);
    for check in gear.iter().filter(|c| c.usable && c.level.is_none()) {
        problems.push(Problem::warning(None, format!("{} is out of reach by Lv {end} on this plan.", check.item.name)));
    }
    gear
}

/// Stats aren't compared until the player has entered theirs.
pub(super) fn meets(item: &ItemRecord, level: u32, stats: &Stats, known: bool) -> bool {
    level >= item.req_level
        && (!known
            || (stats.str >= item.req_str
                && stats.dex >= item.req_dex
                && stats.int >= item.req_int
                && stats.luk >= item.req_luk))
}

#[allow(clippy::too_many_arguments)]
fn end_notes(
    build: &Build,
    goals: &[SkillGoal],
    skills: &HashMap<u32, u32>,
    stats: &Stats,
    catalog: &Catalog,
    end: u32,
    ap: u32,
    sp: u32,
) -> Vec<Problem> {
    let mut notes = Vec::new();
    let mut reported = Vec::new();
    for goal in goals {
        let have = skills.get(&goal.id).copied().unwrap_or(0);
        if have < goal.level && !reported.contains(&goal.id) {
            reported.push(goal.id);
            let name = catalog.skill(goal.id).map_or("A skill", |s| s.name.as_str());
            notes.push(Problem::warning(None, format!("{name} only reaches {have} of {} by Lv {end}.", goal.level)));
        }
    }
    for goal in &build.plan.stats {
        let have = stats.get(goal.stat);
        if let Some(target) = goal.target
            && have < target
        {
            notes.push(Problem::warning(None, format!("{} only reaches {have} of {target} by Lv {end}.", goal.stat.label())));
        }
    }
    if let Some(i) = build.plan.stats.iter().position(|g| g.target.is_none() && g.from_level.is_none())
        && i + 1 < build.plan.stats.len()
    {
        let stat = build.plan.stats[i].stat.label();
        notes.push(Problem::warning(None, format!("Goals after \"{stat} with the rest\" never get AP.")));
    }
    if ap > 0 {
        notes.push(Problem::note(None, format!("{ap} AP left over by Lv {end}. Add a stat goal for them.")));
    }
    if sp > 0 {
        notes.push(Problem::note(None, format!("{sp} SP left over by Lv {end}. Add a skill goal for them.")));
    }
    notes
}

#[cfg(test)]
mod tests {
    use super::super::catalog::tests::{item, skill};
    use super::*;
    use crate::mapdata::osms::ItemStat;

    #[test]
    fn builds_saved_before_pieces_load_with_none() {
        let start: Start = serde_json::from_str(r#"{"level":12,"ap":0,"sp":0,"skills":[]}"#).unwrap();
        assert_eq!(start.level, 12);
        assert!(start.pieces.is_empty());
        let piece: Piece = serde_json::from_str(r#"{"name":"Black Bandana","stats":[{"label":"LUK","value":3}]}"#).unwrap();
        assert_eq!((piece.id, piece.upgrades, piece.stats.len()), (0, None, 1));
    }

    const FIRST: u32 = 400;
    const SECOND: u32 = 420;
    const TOSS: u32 = 1;
    const DODGE: u32 = 2;
    const THROW: u32 = 3;
    const STAB: u32 = 4;
    const FLICK: u32 = 6;

    fn catalog() -> Catalog {
        Catalog::new(
            vec![
                skill(TOSS, "Pebble Toss", FIRST, 20, &[]),
                skill(DODGE, "Quick Dodge", FIRST, 10, &[]),
                skill(THROW, "Twin Throw", FIRST, 20, &[(TOSS, 3)]),
                skill(STAB, "Fern Stab", SECOND, 30, &[]),
                skill(5, "Thorn Whirl", 410, 30, &[]),
                skill(FLICK, "Snail Flick", 0, 3, &[]),
            ],
            vec![item(70, "Fern Dagger", 15, &["Thief"]), item(71, "Twig Wand", 5, &["Magician"])],
        )
    }

    fn goal(skill_id: u32, level: u32) -> SkillLevel {
        SkillLevel { skill_id, level }
    }

    fn stat(stat: Stat, target: Option<u32>) -> StatGoal {
        StatGoal { stat, target, from_level: None }
    }

    /// A thief at Lv 11 with stats entered, before the level's SP. Rules are Leafline's defaults.
    fn build(skills: Vec<SkillLevel>, stats: Vec<StatGoal>, plan_to: u32) -> Build {
        Build {
            name: "Test".into(),
            start: Start { level: 10, stats: Some(Stats { str: 4, dex: 25, int: 4, luk: 40 }), ..Start::default() },
            plan: Plan { job: SECOND, skills, stats, gear: Vec::new(), plan_to },
            ..Build::default()
        }
    }

    fn run(build: &Build) -> Expansion {
        expand(build, &Rules::default(), &catalog())
    }

    fn messages(expansion: &Expansion) -> Vec<&str> {
        expansion.problems.iter().map(|p| p.message.as_str()).collect()
    }

    fn step(expansion: &Expansion, level: u32) -> &LevelStep {
        expansion.steps.iter().find(|s| s.level == level).unwrap()
    }

    #[test]
    fn points_follow_the_goals_in_order() {
        let e = run(&build(vec![goal(TOSS, 4), goal(DODGE, 2)], vec![stat(Stat::Dex, Some(30)), stat(Stat::Luk, None)], 13));
        let at_11 = step(&e, 11);
        assert_eq!(at_11.sp.iter().map(|s| (s.skill_id, s.from, s.to)).collect::<Vec<_>>(), [(TOSS, 0, 3)]);
        assert_eq!(at_11.ap, [StatGain { stat: Stat::Dex, amount: 5, total: 30 }]);

        let at_12 = step(&e, 12);
        assert_eq!(at_12.sp.iter().map(|s| (s.skill_id, s.from, s.to)).collect::<Vec<_>>(), [(TOSS, 3, 4), (DODGE, 0, 2)]);
        assert_eq!(at_12.ap, [StatGain { stat: Stat::Luk, amount: 5, total: 45 }]);
        assert_eq!(e.step(99).unwrap().level, 13, "past the plan is its last step");
    }

    #[test]
    fn the_same_skill_can_come_back_later_in_the_list() {
        let e = run(&build(vec![goal(TOSS, 1), goal(DODGE, 1), goal(TOSS, 2)], vec![stat(Stat::Luk, None)], 11));
        let gains: Vec<_> = step(&e, 11).sp.iter().map(|s| (s.skill_id, s.to)).collect();
        assert_eq!(gains, [(TOSS, 2), (DODGE, 1)]);
    }

    #[test]
    fn first_job_sp_cant_buy_second_job_skills() {
        let e = run(&build(vec![goal(STAB, 2), goal(TOSS, 2)], vec![stat(Stat::Luk, None)], 30));
        assert_eq!(step(&e, 11).sp[0].skill_id, TOSS, "the 2nd-job goal waits for its SP");
        assert_eq!(step(&e, 30).sp[0].skill_id, STAB);
    }

    #[test]
    fn second_job_sp_buys_first_job_skills_only_if_the_rules_say_so() {
        let mut plan = build(vec![goal(THROW, 20)], vec![stat(Stat::Luk, None)], 31);
        plan.start.level = 29;
        plan.start.skills = vec![goal(TOSS, 3)];
        let e = run(&plan);
        assert_eq!(step(&e, 30).sp.iter().map(|s| (s.skill_id, s.to)).collect::<Vec<_>>(), [(THROW, 3)], "Lv 30's own SP is 1st-job");
        assert!(step(&e, 31).sp.is_empty());

        let rules = Rules { second_sp_buys_first: true, ..Rules::default() };
        let e = expand(&plan, &rules, &catalog());
        assert_eq!(step(&e, 31).sp[0].to, 7, "the advancement's point and Lv 31's three");
    }

    #[test]
    fn first_job_sp_runs_to_the_second_job_with_a_point_from_the_advancement() {
        // 1 from the advancement at Lv 10, then 3 a level through Lv 30.
        let mut plan = build(Vec::new(), vec![stat(Stat::Luk, None)], 30);
        plan.start.level = 9;
        let e = run(&plan);
        assert!(messages(&e).contains(&"61 1st-job SP would be lost at Lv 30: no goal can use them."), "{:?}", e.problems);
    }

    #[test]
    fn each_advancement_gives_a_point_at_its_level() {
        let mut plan = build(vec![goal(TOSS, 1), goal(STAB, 1)], vec![stat(Stat::Luk, None)], 30);
        plan.start.level = 9;
        let e = run(&plan);
        assert_eq!(step(&e, 10).sp.iter().map(|s| s.skill_id).collect::<Vec<_>>(), [TOSS]);
        assert_eq!(step(&e, 30).sp.iter().map(|s| s.skill_id).collect::<Vec<_>>(), [STAB]);
    }

    #[test]
    fn beginners_earn_their_own_sp() {
        let mut plan = build(vec![goal(FLICK, 3), goal(TOSS, 1)], vec![stat(Stat::Luk, None)], 10);
        plan.start.level = 1;
        let e = run(&plan);
        assert_eq!(step(&e, 2).sp.iter().map(|s| (s.skill_id, s.to)).collect::<Vec<_>>(), [(FLICK, 1)]);
        assert_eq!(step(&e, 4).sp[0].to, 3);
        assert!(step(&e, 5).sp.is_empty(), "Beginner SP doesn't buy job skills");
        assert_eq!(step(&e, 10).sp_left, 6, "9 by Lv 10, 3 spent");
        assert_eq!(step(&e, 10).sp[0].skill_id, TOSS, "the advancement's point does");
    }

    #[test]
    fn beginner_skills_can_be_planned_before_a_job_is_chosen() {
        let mut plan = build(vec![goal(FLICK, 1)], vec![stat(Stat::Luk, None)], 2);
        plan.start.level = 1;
        plan.plan.job = 0;
        let e = run(&plan);
        assert_eq!(step(&e, 2).sp[0].skill_id, FLICK);
    }

    #[test]
    fn warns_when_first_job_sp_would_be_lost() {
        let e = run(&build(vec![goal(DODGE, 2)], vec![stat(Stat::Luk, None)], 31));
        assert!(messages(&e).contains(&"58 1st-job SP would be lost at Lv 30: no goal can use them."), "{:?}", e.problems);
        assert_eq!(e.problems.iter().find(|p| p.message.contains("lost")).unwrap().level, Some(30));
        assert_eq!(step(&e, 30).sp_left, 1, "only the advancement's SP is left");
    }

    #[test]
    fn job_advancements_show_on_their_levels() {
        let mut plan = build(Vec::new(), vec![stat(Stat::Luk, None)], 31);
        plan.start.level = 8;
        let e = run(&plan);
        assert_eq!(step(&e, 10).advance.as_deref(), Some("1st job: Job 400"));
        assert_eq!(step(&e, 30).advance.as_deref(), Some("2nd job: Job 420"));
    }

    #[test]
    fn prerequisites_hold_a_goal_back_and_say_why() {
        let e = run(&build(vec![goal(THROW, 2), goal(TOSS, 3)], vec![stat(Stat::Luk, None)], 12));
        assert!(messages(&e).contains(&"Twin Throw needs Pebble Toss level 3 first, which comes later in your list. It waits until then."));
        assert_eq!(step(&e, 11).sp[0].skill_id, TOSS);
        assert_eq!(step(&e, 12).sp[0].skill_id, THROW);
        assert_eq!(step(&e, 12).sp[0].to, 2);
    }

    #[test]
    fn a_prerequisite_missing_from_the_plan_is_named() {
        let e = run(&build(vec![goal(THROW, 2)], vec![stat(Stat::Luk, None)], 12));
        assert!(messages(&e).contains(&"Twin Throw needs Pebble Toss level 3, which isn't in your plan."));
        assert!(messages(&e).contains(&"Twin Throw only reaches 0 of 2 by Lv 12."));
    }

    #[test]
    fn a_prerequisite_the_character_has_is_met() {
        let mut plan = build(vec![goal(THROW, 2)], vec![stat(Stat::Luk, None)], 11);
        plan.start.skills = vec![goal(TOSS, 3)];
        let e = run(&plan);
        assert!(e.problems.iter().all(|p| !p.message.contains("needs")), "{:?}", e.problems);
        assert_eq!(step(&e, 11).sp[0].skill_id, THROW);
    }

    #[test]
    fn a_goal_above_the_skills_maximum_stops_there() {
        let e = run(&build(vec![goal(DODGE, 15)], vec![stat(Stat::Luk, None)], 15));
        assert!(messages(&e).contains(&"Quick Dodge stops at level 10, below the goal of 15."));
        assert_eq!(e.skills, [goal(DODGE, 10)]);
    }

    #[test]
    fn skills_outside_the_path_or_the_data_are_reported_and_skipped() {
        let e = run(&build(vec![goal(5, 3), goal(999, 3), goal(TOSS, 1)], vec![stat(Stat::Luk, None)], 11));
        assert!(messages(&e).contains(&"Thorn Whirl isn't a skill of this job path."));
        assert!(messages(&e).contains(&"A goal names a skill Leafline doesn't know (999)."));
        assert_eq!(step(&e, 11).sp[0].skill_id, TOSS);
    }

    #[test]
    fn goals_that_cant_finish_in_time_are_named() {
        let e = run(&build(vec![goal(TOSS, 20)], vec![stat(Stat::Dex, Some(100))], 12));
        assert!(messages(&e).contains(&"Pebble Toss only reaches 6 of 20 by Lv 12."));
        assert!(messages(&e).contains(&"DEX only reaches 35 of 100 by Lv 12."));
    }

    #[test]
    fn a_stat_goal_can_wait_for_a_level() {
        let waiting = StatGoal { stat: Stat::Dex, target: Some(35), from_level: Some(12) };
        let e = run(&build(Vec::new(), vec![waiting, stat(Stat::Luk, None)], 12));
        assert_eq!(step(&e, 11).ap[0].stat, Stat::Luk, "points don't wait for it");
        assert_eq!(step(&e, 12).ap[0].stat, Stat::Dex);
    }

    #[test]
    fn goals_after_the_rest_are_flagged() {
        let e = run(&build(Vec::new(), vec![stat(Stat::Luk, None), stat(Stat::Dex, Some(30))], 12));
        assert!(messages(&e).contains(&"Goals after \"LUK with the rest\" never get AP."));
    }

    #[test]
    fn ap_is_the_players_from_the_first_level_up() {
        let mut plan = build(Vec::new(), vec![stat(Stat::Luk, None)], 2);
        plan.start.level = 1;
        let e = run(&plan);
        assert_eq!(step(&e, 2).ap, [StatGain { stat: Stat::Luk, amount: 5, total: 45 }]);
    }

    #[test]
    fn starting_points_are_spent_first() {
        let mut plan = build(vec![goal(TOSS, 2)], vec![stat(Stat::Dex, Some(27))], 10);
        plan.start.ap = 2;
        plan.start.sp = 2;
        let e = run(&plan);
        assert_eq!(step(&e, 10).ap[0].amount, 2);
        assert_eq!(step(&e, 10).sp[0].to, 2);
    }

    #[test]
    fn starting_stats_are_asked_for_and_items_wait_on_level_alone_until_then() {
        let mut plan = build(Vec::new(), vec![stat(Stat::Luk, None)], 20);
        plan.start.stats = None;
        plan.plan.gear = vec![70];
        let e = run(&plan);
        assert!(messages(&e).contains(&"Enter your stats from the Character Stat window, so the totals are right."));
        assert_eq!(e.gear[0].level, Some(15));
    }

    /// A hat that gives 10 DEX, and a dagger needing 50 DEX. The build has 25 and adds none.
    fn hat_and_dagger(hat_dex: u32) -> (Build, Catalog) {
        let mut hat = item(70, "Fern Hat", 1, &["Thief"]);
        hat.stats = vec![ItemStat { label: "DEX".into(), value: 10 }];
        hat.req_dex = hat_dex;
        let mut dagger = item(71, "Fern Dagger", 1, &["Thief"]);
        dagger.slot = "Weapon".into();
        dagger.req_dex = 35;
        let mut plan = build(Vec::new(), vec![stat(Stat::Luk, None)], 12);
        plan.plan.gear = vec![71, 70];
        (plan, Catalog::new(vec![skill(TOSS, "Pebble Toss", FIRST, 20, &[])], vec![hat, dagger]))
    }

    #[test]
    fn other_gear_counts_toward_a_requirement_and_its_own_doesnt() {
        let (plan, catalog) = hat_and_dagger(0);
        let e = expand(&plan, &Rules::default(), &catalog);
        assert_eq!((e.gear[0].level, e.gear[1].level), (Some(10), Some(10)), "the hat brings the dagger in reach");
        assert_eq!(step(&e, 10).gear, ["Fern Hat", "Fern Dagger"]);

        // Needing more than base stats, the hat can't lift itself.
        let (plan, catalog) = hat_and_dagger(30);
        let e = expand(&plan, &Rules::default(), &catalog);
        assert_eq!(e.gear[1].level, None);
        assert_eq!(e.gear[0].level, None);
    }

    #[test]
    fn gear_can_be_left_out_of_requirements_by_the_rules() {
        let (plan, catalog) = hat_and_dagger(0);
        let rules = Rules { gear_counts: false, ..Rules::default() };
        let e = expand(&plan, &rules, &catalog);
        assert_eq!((e.gear[0].level, e.gear[1].level), (None, Some(10)));
        assert!(step(&e, 12).worn.is_empty());
    }

    #[test]
    fn a_new_piece_replaces_what_was_in_its_slot() {
        let (mut plan, _) = hat_and_dagger(0);
        let mut old = item(70, "Old Hat", 1, &["Thief"]);
        old.stats = vec![ItemStat { label: "DEX".into(), value: 10 }];
        let mut new = item(72, "New Hat", 11, &["Thief"]);
        new.stats = vec![ItemStat { label: "DEX".into(), value: 2 }];
        plan.plan.gear = vec![70, 72];
        let catalog = Catalog::new(Vec::new(), vec![old, new]);
        let e = expand(&plan, &Rules::default(), &catalog);
        assert_eq!(step(&e, 10).worn[0].bonus.dex, 10);
        assert_eq!(step(&e, 11).worn.len(), 1);
        assert_eq!(step(&e, 11).worn[0].bonus.dex, 2);
    }

    #[test]
    fn gear_is_marked_where_the_plan_can_wear_it() {
        let mut items = vec![item(70, "Fern Dagger", 15, &["Thief"])];
        items[0].req_dex = 40;
        let catalog = Catalog::new(vec![skill(TOSS, "Pebble Toss", FIRST, 20, &[])], items);
        let mut plan = build(Vec::new(), vec![stat(Stat::Dex, None)], 20);
        plan.plan.gear = vec![70];
        let e = expand(&plan, &Rules::default(), &catalog);
        assert_eq!(e.gear[0].level, Some(15), "Lv 13 has the DEX, Lv 15 the level");
        assert_eq!(step(&e, 15).gear, ["Fern Dagger"]);
    }

    #[test]
    fn gear_out_of_reach_or_for_another_job_is_flagged() {
        let mut plan = build(Vec::new(), vec![stat(Stat::Luk, None)], 14);
        plan.plan.gear = vec![70, 71];
        let e = run(&plan);
        assert!(messages(&e).contains(&"Fern Dagger is out of reach by Lv 14 on this plan."));
        assert!(messages(&e).contains(&"Twig Wand isn't usable by this job."));
        assert!(!e.gear[1].usable);
    }

    #[test]
    fn rule_changes_change_the_plan() {
        let rules = Rules { ap_per_level: 4, sp_per_level: 1, ..Rules::default() };
        let e = expand(&build(vec![goal(TOSS, 5)], vec![stat(Stat::Luk, None)], 11), &rules, &catalog());
        assert_eq!(step(&e, 11).ap[0].amount, 4);
        assert_eq!(step(&e, 11).sp[0].to, 1);
    }

    #[test]
    fn with_no_job_nothing_earns_sp_but_stats_still_count() {
        let mut plan = build(Vec::new(), vec![stat(Stat::Luk, None)], 12);
        plan.plan.job = 0;
        let e = run(&plan);
        assert!(messages(&e).contains(&"Choose a job to plan its skills."));
        assert_eq!(step(&e, 12).sp_left, 0);
        assert_eq!(step(&e, 12).stats.luk, 50);
    }

    #[test]
    fn reads_and_writes_the_names_the_overlay_uses() {
        // As src/lib/types.ts writes a build. A renamed field here would lose the player's goals.
        let json = r#"{"id": 3, "name": "Mine", "character": null, "guide": "",
            "start": {"level": 12, "stats": {"str": 4, "dex": 30, "int": 4, "luk": 12}, "ap": 1, "sp": 2,
                      "skills": [{"skillId": 1, "level": 3}]},
            "plan": {"job": 420, "skills": [{"skillId": 2, "level": 5}],
                     "stats": [{"stat": "luk", "target": null, "fromLevel": 15}], "gear": [70], "planTo": 13}}"#;
        let build: Build = serde_json::from_str(json).unwrap();
        assert_eq!(build.start.skills, [goal(1, 3)]);
        assert_eq!(build.plan.stats[0].from_level, Some(15));
        assert_eq!(build.plan.plan_to, 13);

        let written = serde_json::to_value(run(&build)).unwrap();
        let step = &written["steps"][1];
        for key in ["level", "advance", "ap", "sp", "gear", "stats", "apLeft", "spLeft"] {
            assert!(step.get(key).is_some(), "{key} is missing from {step}");
        }
        assert_eq!(step["sp"][0]["skillId"], 2);
        assert_eq!(written["problems"][0]["severity"], "warning");
        assert_eq!(written["gear"][0]["item"]["reqLevel"], 15);
    }

    #[test]
    fn the_end_of_a_plan_says_what_is_unplanned() {
        let e = run(&build(vec![goal(DODGE, 1)], Vec::new(), 12));
        assert!(messages(&e).contains(&"10 AP left over by Lv 12. Add a stat goal for them."));
        assert!(messages(&e).contains(&"5 SP left over by Lv 12. Add a skill goal for them."));
    }
}
