//! The character as the training estimates see them: their blows, reach, accuracy and attack
//! speed, put together from what the game's windows last showed (see [`super::damage`] and
//! [`super::sheet`]), the buffs marked on, the worn weapon and the skill data.

use tauri::{AppHandle, Manager};

use super::damage::{self, DamageSkill, DamageState};
use super::mapdata::MapDataState;
use super::records::UserDataState;
use super::sheet;
use crate::build::Catalog;
use crate::mapdata::combat::{self, Fighter};
use crate::mapdata::training::{Assumptions, MeasuredRate};
use crate::session::SessionLine;

/// What the player typed or chose over what was read: the attack range and skill to count.
#[derive(Debug, Clone, Default)]
pub struct Choice<'a> {
    pub assumptions: Option<&'a Assumptions>,
    /// The skill the player picked by name; otherwise the hardest-hitting one read.
    pub skill: Option<&'a str>,
}

/// `name`'s fighter at `level` (or the level the stat window showed). `None` until the stat
/// window's attack range, or a spell and the Magic line, has been read or typed.
pub fn fighter(app: &AppHandle, name: &str, level: Option<u32>, choice: &Choice) -> Option<Fighter> {
    let stored = app.state::<DamageState>().get(name);
    let view = sheet::view(app, name, stored.as_ref());
    let catalog = app.state::<MapDataState>().catalog()?;
    let build = app.state::<UserDataState>().with(|db| db.active_build(Some(name))).ok().flatten();
    let booster = view.buffs.iter().any(|b| b.on && b.name.to_lowercase().contains("booster"));
    let weapon = build.as_ref().and_then(|b| {
        b.start.pieces.iter().filter_map(|p| catalog.item(p.item?)).find(|item| item.weapon_type.is_some())
    });
    let sheet = view.buffed.or(view.sheet).unwrap_or_default();
    let skills = stored.as_ref().map(|s| s.skills.as_slice()).unwrap_or_default();
    let read_attack = stored.as_ref().and_then(|s| s.attack_min.zip(s.attack_max));
    let parts = Parts {
        level: level.or(sheet.level)?,
        skills,
        chosen: choice.skill,
        typed: choice.assumptions,
        attack: sheet.attack_min.zip(sheet.attack_max).or(read_attack),
        magic: sheet.magic,
        int: sheet.int.map(|i| i + sheet.gear_int),
        accuracy: sheet.accuracy,
        crit_rate: sheet.crit_rate,
        crit_damage: sheet.crit_damage,
        weapon_type: weapon.and_then(|w| w.weapon_type.as_deref()),
        weapon_speed: weapon.and_then(|w| w.attack_speed),
        booster,
    };
    parts.fighter(&catalog)
}

/// `name`'s saved play as the estimates take it, one entry per stretch with EXP, leaving out the
/// session that started at `except` (the one going on, which is saved as it goes).
pub fn measured(app: &AppHandle, name: &str, except: Option<f64>) -> Vec<MeasuredRate> {
    let sessions = app.state::<UserDataState>().with(|db| db.sessions()).unwrap_or_default();
    sessions
        .iter()
        .filter(|s| s.character.as_deref().is_some_and(|c| c.eq_ignore_ascii_case(name)))
        .filter(|s| except.is_none_or(|at| s.started_at != Some(at)))
        .flat_map(|s| &s.lines)
        .filter(|l| l.map.is_some() && !l.town && l.active_seconds > 0.0 && l.exp_gained > 0)
        .map(rate_of)
        .collect()
}

/// A stretch of play as the estimates take it. Kills count only when nearly every gain matched.
pub fn rate_of(line: &SessionLine) -> MeasuredRate {
    let kills = line.kills.filter(|&k| k > 0 && line.gains_missed * 10 <= k).map(f64::from);
    MeasuredRate {
        label: line.map.clone().unwrap_or_default(),
        exp_per_hour: line.exp_gained as f64 * 3600.0 / line.active_seconds,
        active_seconds: line.active_seconds,
        level: line.level,
        party: line.party,
        kills,
        fighter: line.fighter.as_ref().and_then(|f| serde_json::from_value(f.clone()).ok()),
        deaths: Some(f64::from(line.deaths)),
    }
}

/// What `name` can expect on the map labelled `label` (`Street : Name`) at `level`, from their
/// saved play there and the combat model, for the session's pace to start from. `None` when
/// neither has anything to say.
pub fn prior(app: &AppHandle, name: &str, level: Option<u32>, label: &str, except: Option<f64>) -> Option<f64> {
    let state = app.state::<MapDataState>();
    let atlas = state.atlas()?;
    let layouts = state.layouts();
    let (street, map_name) = label.split_once(" : ").map_or((None, label), |(s, n)| (Some(s), n));
    let found = atlas.find(map_name, street)?;
    let measured = measured(app, name, except);
    let fighter = fighter(app, name, level, &Choice::default());
    let layout = layouts.get(&found.card.map.id);
    match atlas.model(&measured, fighter, Assumptions::default(), &layouts) {
        Some(model) => {
            let (estimate, spread) = model.estimate(&found.card.mobs, layout)?;
            let carried = atlas.carried_play(&measured, &model, &layouts);
            Some(model.forecast(&estimate, spread, carried.get(&found.card.map.id)).exp_per_hour)
        }
        // Without the character's damage, only their own play here near this level says anything.
        None => {
            let near: Vec<&MeasuredRate> = measured
                .iter()
                .filter(|r| r.party.unwrap_or(1) <= 1)
                .filter(|r| level.zip(r.level).is_some_and(|(now, then)| now.abs_diff(then) <= 3))
                .filter(|r| atlas.find(r.label.split_once(" : ").map_or(r.label.as_str(), |(_, n)| n), r.label.split_once(" : ").map(|(s, _)| s)).is_some_and(|m| m.card.map.id == found.card.map.id))
                .collect();
            let seconds: f64 = near.iter().map(|r| r.active_seconds).sum();
            (seconds >= 300.0).then(|| near.iter().map(|r| r.exp_per_hour * r.active_seconds).sum::<f64>() / seconds)
        }
    }
}

/// Everything a fighter is made from, apart from the app, so it can be tested.
struct Parts<'a> {
    level: u32,
    skills: &'a [DamageSkill],
    chosen: Option<&'a str>,
    typed: Option<&'a Assumptions>,
    /// The attack range as read, buffs counted.
    attack: Option<(u32, u32)>,
    magic: Option<u32>,
    int: Option<u32>,
    accuracy: Option<u32>,
    crit_rate: Option<u32>,
    crit_damage: Option<u32>,
    weapon_type: Option<&'a str>,
    weapon_speed: Option<u32>,
    booster: bool,
}

impl Parts<'_> {
    fn fighter(&self, catalog: &Catalog) -> Option<Fighter> {
        let skill = match self.chosen.filter(|n| !n.is_empty() && *n != "custom") {
            Some(name) => self.skills.iter().find(|s| s.name.eq_ignore_ascii_case(name)),
            // A typed factor means no read skill is counted.
            None if self.chosen == Some("custom") => None,
            None => self
                .skills
                .iter()
                .max_by_key(|s| (s.basic_attack.is_none() || self.attack.is_none(), s.percent, s.basic_attack.unwrap_or(0))),
        };
        let record = skill.and_then(|s| catalog.skills().iter().find(|r| r.name.eq_ignore_ascii_case(&s.name)));
        let at = |s: &DamageSkill| s.level.max(1) as usize - 1;
        let effects = skill.zip(record).and_then(|(s, r)| r.effects.get(at(s)).or(r.effects.last()).copied());
        let lines = effects.map_or(1, |e| e.lines.max(1) as u32);
        let targets = effects.map_or(1, |e| e.targets.max(1) as u32);
        let spell = skill.and_then(|s| {
            let base = s.basic_attack?;
            let mastery = record
                .and_then(|r| r.level_descriptions.get(at(s)))
                .and_then(|d| damage::spell(d))
                .map_or(1, |(_, m)| m);
            Some((base, mastery))
        });
        let (low, high, magic) = match spell {
            Some((base, mastery)) => {
                let (low, high) = combat::spell_range(f64::from(base), f64::from(self.magic?), f64::from(self.int.unwrap_or(0)), f64::from(mastery));
                (low, high, true)
            }
            None => {
                let typed = self.typed.and_then(|a| a.attack_min.zip(a.attack_max));
                let (low, high) = typed.or(self.attack.map(|(l, h)| (f64::from(l), f64::from(h))))?;
                let factor = match skill {
                    Some(s) => f64::from(s.percent) / 100.0,
                    None => self.typed.map_or(1.0, |a| a.skill_factor),
                };
                let factor = if factor > 0.0 { factor } else { 1.0 };
                (low * factor, high * factor, false)
            }
        };
        if high <= 0.0 {
            return None;
        }
        let crit_bonus = self.crit_damage.map_or(0.0, |d| if d >= 100 { f64::from(d - 100) } else { f64::from(d) } / 100.0);
        let tier = self.weapon_speed.map(|t| if self.booster { t.saturating_sub(2) } else { t });
        Some(Fighter {
            level: self.level,
            low,
            high,
            lines,
            targets,
            accuracy: if magic { None } else { self.accuracy.map(f64::from) },
            magic,
            crit_rate: self.crit_rate.map_or(0.0, |r| f64::from(r) / 100.0).clamp(0.0, 1.0),
            crit_bonus,
            attack_seconds: combat::attack_seconds(self.weapon_type, tier, magic),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::catalog::tests::skill;
    use crate::mapdata::osms::Effects;

    fn catalog() -> Catalog {
        let mut double = skill(1, "Double Stab", 400, 2, &[]);
        double.level_descriptions = vec!["MP -8; Damage 80%".into(), "MP -9; Damage 90%".into()];
        double.effects = vec![Effects { lines: 2, targets: 1, ..Effects::default() }; 2];
        let mut claw = skill(2, "Magic Claw", 200, 1, &[]);
        claw.level_descriptions = vec!["MP -10, Basic Attack 45, Mastery level 1".into()];
        claw.effects = vec![Effects { lines: 2, targets: 1, ..Effects::default() }];
        Catalog::new(vec![double, claw], Vec::new())
    }

    fn parts<'a>(skills: &'a [DamageSkill]) -> Parts<'a> {
        Parts {
            level: 30,
            skills,
            chosen: None,
            typed: None,
            attack: Some((100, 200)),
            magic: Some(150),
            int: Some(120),
            accuracy: Some(90),
            crit_rate: Some(20),
            crit_damage: Some(55),
            weapon_type: Some("Dagger"),
            weapon_speed: Some(4),
            booster: false,
        }
    }

    fn read(name: &str, level: u32, percent: u32, basic_attack: Option<u32>) -> DamageSkill {
        DamageSkill { name: name.into(), level, percent, basic_attack }
    }

    #[test]
    fn a_thief_hits_twice_at_the_skills_share() {
        let skills = [read("Double Stab", 2, 90, None)];
        let f = parts(&skills).fighter(&catalog()).unwrap();
        assert_eq!((f.low, f.high, f.lines, f.magic), (90.0, 180.0, 2, false));
        assert_eq!(f.accuracy, Some(90.0));
        assert!((f.crit_rate - 0.2).abs() < 1e-9 && (f.crit_bonus - 0.55).abs() < 1e-9);
        assert!((f.attack_seconds - 0.72).abs() < 1e-9);
    }

    #[test]
    fn a_booster_makes_attacks_quicker() {
        let skills = [read("Double Stab", 2, 90, None)];
        let mut p = parts(&skills);
        p.booster = true;
        assert!((p.fighter(&catalog()).unwrap().attack_seconds - 0.60).abs() < 1e-9);
    }

    #[test]
    fn a_spell_works_out_its_own_range_and_never_misses() {
        let skills = [read("Magic Claw", 1, 100, Some(45))];
        let f = parts(&skills).fighter(&catalog()).unwrap();
        let (low, high) = combat::spell_range(45.0, 150.0, 120.0, 1.0);
        assert_eq!((f.low, f.high, f.lines, f.magic, f.accuracy), (low, high, 2, true, None));
    }

    #[test]
    fn typed_numbers_win_over_read_ones() {
        let typed = Assumptions { attack_min: Some(50.0), attack_max: Some(60.0), skill_factor: 2.0, ..Assumptions::default() };
        let mut p = parts(&[]);
        p.typed = Some(&typed);
        p.chosen = Some("custom");
        let f = p.fighter(&catalog()).unwrap();
        assert_eq!((f.low, f.high, f.lines), (100.0, 120.0, 1));
        let mut none = parts(&[]);
        none.attack = None;
        assert!(none.fighter(&catalog()).is_none(), "nothing read or typed");
    }
}
