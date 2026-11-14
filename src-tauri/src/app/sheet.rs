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
use crate::readers::character::{DetailBonus, DetailWindow, StatWindow};

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
    /// What buffs added to each detail line, as the pane showed it in red: the `+14` of `90 (+14)`.
    pub buff_bonus: DetailBonus,
    /// The Attack line was red, so a buff was in the range.
    pub attack_buffed: bool,
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
        self.buff_bonus = d.bonus;
        self.attack_buffed = d.attack_buffed;
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
    /// As the window showed it, buffs and all.
    pub sheet: Option<StatSheet>,
    /// When the stat window or its detail pane was last read, in Unix seconds.
    pub read_at: Option<i64>,
    pub buffs: Vec<BuffRow>,
    /// The sheet without any buff: what the window showed less what it said buffs added.
    pub base: Option<StatSheet>,
    /// The sheet with the buffs on now. `None` when none is on that changes a number.
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

/// The sheet without buffs. Each detail line loses what the pane said buffs added to it. The
/// Attack line gives no share, only its red, so a red range is scaled back by the buffs that were
/// on as it was read, when the gear's Attack is known.
pub fn without_buffs(sheet: &StatSheet, rows: &[BuffRow], weapon_attack: Option<u32>) -> StatSheet {
    let b = &sheet.buff_bonus;
    let less = |value: Option<u32>, by: i32| value.map(|v| (i64::from(v) - i64::from(by)).max(0) as u32);
    let mut out = *sheet;
    out.weapon_def = less(sheet.weapon_def, b.weapon_def);
    out.magic = less(sheet.magic, b.magic);
    out.magic_def = less(sheet.magic_def, b.magic_def);
    out.accuracy = less(sheet.accuracy, b.accuracy);
    out.evasion = less(sheet.evasion, b.evasion);
    out.crit_rate = less(sheet.crit_rate, b.crit_rate);
    out.crit_damage = less(sheet.crit_damage, b.crit_damage);
    out.speed = less(sheet.speed, b.speed);
    out.jump = less(sheet.jump, b.jump);
    out.buff_bonus = DetailBonus::default();
    let then = sum(rows.iter().filter(|r| r.on_at_read)).attack;
    if sheet.attack_buffed
        && then != 0
        && let Some(gear) = weapon_attack
    {
        let scale = |v: Option<u32>| v.map(|v| (f64::from(v) * f64::from(gear) / f64::from(gear as i32 + then).max(1.0)).round() as u32);
        out.attack_min = scale(sheet.attack_min);
        out.attack_max = scale(sheet.attack_max);
    }
    out.attack_buffed = false;
    out
}

/// The sheet without buffs, `base`, with the buffs on now, and whether a buff's Attack had to be
/// left out of the range for want of the gear's Attack. `None` when no buff on changes a number.
pub fn with_buffs(base: &StatSheet, rows: &[BuffRow], weapon_attack: Option<u32>) -> Option<(StatSheet, bool)> {
    let now = sum(rows.iter().filter(|r| r.on));
    if !now.any() {
        return None;
    }
    let more = |value: Option<u32>, by: i32| value.map(|v| (i64::from(v) + i64::from(by)).max(0) as u32);
    let mut out = *base;
    out.magic = more(base.magic, now.magic);
    out.magic_def = more(base.magic_def, now.magic_def);
    out.accuracy = more(base.accuracy, now.accuracy);
    out.evasion = more(base.evasion, now.evasion);
    out.speed = more(base.speed, now.speed);
    out.jump = more(base.jump, now.jump);
    // Weapon Def.: the share is of the defence without buffs, and the flat part comes on top.
    out.weapon_def = base.weapon_def.map(|v| {
        (f64::from(v) * (1.0 + f64::from(now.weapon_def_percent) / 100.0) + f64::from(now.weapon_def)).round().max(0.0) as u32
    });
    // The attack range grows with Attack, by the gear's Attack and the buffs' over the gear's.
    let mut unscaled = false;
    if now.attack != 0 {
        match weapon_attack {
            Some(gear) => {
                let scale = |v: Option<u32>| {
                    v.map(|v| (f64::from(v) * f64::from(gear as i32 + now.attack).max(0.0) / f64::from(gear)).round() as u32)
                };
                out.attack_min = scale(base.attack_min);
                out.attack_max = scale(base.attack_max);
            }
            None => unscaled = true,
        }
    }
    Some((out, unscaled))
}

/// How many buffs are tried together when working out which are on.
const MOST_TRIED: usize = 12;

/// Which buffs the detail pane shows are on: the one set of the character's buffs, among those
/// that change a line it shows, whose numbers add up to what it showed in red, a red Attack line
/// meaning one with Attack is on. `None` when no set fits, or more than one does, which leaves
/// the switches as the player set them.
pub fn seen_on(rows: &[BuffRow], sheet: &StatSheet) -> Option<Vec<u32>> {
    let tried: Vec<&BuffRow> = rows.iter().filter(|r| r.level.is_some() && r.effects.any()).take(MOST_TRIED).collect();
    let shown = &sheet.buff_bonus;
    let base_def = sheet.weapon_def.map(|v| i64::from(v) - i64::from(shown.weapon_def));
    let fits = |set: u32| {
        let all = sum(tried.iter().enumerate().filter(|(i, _)| set & (1 << i) != 0).map(|(_, r)| *r));
        let def = i64::from(all.weapon_def) + base_def.map_or(0, |b| (b as f64 * f64::from(all.weapon_def_percent) / 100.0).round() as i64);
        all.accuracy == shown.accuracy
            && all.evasion == shown.evasion
            && all.magic == shown.magic
            && all.magic_def == shown.magic_def
            && all.speed == shown.speed
            && all.jump == shown.jump
            && (def - i64::from(shown.weapon_def)).abs() <= 1
            && (all.attack != 0) == sheet.attack_buffed
    };
    let mut found = (0..1u32 << tried.len()).filter(|&set| fits(set));
    let set = found.next()?;
    if found.next().is_some() {
        return None;
    }
    Some(tried.iter().enumerate().filter(|(i, _)| set & (1 << i) != 0).map(|(_, r)| r.id).collect())
}

/// The ids of the buffs [`seen_on`] can tell about: on or off, the pane would show it.
pub fn seen_buffs(rows: &[BuffRow]) -> Vec<u32> {
    rows.iter().filter(|r| r.level.is_some() && r.effects.any()).take(MOST_TRIED).map(|r| r.id).collect()
}

/// The character's buffs and the gear's Attack, from their build and what was read.
pub fn buffs_of(app: &AppHandle, name: &str, stored: &PlayerDamage) -> (Vec<BuffRow>, Option<u32>) {
    let build = app.state::<UserDataState>().with(|db| db.active_build(Some(name))).ok().flatten();
    // A build tied to someone else isn't this character's.
    let build = build.filter(|b| b.character.as_deref().is_none_or(|owner| owner.eq_ignore_ascii_case(name)));
    let buffs = app
        .state::<MapDataState>()
        .catalog()
        .map(|catalog| buff_rows(&catalog, build.as_ref(), &stored.skill_levels, &stored.buffs_on, &stored.read_with))
        .unwrap_or_default();
    (buffs, weapon_attack(build.as_ref()))
}

/// The view of `name`'s stats from what is stored.
pub fn view(app: &AppHandle, name: &str, stored: Option<&PlayerDamage>) -> StatsView {
    let empty = PlayerDamage::default();
    let stored = stored.unwrap_or(&empty);
    let (buffs, weapon_attack) = buffs_of(app, name, stored);
    let base = stored.sheet.as_ref().map(|sheet| without_buffs(sheet, &buffs, weapon_attack));
    let buffed = base.as_ref().and_then(|base| with_buffs(base, &buffs, weapon_attack));
    StatsView {
        sheet: stored.sheet,
        read_at: stored.read_at,
        buffs,
        base,
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

    /// The window as it shows Focus Lv 2 on: `86 (+10)`, `43 (+10)` in red.
    fn focused() -> StatSheet {
        StatSheet {
            accuracy: Some(86),
            evasion: Some(43),
            buff_bonus: DetailBonus { accuracy: 10, evasion: 10, ..DetailBonus::default() },
            ..sheet()
        }
    }

    #[test]
    fn what_the_pane_says_buffs_add_is_taken_off_and_the_buffs_on_now_added() {
        // Read with Focus on in the game, and marked on: shown as it was, 76 beside it.
        let rows = buff_rows(&catalog(), Some(&build(300)), &[seen("Focus", 2)], &[2], &[2]);
        let base = without_buffs(&focused(), &rows, None);
        assert_eq!((base.accuracy, base.evasion), (Some(76), Some(33)));
        let (buffed, unscaled) = with_buffs(&base, &rows, None).unwrap();
        assert_eq!((buffed.accuracy, buffed.evasion, buffed.attack_max, unscaled), (Some(86), Some(43), Some(200), false));
        // Read with it off in the game while it was still marked on: the plain numbers are the
        // ones without buffs, never less.
        let base = without_buffs(&sheet(), &rows, None);
        assert_eq!(base.accuracy, Some(76));
        // Nothing marked on: nothing to add.
        let off = buff_rows(&catalog(), Some(&build(300)), &[seen("Focus", 2)], &[], &[]);
        assert!(with_buffs(&base, &off, None).is_none());
    }

    #[test]
    fn the_pane_says_which_buffs_are_on() {
        let rows = buff_rows(&catalog(), Some(&build(300)), &[seen("Focus", 2)], &[], &[]);
        assert_eq!(seen_on(&rows, &focused()), Some(vec![2]));
        assert_eq!(seen_on(&rows, &sheet()), Some(vec![]), "no red: off");
        let odd = StatSheet { buff_bonus: DetailBonus { accuracy: 7, ..DetailBonus::default() }, ..sheet() };
        assert_eq!(seen_on(&rows, &odd), None, "nothing fits: the switches stay");
        // A red Attack line is a buff with Attack.
        let rows = buff_rows(&catalog(), Some(&build(110)), &[seen("Rage", 1), seen("Iron Body", 1)], &[], &[]);
        let raging = StatSheet { attack_buffed: true, weapon_def: Some(130), buff_bonus: DetailBonus { weapon_def: -10, ..DetailBonus::default() }, ..sheet() };
        assert_eq!(seen_on(&rows, &raging), Some(vec![1]));
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
        // Read red with Rage on: scaled back to the gear's Attack alone.
        let rows = buff_rows(&catalog(), Some(&build(110)), &[seen("Rage", 3)], &[1], &[1]);
        let red = StatSheet { attack_min: Some(150), attack_max: Some(300), attack_buffed: true, ..sheet() };
        let base = without_buffs(&red, &rows, Some(60));
        assert_eq!((base.attack_min, base.attack_max), (Some(100), Some(200)));
    }

    #[test]
    fn a_share_of_weapon_def_is_of_the_defence_without_buffs() {
        let rows = buff_rows(&catalog(), Some(&build(110)), &[seen("Iron Body", 1)], &[5], &[5]);
        // Shown as 140 (+40) with Iron Body's 40% on: 100 without it, 140 with it.
        let read = StatSheet { buff_bonus: DetailBonus { weapon_def: 40, ..DetailBonus::default() }, ..sheet() };
        let base = without_buffs(&read, &rows, None);
        assert_eq!(base.weapon_def, Some(100));
        assert_eq!(with_buffs(&base, &rows, None).unwrap().0.weapon_def, Some(140));
        assert_eq!(seen_on(&rows, &read), Some(vec![5]));
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
