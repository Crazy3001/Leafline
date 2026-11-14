//! The character's stats as the game's Character Stat window and its detail pane last showed them,
//! and the same with the buffs the player marks as on. Kept per character beside the damage read
//! (see [`super::damage`]), so it needs no window open to be shown.
//!
//! A buff's effect at each level comes from the skill data; its level from the last Skill window
//! read, or else from the build. The window already counts any buff that was on when it was read,
//! so the buffs marked on at that moment are taken back out before the ones on now are added.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use super::damage::{DamageState, PlayerDamage};
use super::mapdata::MapDataState;
use super::records::UserDataState;
use crate::build::{Build, Catalog};
use crate::mapdata::osms::{Effects, SkillRecord};
use crate::readers::character::{DetailWindow, StatWindow};

/// Everything the stat window and its detail pane showed. Each read fills in what it saw; a part
/// that wasn't open keeps what an earlier read gave.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct StatSheet {
    pub level: Option<u32>,
    /// Points put into each stat: the `28` of `30 (28+2)`.
    pub str: Option<u32>,
    pub dex: Option<u32>,
    pub int: Option<u32>,
    pub luk: Option<u32>,
    /// What gear adds to each: the `2` of `30 (28+2)`.
    pub gear_str: u32,
    pub gear_dex: u32,
    pub gear_int: u32,
    pub gear_luk: u32,
    pub ability_points: Option<u32>,
    pub max_hp: Option<u32>,
    pub max_mp: Option<u32>,
    pub fame: Option<u32>,
    pub attack_min: Option<u32>,
    pub attack_max: Option<u32>,
    pub weapon_def: Option<u32>,
    pub magic: Option<u32>,
    pub magic_def: Option<u32>,
    pub accuracy: Option<u32>,
    pub evasion: Option<u32>,
    /// As the pane shows them: 26 is 26%.
    pub crit_rate: Option<u32>,
    pub crit_damage: Option<u32>,
    pub speed: Option<u32>,
    pub jump: Option<u32>,
}

impl StatSheet {
    pub fn take_stats(&mut self, w: &StatWindow) {
        self.level = Some(w.level);
        (self.str, self.dex, self.int, self.luk) = (Some(w.str), Some(w.dex), Some(w.int), Some(w.luk));
        (self.gear_str, self.gear_dex, self.gear_int, self.gear_luk) = (w.bonus.str, w.bonus.dex, w.bonus.int, w.bonus.luk);
        self.ability_points = Some(w.ability_points);
        // A line that didn't read this time keeps the last one that did.
        self.max_hp = w.max_hp.or(self.max_hp);
        self.max_mp = w.max_mp.or(self.max_mp);
        self.fame = w.fame.or(self.fame);
    }

    pub fn take_detail(&mut self, d: &DetailWindow) {
        self.attack_min = Some(d.attack_min);
        self.attack_max = Some(d.attack_max);
        let keep = |new: Option<u32>, old: Option<u32>| new.or(old);
        self.weapon_def = keep(d.weapon_def, self.weapon_def);
        self.magic = keep(d.magic, self.magic);
        self.magic_def = keep(d.magic_def, self.magic_def);
        self.accuracy = keep(d.accuracy, self.accuracy);
        self.evasion = keep(d.evasion, self.evasion);
        self.crit_rate = keep(d.crit_rate, self.crit_rate);
        self.crit_damage = keep(d.crit_damage, self.crit_damage);
        self.speed = keep(d.speed, self.speed);
        self.jump = keep(d.jump, self.jump);
    }
}

/// A skill's level as the Skill window showed it, by the skill data's name. Names, not ids: jobs
/// share some (Haste, Meditation, the boosters), and the window doesn't say whose it is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillSeen {
    pub name: String,
    pub level: u32,
}

/// One of the character's buffs, at its level.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuffRow {
    pub id: u32,
    pub name: String,
    /// `None` until the Skill window has shown it, or the build gives it.
    pub level: Option<u32>,
    pub max_level: u32,
    /// The level came from the Skill window rather than the build.
    pub level_read: bool,
    /// Marked as on now.
    pub on: bool,
    /// Marked as on when the stat window was last read, so already in its numbers.
    pub on_at_read: bool,
    /// What it does at that level.
    pub effects: Effects,
    pub description: Option<String>,
}

/// What the Character tab shows of the character's stats.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsView {
    pub sheet: Option<StatSheet>,
    /// When the stat window or its detail pane was last read, in Unix seconds.
    pub read_at: Option<i64>,
    pub buffs: Vec<BuffRow>,
    /// The sheet with the buffs on now in place of those on at the read. `None` when it would be
    /// the same.
    pub buffed: Option<StatSheet>,
    /// The worn gear's Attack, which a buff's Attack is counted against. `None` with no gear worn
    /// that has any.
    pub weapon_attack: Option<u32>,
    /// A buff on changes Attack, but no worn gear says what the Attack is, so the attack range is
    /// left as read.
    pub attack_unscaled: bool,
}

/// The jobs whose skills a character of `job` has: the beginner's, and each job on the way.
fn job_path(job: u32) -> Vec<u32> {
    let mut path = vec![0, job / 100 * 100, job / 10 * 10, job];
    path.sort_unstable();
    path.dedup();
    path
}

/// The skill's effect and description at `level`, the last level the data has when it has fewer.
fn at_level(skill: &SkillRecord, level: u32) -> (Effects, Option<String>) {
    let i = level.max(1) as usize - 1;
    let effects = skill.effects.get(i).or(skill.effects.last()).copied().unwrap_or_default();
    let description = skill.level_descriptions.get(i).or(skill.level_descriptions.last()).cloned();
    (effects, description)
}

/// The character's buffs: those of their build's jobs, or with no build, those the Skill window
/// showed. Each at the level the Skill window last showed, or else the build's. A buff read as not
/// learned is left out.
pub fn buff_rows(catalog: &Catalog, build: Option<&Build>, seen: &[SkillSeen], on: &[u32], on_at_read: &[u32]) -> Vec<BuffRow> {
    let seen_level = |skill: &SkillRecord| seen.iter().find(|s| s.name.eq_ignore_ascii_case(&skill.name)).map(|s| s.level);
    let planned = |skill: &SkillRecord| {
        build.and_then(|b| b.start.skills.iter().find(|k| k.skill_id == skill.id)).map(|k| k.level).filter(|&l| l > 0)
    };
    let job = build.map(|b| b.plan.job).filter(|&j| j > 0);
    let path = job.map(job_path);
    let mut rows: Vec<BuffRow> = Vec::new();
    for skill in catalog.skills().iter().filter(|s| s.buff) {
        let mine = match &path {
            Some(path) => path.contains(&skill.job_id),
            None => seen_level(skill).is_some_and(|l| l > 0),
        };
        // Jobs share some buffs by name; the first of a name stands for them all.
        if !mine || rows.iter().any(|r| r.name.eq_ignore_ascii_case(&skill.name)) {
            continue;
        }
        let read = seen_level(skill);
        if read == Some(0) {
            continue;
        }
        let level = read.or_else(|| planned(skill)).map(|l| l.min(skill.max_level.max(1)));
        let (effects, description) = level.map(|l| at_level(skill, l)).unwrap_or_default();
        rows.push(BuffRow {
            id: skill.id,
            name: skill.name.clone(),
            level,
            max_level: skill.max_level,
            level_read: read.is_some(),
            on: level.is_some() && on.contains(&skill.id),
            on_at_read: level.is_some() && on_at_read.contains(&skill.id),
            effects,
            description,
        });
    }
    rows
}

/// What the worn gear adds to Attack, from the pieces' own stats.
pub fn weapon_attack(build: Option<&Build>) -> Option<u32> {
    let total: i32 = build?
        .start
        .pieces
        .iter()
        .flat_map(|p| &p.stats)
        .filter(|s| s.label.eq_ignore_ascii_case("Attack") || s.label.eq_ignore_ascii_case("Weapon Attack"))
        .map(|s| s.value)
        .sum();
    u32::try_from(total).ok().filter(|&t| t > 0)
}

fn sum<'a>(rows: impl Iterator<Item = &'a BuffRow>) -> Effects {
    rows.fold(Effects::default(), |mut all, r| {
        let e = &r.effects;
        all.attack += e.attack;
        all.magic += e.magic;
        all.weapon_def += e.weapon_def;
        all.weapon_def_percent += e.weapon_def_percent;
        all.magic_def += e.magic_def;
        all.accuracy += e.accuracy;
        all.evasion += e.evasion;
        all.speed += e.speed;
        all.jump += e.jump;
        all
    })
}

/// The sheet with the buffs on now in place of those on when it was read, and whether the attack
/// range had to be left as read. `None` when nothing changes.
pub fn with_buffs(sheet: &StatSheet, rows: &[BuffRow], weapon_attack: Option<u32>) -> Option<(StatSheet, bool)> {
    let now = sum(rows.iter().filter(|r| r.on));
    let then = sum(rows.iter().filter(|r| r.on_at_read));
    if now == then {
        return None;
    }
    let shift = |value: Option<u32>, by: i32| value.map(|v| (i64::from(v) + i64::from(by)).max(0) as u32);
    let mut out = *sheet;
    out.magic = shift(sheet.magic, now.magic - then.magic);
    out.magic_def = shift(sheet.magic_def, now.magic_def - then.magic_def);
    out.accuracy = shift(sheet.accuracy, now.accuracy - then.accuracy);
    out.evasion = shift(sheet.evasion, now.evasion - then.evasion);
    out.speed = shift(sheet.speed, now.speed - then.speed);
    out.jump = shift(sheet.jump, now.jump - then.jump);
    // Weapon Def.: the share is of the defence without buffs, and the flat part comes on top.
    out.weapon_def = sheet.weapon_def.map(|shown| {
        let base = (f64::from(shown) - f64::from(then.weapon_def)) / (1.0 + f64::from(then.weapon_def_percent) / 100.0);
        (base * (1.0 + f64::from(now.weapon_def_percent) / 100.0) + f64::from(now.weapon_def)).round().max(0.0) as u32
    });
    // The attack range grows with Attack: the range read went with the gear's Attack and the
    // buffs then, so it is scaled by the Attack now over that. Without the gear's Attack the
    // share is unknown, and the range stays as read.
    let mut unscaled = false;
    if now.attack != then.attack {
        match weapon_attack {
            Some(gear) => {
                let then_attack = f64::from(gear) + f64::from(then.attack);
                let now_attack = f64::from(gear) + f64::from(now.attack);
                if then_attack > 0.0 {
                    let scale = |v: Option<u32>| v.map(|v| (f64::from(v) * now_attack.max(0.0) / then_attack).round() as u32);
                    out.attack_min = scale(sheet.attack_min);
                    out.attack_max = scale(sheet.attack_max);
                }
            }
            None => unscaled = true,
        }
    }
    Some((out, unscaled))
}

/// The view of `name`'s stats from what is stored.
pub fn view(app: &AppHandle, name: &str, stored: Option<&PlayerDamage>) -> StatsView {
    let build = app.state::<UserDataState>().with(|db| db.active_build(Some(name))).ok().flatten();
    // A build tied to someone else isn't this character's.
    let build = build.filter(|b| b.character.as_deref().is_none_or(|owner| owner.eq_ignore_ascii_case(name)));
    let empty = PlayerDamage::default();
    let stored = stored.unwrap_or(&empty);
    let buffs = app
        .state::<MapDataState>()
        .catalog()
        .map(|catalog| buff_rows(&catalog, build.as_ref(), &stored.skill_levels, &stored.buffs_on, &stored.read_with))
        .unwrap_or_default();
    let weapon_attack = weapon_attack(build.as_ref());
    let buffed = stored.sheet.as_ref().and_then(|sheet| with_buffs(sheet, &buffs, weapon_attack));
    StatsView {
        sheet: stored.sheet,
        read_at: stored.read_at,
        buffs,
        attack_unscaled: buffed.is_some_and(|(_, unscaled)| unscaled),
        buffed: buffed.map(|(sheet, _)| sheet),
        weapon_attack,
    }
}

/// What the Character tab shows of `name`'s stats and buffs.
#[tauri::command]
pub fn character_stats(app: AppHandle, state: State<DamageState>, name: String) -> StatsView {
    let stored = state.get(&name);
    view(&app, &name, stored.as_ref())
}

/// Marks one of `name`'s buffs as on or off.
#[tauri::command]
pub fn set_buff(app: AppHandle, name: String, skill: u32, on: bool) -> crate::error::Result<()> {
    super::damage::set_buff(&app, &name, skill, on)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::catalog::tests::skill;
    use crate::build::plan::{Piece, PieceStat};

    fn buff(id: u32, name: &str, job_id: u32, levels: &[Effects]) -> SkillRecord {
        let mut s = skill(id, name, job_id, levels.len() as u32, &[]);
        s.buff = true;
        s.effects = levels.to_vec();
        s.level_descriptions = levels.iter().map(|e| format!("Attack +{}", e.attack)).collect();
        s
    }

    fn catalog() -> Catalog {
        let rage = |attack| Effects { attack, weapon_def: -attack, seconds: 300, ..Effects::default() };
        let focus = |n| Effects { accuracy: n, evasion: n, ..Effects::default() };
        Catalog::new(
            vec![
                buff(1, "Rage", 110, &[rage(10), rage(20), rage(30)]),
                buff(2, "Focus", 300, &[focus(5), focus(10)]),
                buff(3, "Haste", 410, &[Effects { speed: 30, jump: 10, ..Effects::default() }]),
                buff(4, "Haste", 420, &[Effects { speed: 30, jump: 10, ..Effects::default() }]),
                buff(5, "Iron Body", 100, &[Effects { weapon_def_percent: 40, ..Effects::default() }]),
                skill(6, "Power Strike", 100, 20, &[]),
            ],
            Vec::new(),
        )
    }

    fn seen(name: &str, level: u32) -> SkillSeen {
        SkillSeen { name: name.into(), level }
    }

    fn build(job: u32) -> Build {
        let mut b = Build::default();
        b.plan.job = job;
        b
    }

    #[test]
    fn a_warriors_buffs_come_from_the_job_path_at_the_level_read() {
        let rows = buff_rows(&catalog(), Some(&build(110)), &[seen("Rage", 2)], &[1], &[]);
        let names: Vec<_> = rows.iter().map(|r| (r.name.as_str(), r.level, r.on)).collect();
        // Iron Body is the warrior's, not yet read: listed without a level, so it can't be on.
        assert_eq!(names, [("Rage", Some(2), true), ("Iron Body", None, false)]);
        assert_eq!(rows[0].effects.attack, 20);
        assert_eq!(rows[0].description.as_deref(), Some("Attack +20"));
    }

    #[test]
    fn with_no_build_the_buffs_are_those_the_skill_window_showed_and_shared_names_count_once() {
        let rows = buff_rows(&catalog(), None, &[seen("Haste", 1), seen("Focus", 0)], &[], &[]);
        assert_eq!(rows.iter().map(|r| r.id).collect::<Vec<_>>(), [3]);
    }

    #[test]
    fn a_buff_read_as_unlearned_is_left_out_and_the_build_gives_a_level_the_window_didnt() {
        let mut b = build(300);
        b.start.skills.push(crate::build::SkillLevel { skill_id: 2, level: 2 });
        let rows = buff_rows(&catalog(), Some(&b), &[], &[], &[]);
        assert_eq!((rows[0].level, rows[0].level_read), (Some(2), false));
        assert!(buff_rows(&catalog(), Some(&b), &[seen("Focus", 0)], &[], &[]).is_empty());
    }

    fn sheet() -> StatSheet {
        StatSheet {
            attack_min: Some(100),
            attack_max: Some(200),
            weapon_def: Some(140),
            accuracy: Some(76),
            evasion: Some(33),
            speed: Some(100),
            jump: Some(100),
            ..StatSheet::default()
        }
    }

    #[test]
    fn buffs_on_now_are_added_and_those_on_at_the_read_taken_out() {
        let rows = buff_rows(&catalog(), Some(&build(300)), &[seen("Focus", 2)], &[2], &[]);
        let (buffed, unscaled) = with_buffs(&sheet(), &rows, None).unwrap();
        assert_eq!((buffed.accuracy, buffed.evasion, buffed.attack_max), (Some(86), Some(43), Some(200)));
        assert!(!unscaled, "no Attack changed");
        // Read with Focus on, and Focus still on: nothing changes.
        let rows = buff_rows(&catalog(), Some(&build(300)), &[seen("Focus", 2)], &[2], &[2]);
        assert!(with_buffs(&sheet(), &rows, None).is_none());
        // Read with it on, now off.
        let rows = buff_rows(&catalog(), Some(&build(300)), &[seen("Focus", 2)], &[], &[2]);
        assert_eq!(with_buffs(&sheet(), &rows, None).unwrap().0.accuracy, Some(66));
    }

    #[test]
    fn attack_scales_the_range_by_the_gears_attack_and_stays_as_read_without_it() {
        let rows = buff_rows(&catalog(), Some(&build(110)), &[seen("Rage", 3)], &[1], &[]);
        let (buffed, unscaled) = with_buffs(&sheet(), &rows, Some(60)).unwrap();
        // 60 Attack from gear, 90 with Rage: the range grows by half.
        assert_eq!((buffed.attack_min, buffed.attack_max, unscaled), (Some(150), Some(300), false));
        assert_eq!(buffed.weapon_def, Some(110), "Rage takes Weapon Def. off");
        let (buffed, unscaled) = with_buffs(&sheet(), &rows, None).unwrap();
        assert_eq!((buffed.attack_max, unscaled), (Some(200), true));
    }

    #[test]
    fn a_share_of_weapon_def_is_of_the_defence_without_buffs() {
        let rows = buff_rows(&catalog(), Some(&build(110)), &[seen("Iron Body", 1)], &[], &[5]);
        // Read at 140 with Iron Body's 40% on: 100 without it.
        assert_eq!(with_buffs(&sheet(), &rows, None).unwrap().0.weapon_def, Some(100));
    }

    #[test]
    fn the_gears_attack_is_the_worn_pieces_attack() {
        let mut b = build(320);
        let piece = |label: &str, value| Piece {
            id: 0,
            item: None,
            name: "Made-up piece".into(),
            item_type: None,
            level: None,
            stats: vec![PieceStat { label: label.into(), value }],
            upgrades: None,
            slots_left: None,
            slots: None,
        };
        b.start.pieces = vec![piece("Weapon Attack", 47), piece("Attack", 2), piece("DEX", 3)];
        assert_eq!(weapon_attack(Some(&b)), Some(49));
        assert_eq!(weapon_attack(Some(&build(320))), None);
    }

    /// With `LEAFLINE_OSMS` naming a folder holding the downloaded `skills.json`, the buffs found
    /// in it are the ones a player would call buffs.
    #[test]
    fn the_real_skill_data_gives_the_buffs_players_know() {
        use crate::mapdata::osms::Records;
        let Some(dir) = std::env::var_os("LEAFLINE_OSMS") else { return };
        let body = std::fs::read(std::path::Path::new(&dir).join("skills.json")).expect("skills.json");
        let Records::Skills(skills) = Records::parse("skills.json", &body).unwrap() else { panic!() };
        let buff = |name: &str| skills.iter().find(|s| s.name == name).map(|s| s.buff);
        for name in ["Rage", "Focus", "Haste", "Meditation", "Magic Armor", "Iron Body", "Nimble Feet", "Magic Guard", "Dark Sight"] {
            assert_eq!(buff(name), Some(true), "{name}");
        }
        for name in ["Heal", "Teleport", "Arrow Blow", "Improved HP Recovery", "MP Eater"] {
            assert_eq!(buff(name), Some(false), "{name}");
        }
        let rage = skills.iter().find(|s| s.name == "Rage").unwrap();
        assert!(rage.effects.last().unwrap().attack > 0);
    }

    #[test]
    fn a_read_fills_in_what_it_saw_and_keeps_the_rest() {
        let mut s = StatSheet::default();
        let w = StatWindow { level: 33, str: 28, dex: 149, int: 4, luk: 4, max_hp: Some(1220), ..StatWindow::default() };
        s.take_stats(&w);
        s.take_detail(&DetailWindow { attack_min: 107, attack_max: 289, accuracy: Some(76), ..DetailWindow::default() });
        s.take_stats(&StatWindow { max_hp: None, ..w });
        s.take_detail(&DetailWindow { attack_min: 107, attack_max: 290, ..DetailWindow::default() });
        assert_eq!((s.max_hp, s.accuracy, s.attack_max, s.dex), (Some(1220), Some(76), Some(290), Some(149)));
    }
}
