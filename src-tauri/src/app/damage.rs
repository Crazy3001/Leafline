//! What the player's blows do, as the game shows it: the attack range from the stat window's detail
//! pane, and each damaging skill's level and damage from the skill window and the skill data. The
//! training estimates use these to count a monster's weapon defence against the player's damage.
//! Kept per character, so a read of one character doesn't change another's.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

use super::mapdata::MapDataState;
use super::records::now;
use super::sheet::{SkillSeen, StatSheet};
use crate::build::Catalog;
use crate::mapdata::osms::SkillRecord;
use crate::error::{Error, Result};
use crate::pipeline::CharacterScan;
use crate::readers::character::SkillWindow;

const FILE_NAME: &str = "damage.json";

/// Emitted to every window when a character's damage was read anew and differs.
pub const DAMAGE_EVENT: &str = "player-damage";

/// A skill name read from the window counts as this one when they are this alike.
const MIN_SKILL_SCORE: f64 = 0.8;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PlayerDamage {
    /// The Attack line, `107 - 289`.
    pub attack_min: Option<u32>,
    pub attack_max: Option<u32>,
    /// The damaging skills last seen in the skill window.
    pub skills: Vec<DamageSkill>,
    /// Everything the stat window and its detail pane showed, and when they were last read.
    pub sheet: Option<StatSheet>,
    pub read_at: Option<i64>,
    /// Every skill the skill window has shown, at the level it showed.
    pub skill_levels: Vec<SkillSeen>,
    /// The buffs the player marked as on, by skill id.
    pub buffs_on: Vec<u32>,
    /// The buffs marked as on when the stat window was last read, already in its numbers.
    pub read_with: Vec<u32>,
}

/// A skill that hits for a share of the player's attack, at the level it was read.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DamageSkill {
    pub name: String,
    pub level: u32,
    /// Its damage as the skill data gives it at that level: 240 is 240%.
    pub percent: u32,
}

#[derive(Debug, Clone, Serialize)]
struct Changed<'a> {
    name: &'a str,
    damage: &'a PlayerDamage,
}

pub struct DamageState {
    path: PathBuf,
    all: Mutex<HashMap<String, PlayerDamage>>,
}

impl DamageState {
    /// A missing or unreadable file means nothing has been read yet.
    pub fn load(app: &AppHandle) -> Result<Self> {
        let path = app.path().app_data_dir()?.join(FILE_NAME);
        let all = std::fs::read_to_string(&path).ok().and_then(|json| serde_json::from_str(&json).ok()).unwrap_or_default();
        Ok(Self { path, all: Mutex::new(all) })
    }

    /// What is stored for `name`.
    pub fn get(&self, name: &str) -> Option<PlayerDamage> {
        self.all.lock().expect("damage lock").get(&name.to_lowercase()).cloned()
    }

    fn save(&self, all: &HashMap<String, PlayerDamage>) -> Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_string(all).map_err(|err| Error::Invalid(err.to_string()))?;
        std::fs::write(&self.path, json)?;
        Ok(())
    }
}

/// The damage a skill level's description gives: `MP -14; Damage 240%` is 240.
fn percent(description: &str) -> Option<u32> {
    let rest = &description[description.find("Damage ")? + "Damage ".len()..];
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    (rest[digits.len()..].starts_with('%')).then(|| digits.parse().ok()).flatten()
}

/// The skill a name read from the window stands for, among those `wanted`.
fn skill_named<'a>(catalog: &'a Catalog, read: &str, wanted: impl Fn(&SkillRecord) -> bool) -> Option<&'a SkillRecord> {
    catalog
        .skills()
        .iter()
        .filter(|s| wanted(s))
        .map(|s| (strsim::normalized_levenshtein(&s.name.to_lowercase(), &read.to_lowercase()), s))
        .filter(|(score, _)| *score >= MIN_SKILL_SCORE)
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, skill)| skill)
}

/// Every skill in `window` the data knows, at the level read, unlearned ones too.
fn seen_skills(window: &SkillWindow, catalog: &Catalog) -> Vec<SkillSeen> {
    let mut found: Vec<SkillSeen> = Vec::new();
    for row in &window.rows {
        let Some(skill) = skill_named(catalog, &row.name, |_| true) else { continue };
        if !found.iter().any(|f| f.name == skill.name) {
            found.push(SkillSeen { name: skill.name.clone(), level: row.level });
        }
    }
    found
}

/// Adds the skills of one page of the skill window to those already known.
fn merge_seen(known: &mut Vec<SkillSeen>, read: Vec<SkillSeen>) {
    for skill in read {
        match known.iter_mut().find(|k| k.name == skill.name) {
            Some(existing) => *existing = skill,
            None => known.push(skill),
        }
    }
}

/// The skills in `window` that hit for a share of the attack, each at the level read.
fn damaging_skills(window: &SkillWindow, catalog: &Catalog) -> Vec<DamageSkill> {
    let mut found: Vec<DamageSkill> = Vec::new();
    for row in window.rows.iter().filter(|r| r.level > 0) {
        let Some(skill) = skill_named(catalog, &row.name, |s| !s.passive) else { continue };
        let level = row.level.min(skill.level_descriptions.len() as u32).max(1) as usize;
        let Some(percent) = skill.level_descriptions.get(level - 1).and_then(|d| percent(d)) else { continue };
        if !found.iter().any(|f| f.name == skill.name) {
            found.push(DamageSkill { name: skill.name.clone(), level: row.level, percent });
        }
    }
    found
}

/// Adds what a read of the skill window showed to the skills already known. The window shows a few
/// skills at a time, so one read is only the page in view: skills not on it stay as they were.
fn merge(known: &mut Vec<DamageSkill>, read: Vec<DamageSkill>) {
    for skill in read {
        match known.iter_mut().find(|k| k.name == skill.name) {
            Some(existing) => *existing = skill,
            None => known.push(skill),
        }
    }
}

/// Takes in what a read of the character's windows shows of the damage, when it shows any. Best
/// effort: a failure to save is logged and the read goes on.
pub fn note(app: &AppHandle, scan: &CharacterScan) {
    let Some(name) = scan.name.as_deref().filter(|n| !n.trim().is_empty()) else { return };
    let (detail, skills) = (scan.windows.detail, scan.windows.skills.as_ref());
    if detail.is_none() && skills.is_none() && scan.windows.stats.is_none() {
        return;
    }
    let catalog = app.state::<MapDataState>().catalog();
    let state = app.state::<DamageState>();
    let mut all = state.all.lock().expect("damage lock");
    let entry = all.entry(name.to_lowercase()).or_default();
    let before = entry.clone();
    if let Some(detail) = detail {
        entry.attack_min = Some(detail.attack_min);
        entry.attack_max = Some(detail.attack_max);
    }
    let stats = scan.windows.stats.as_ref();
    if stats.is_some() || detail.is_some() {
        let sheet = entry.sheet.get_or_insert_default();
        if let Some(stats) = stats {
            sheet.take_stats(stats);
        }
        if let Some(detail) = &detail {
            sheet.take_detail(detail);
        }
        entry.read_at = Some(now());
        // The window's numbers already count whatever was on as it was read.
        entry.read_with.clone_from(&entry.buffs_on);
    }
    if let (Some(window), Some(catalog)) = (skills, catalog) {
        merge(&mut entry.skills, damaging_skills(window, &catalog));
        merge_seen(&mut entry.skill_levels, seen_skills(window, &catalog));
    }
    if *entry == before {
        return;
    }
    let damage = entry.clone();
    if let Err(err) = state.save(&all) {
        eprintln!("could not save the damage read: {err}");
    }
    drop(all);
    let _ = app.emit(DAMAGE_EVENT, Changed { name, damage: &damage });
}

/// What was last read of this character's damage.
#[tauri::command]
pub fn player_damage(state: State<DamageState>, name: String) -> Option<PlayerDamage> {
    state.get(&name)
}

/// Marks one of `name`'s buffs as on or off, and tells every window.
pub fn set_buff(app: &AppHandle, name: &str, skill: u32, on: bool) -> Result<()> {
    let name = name.trim();
    if name.is_empty() {
        return Err(Error::Invalid("The character's name isn't known yet.".into()));
    }
    let state = app.state::<DamageState>();
    let mut all = state.all.lock().expect("damage lock");
    let entry = all.entry(name.to_lowercase()).or_default();
    entry.buffs_on.retain(|&id| id != skill);
    if on {
        entry.buffs_on.push(skill);
    }
    let damage = entry.clone();
    state.save(&all)?;
    drop(all);
    let _ = app.emit(DAMAGE_EVENT, Changed { name, damage: &damage });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_skill_level_gives_its_damage_percent() {
        assert_eq!(percent("MP -14; Damage 240%"), Some(240));
        assert_eq!(percent("HP -16; MP -14; Damage 130%; Range +150"), Some(130));
        assert_eq!(percent("Critical Rate +20%, Critical Damage +15%"), None);
        assert_eq!(percent("Damage 240"), None);
        assert_eq!(percent("Fires an arrow"), None);
    }

    fn catalog() -> Catalog {
        use crate::build::catalog::tests::skill;
        let levelled = |id, name: &str, passive: bool, descriptions: &[&str]| {
            let mut s = skill(id, name, 300, descriptions.len() as u32, &[]);
            s.passive = passive;
            s.level_descriptions = descriptions.iter().map(|d| d.to_string()).collect();
            s
        };
        Catalog::new(
            vec![
                levelled(1, "Arrow Blow", false, &["MP -6; Damage 160%", "MP -7; Damage 165%", "MP -8; Damage 170%"]),
                levelled(2, "Double Shot", false, &["MP -8; Damage 80%"]),
                levelled(3, "Critical Shot", true, &["Critical Rate +5%, Critical Damage +1%"]),
                levelled(4, "Focus", false, &["MP -10; Accuracy +5%"]),
            ],
            Vec::new(),
        )
    }

    #[test]
    fn a_read_of_another_page_of_skills_adds_to_what_is_known() {
        let skill = |name: &str, level, percent| DamageSkill { name: name.into(), level, percent };
        let mut known = vec![skill("Arrow Blow", 20, 240)];
        merge(&mut known, vec![skill("Iron Arrow", 1, 120)]);
        assert_eq!(known, vec![skill("Arrow Blow", 20, 240), skill("Iron Arrow", 1, 120)]);
        // Seen again at a higher level, a skill is updated in place.
        merge(&mut known, vec![skill("Iron Arrow", 2, 125)]);
        assert_eq!(known, vec![skill("Arrow Blow", 20, 240), skill("Iron Arrow", 2, 125)]);
    }

    #[test]
    fn the_skill_window_gives_each_damaging_skill_at_its_level() {
        use crate::readers::character::SkillRow;
        let row = |name: &str, level| SkillRow { name: name.into(), level };
        let window = SkillWindow {
            rows: vec![row("Critical Shot", 1), row("Focus", 1), row("Arrow Blow", 3), row("Double Shot", 0), row("Arrow B1ow", 2)],
            skill_points: None,
        };
        let found = damaging_skills(&window, &catalog());
        // Passive, not-damaging and unlearned skills are left out; a misread name still matches
        // its skill once, and the first reading of it stands.
        assert_eq!(found, vec![DamageSkill { name: "Arrow Blow".into(), level: 3, percent: 170 }]);
    }
}
