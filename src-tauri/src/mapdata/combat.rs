//! What a fight costs the player, worked out from Classic's combat rules as the community has
//! measured them and from the player's own numbers as the game shows them. It gives kills and EXP
//! an hour on any map, and is tuned to the player's own measured play, so what the rules don't
//! pin down is filled in from how they really play.
//!
//! The rules (mapleclassic.wiki, CC BY-NC-SA 4.0, pages marked "pre-launch", read 2026-11-13):
//! - **Hit chance** (Accuracy, "the best model available today"): with `L` the levels the monster
//!   is above the player, `A = ACC × 100 / ((L + 51) × 5)`, `f = 0.3 / (1 + e^((A − avoid) / 12))`,
//!   and a blow lands when a roll from `0.95 − f` to `1.05 + f` times `A` reaches the avoid.
//!   Spells never miss.
//! - **Level penalty** (Damage, "a confirmed formula"): against a monster `d` levels above, damage
//!   is divided by `d² × 0.005 + 1` under 10 levels, and by `d × 0.05 + 1` from 10.
//! - **Defence** divides damage by a factor that grows with it; the wiki says the coefficients
//!   aren't confirmed. Here the factor is `1 + defence / K`, with `K` fitted to the player's play.
//! - **Attack time** by the weapon's speed tier (Attack Speed).
//!
//! Nothing here comes from Nexon. Every constant is a starting point the player's own play moves.

use serde::{Deserialize, Serialize};

use super::atlas::MobRow;

/// Kills simulated per monster to find how many attacks one takes.
const SIMULATED_KILLS: u32 = 400;
/// Attacks after which a monster counts as one the player can't really kill.
const MAX_ATTACKS: u32 = 120;
/// How much of an attack's reach beyond one monster pays off: monsters aren't always bunched.
pub const AOE_SHARE: f64 = 0.5;
/// The spawn limit eases in: kills an hour are `(free⁻ᵖ + limit⁻ᵖ)^(−1/p)`, since near the limit
/// part of each round is spent waiting. A hard limit read too high there.
const SOFT_LIMIT: f64 = 3.0;
/// Measured play this close to the spawn limit says little about how fast the player kills.
const NEAR_LIMIT: f64 = 0.9;

/// The player as the estimates see them: their blows, their reach and how fast they attack.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Fighter {
    pub level: u32,
    /// One damage line before the monster's defence and the level penalty: its low and high ends,
    /// the skill's share included.
    pub low: f64,
    pub high: f64,
    /// Damage lines one attack deals, and monsters it can reach.
    pub lines: u32,
    pub targets: u32,
    /// Weapon accuracy. `None` for spells, which never miss, or when it wasn't read.
    pub accuracy: Option<f64>,
    pub magic: bool,
    /// Chance of a critical hit, 0 to 1, and what it adds: 0.35 is +35%.
    pub crit_rate: f64,
    pub crit_bonus: f64,
    /// Seconds one attack takes.
    pub attack_seconds: f64,
}

/// What the rules leave open, as fitted to the player's play.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tuning {
    /// The share of fighting time spent attacking: positioning, knock-back and potions take the
    /// rest.
    pub efficiency: f64,
    /// Seconds each kill costs beyond fighting and the map's walking: looting, finding the next.
    pub overhead: f64,
    /// `K` in the defence factor `1 + defence / K`.
    pub defence: f64,
}

impl Default for Tuning {
    fn default() -> Self {
        Self { efficiency: 0.85, overhead: 1.0, defence: 100.0 }
    }
}

/// How sure the starting values are, for the fit: one standard deviation of each. The defence's is
/// of its logarithm.
const EFFICIENCY_SPREAD: f64 = 0.6;
const OVERHEAD_SPREAD: f64 = 2.0;
const DEFENCE_SPREAD: f64 = 1.0;
/// Values of `K` tried.
const DEFENCE_GRID: [f64; 10] = [20.0, 30.0, 45.0, 65.0, 100.0, 140.0, 200.0, 280.0, 400.0, 600.0];

/// The chance one blow lands, from the wiki's roll.
pub fn hit_chance(accuracy: f64, level: u32, foe_level: u32, avoid: f64) -> f64 {
    if avoid <= 0.0 {
        return 1.0;
    }
    let above = f64::from(foe_level.saturating_sub(level));
    let a = accuracy * 100.0 / ((above + 51.0) * 5.0);
    if a <= 0.0 {
        return 0.0;
    }
    let f = 0.3 / (1.0 + ((a - avoid) / 12.0).exp());
    ((1.05 + f - avoid / a) / (0.10 + 2.0 * f)).clamp(0.0, 1.0)
}

/// What is left of a blow against a monster above the player's level.
pub fn level_factor(level: u32, foe_level: u32) -> f64 {
    let d = f64::from(foe_level.saturating_sub(level));
    if d < 10.0 { 1.0 / (d * d * 0.005 + 1.0) } else { 1.0 / (d * 0.05 + 1.0) }
}

/// A small, fast random number source, the same every run so estimates don't flicker.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> f64 {
        // SplitMix64.
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        (z >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// What one monster type costs the player.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Duel {
    /// Chance a blow lands.
    pub hit: f64,
    /// Attacks one kill takes on average.
    pub attacks: f64,
}

/// Attacks to defeat `foe`, simulated kill by kill, since a monster that falls to one blow 60%
/// of the time takes 1.4 attacks, not the one its HP over the average blow would say.
pub fn duel(fighter: &Fighter, foe: &MobRow, defence_k: f64) -> Duel {
    let hit = match (fighter.magic, fighter.accuracy) {
        (false, Some(acc)) => hit_chance(acc, fighter.level, foe.level, f64::from(foe.evasion.unwrap_or(0))),
        _ => 1.0,
    };
    let defence = if fighter.magic { foe.magic_def } else { foe.weapon_def };
    let divisor = 1.0 + f64::from(defence.unwrap_or(0)) / defence_k.max(1.0);
    let scale = level_factor(fighter.level, foe.level) / divisor;
    if hit <= 0.0 || fighter.high <= 0.0 {
        return Duel { hit, attacks: f64::from(MAX_ATTACKS) };
    }
    let mut rng = Rng(u64::from(foe.hp) << 32 ^ u64::from(foe.level) << 16 ^ fighter.high.to_bits().rotate_left(7));
    let (low, span) = (fighter.low.min(fighter.high), (fighter.high - fighter.low).abs());
    let lines = fighter.lines.max(1);
    let mut total = 0u64;
    for _ in 0..SIMULATED_KILLS {
        let mut hp = f64::from(foe.hp.max(1));
        let mut attacks = 0;
        while hp > 0.0 && attacks < MAX_ATTACKS {
            attacks += 1;
            for _ in 0..lines {
                if rng.next() >= hit {
                    continue;
                }
                let mut blow = low + span * rng.next();
                if rng.next() < fighter.crit_rate {
                    blow *= 1.0 + fighter.crit_bonus;
                }
                hp -= (blow * scale).floor().max(1.0);
                if hp <= 0.0 {
                    break;
                }
            }
        }
        total += u64::from(attacks);
    }
    Duel { hit, attacks: total as f64 / f64::from(SIMULATED_KILLS) }
}

/// A map's monsters as the player would fight them, bosses left out, each counted by how many
/// spawn.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Battle {
    /// Seconds of attacking per kill at full efficiency, reach counted.
    pub fight_seconds: f64,
    pub exp_per_kill: f64,
    /// Average chance a blow lands, and attacks per kill.
    pub hit: f64,
    pub attacks: f64,
    /// Monsters on the map when it is full.
    pub count: u32,
    /// Average monster level.
    pub level: f64,
}

/// How `fighter` fares on a map with these monsters. `crowd` is how many monsters stand together
/// on a platform, for attacks that reach several. `None` with nothing but bosses.
pub fn battle(fighter: &Fighter, mobs: &[MobRow], defence_k: f64, crowd: f64) -> Option<Battle> {
    let foes: Vec<&MobRow> = mobs.iter().filter(|m| !m.boss && m.count > 0 && m.hp > 0).collect();
    let count: u32 = foes.iter().map(|m| m.count).sum();
    if count == 0 {
        return None;
    }
    let reach = 1.0 + AOE_SHARE * (f64::from(fighter.targets.max(1)).min(crowd.max(1.0)) - 1.0).max(0.0);
    let (mut fight, mut exp, mut hit, mut attacks, mut level) = (0.0, 0.0, 0.0, 0.0, 0.0);
    for foe in &foes {
        let w = f64::from(foe.count) / f64::from(count);
        let duel = duel(fighter, foe, defence_k);
        fight += w * duel.attacks * fighter.attack_seconds / reach;
        exp += w * f64::from(foe.exp);
        hit += w * duel.hit;
        attacks += w * duel.attacks;
        level += w * f64::from(foe.level);
    }
    Some(Battle { fight_seconds: fight, exp_per_kill: exp, hit, attacks, count, level })
}

/// Kills an hour the map can give at most: every monster defeated as soon as it is back.
pub fn spawn_limit(count: u32, respawn_seconds: f64) -> f64 {
    f64::from(count) * 3600.0 / respawn_seconds.max(1.0)
}

/// Kills an hour: as fast as the player kills, held back by the spawn limit as they near it.
pub fn kills_per_hour(free: f64, limit: f64) -> f64 {
    if free <= 0.0 || limit <= 0.0 {
        return 0.0;
    }
    (free.powf(-SOFT_LIMIT) + limit.powf(-SOFT_LIMIT)).powf(-1.0 / SOFT_LIMIT)
}

/// How fast the player would kill with no spawn limit, from what they measured under one. `None`
/// when they were near the limit, which hides how fast they could have gone.
pub fn free_kills(measured: f64, limit: f64) -> Option<f64> {
    if measured <= 0.0 || measured >= NEAR_LIMIT * limit {
        return None;
    }
    Some((measured.powf(-SOFT_LIMIT) - limit.powf(-SOFT_LIMIT)).powf(-1.0 / SOFT_LIMIT))
}

/// Seconds a kill takes, everything counted, with no spawn limit.
pub fn kill_seconds(battle: &Battle, tuning: &Tuning, walk_seconds: f64) -> f64 {
    battle.fight_seconds / tuning.efficiency.max(0.05) + walk_seconds + tuning.overhead.max(0.0)
}

/// One stretch of the player's own solo play, for fitting the [`Tuning`].
#[derive(Debug, Clone)]
pub struct Observation<'a> {
    /// The player as they were then.
    pub fighter: Fighter,
    pub mobs: &'a [MobRow],
    pub crowd: f64,
    pub walk_seconds: f64,
    pub limit: f64,
    pub kills_per_hour: f64,
    pub seconds: f64,
}

/// The tuning that fits the player's play, and how well.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Fit {
    pub tuning: Tuning,
    /// The typical miss of the fitted model on the player's own maps, as the standard deviation of
    /// the logarithm of measured over modelled kill time. Never under 0.15; 0.3 or more with
    /// little play to go on.
    pub spread: f64,
    /// Stretches and seconds of play the fit rests on.
    pub stretches: u32,
    pub seconds: f64,
}

impl Fit {
    /// The starting values, with nothing measured.
    pub fn unfitted() -> Self {
        Self { tuning: Tuning::default(), spread: 0.45, stretches: 0, seconds: 0.0 }
    }
}

/// Fits the [`Tuning`] to the player's play: for each `K` tried, the efficiency and overhead that
/// best explain each stretch's time per kill (a straight line in the time spent attacking),
/// held towards their starting values when little play says otherwise; then the `K` that fits
/// best, held the same way. Each stretch counts by its minutes, at most 30, so one long session
/// can't outweigh the rest.
pub fn fit(observations: &[Observation]) -> Fit {
    let start = Tuning::default();
    struct Point {
        weight: f64,
        minutes: f64,
        /// Seconds per kill measured, less the walking.
        rest: f64,
        seconds: f64,
        walk: f64,
    }
    let usable: Vec<(&Observation, f64)> = observations
        .iter()
        .filter_map(|o| free_kills(o.kills_per_hour, o.limit).map(|free| (o, 3600.0 / free)))
        .collect();
    if usable.is_empty() {
        return Fit::unfitted();
    }
    let a0 = 1.0 / start.efficiency;
    let (la, lo) = (1.0 / (EFFICIENCY_SPREAD * EFFICIENCY_SPREAD), 1.0 / (OVERHEAD_SPREAD * OVERHEAD_SPREAD));
    let mut best: Option<(f64, Tuning, Vec<(f64, f64, f64)>)> = None;
    for &k in &DEFENCE_GRID {
        let mut points: Vec<(Point, f64)> = Vec::new();
        for (o, seconds) in &usable {
            let Some(b) = battle(&o.fighter, o.mobs, k, o.crowd) else { continue };
            let minutes = (o.seconds / 60.0).min(30.0);
            // Errors are relative: a quarter of the time per kill is one standard deviation.
            let weight = minutes / (0.25 * seconds).powi(2);
            points.push((Point { weight, minutes, rest: seconds - o.walk_seconds, seconds: *seconds, walk: o.walk_seconds }, b.fight_seconds));
        }
        if points.is_empty() {
            continue;
        }
        // Weighted least squares of rest = a × fight + o, with the priors as extra terms.
        let (mut sff, mut sf, mut sw, mut sfy, mut sy) = (la, 0.0, lo, la * a0, lo * start.overhead);
        for (p, f) in &points {
            sff += p.weight * f * f;
            sf += p.weight * f;
            sw += p.weight;
            sfy += p.weight * f * p.rest;
            sy += p.weight * p.rest;
        }
        let det = sff * sw - sf * sf;
        if det.abs() < 1e-12 {
            continue;
        }
        let a = ((sfy * sw - sf * sy) / det).clamp(0.5, 4.0);
        let o = ((sy - sf * a) / sw).clamp(0.0, 20.0);
        let mut loss = la * (a - a0).powi(2) + lo * (o - start.overhead).powi(2);
        loss += ((k / start.defence).ln() / DEFENCE_SPREAD).powi(2);
        let mut logs = Vec::new();
        for (p, f) in &points {
            loss += p.weight * (p.rest - a * f - o).powi(2);
            let modelled = (a * f + o + p.walk).max(0.1);
            logs.push((p.minutes, p.seconds, modelled));
        }
        if best.as_ref().is_none_or(|(l, _, _)| loss < *l) {
            best = Some((loss, Tuning { efficiency: 1.0 / a, overhead: o, defence: k }, logs));
        }
    }
    let Some((_, tuning, logs)) = best else { return Fit::unfitted() };
    let minutes: f64 = logs.iter().map(|l| l.0).sum();
    let stretches = logs.len() as u32;
    let spread = if stretches >= 3 && minutes > 0.0 {
        let var = logs.iter().map(|(m, seen, model)| m * (seen / model).ln().powi(2)).sum::<f64>() / minutes;
        // A few stretches fit closely by chance: widen for how few there are.
        (var.sqrt() * (f64::from(stretches) / f64::from(stretches - 2)).sqrt()).max(0.15)
    } else {
        0.3
    };
    Fit { tuning, spread, stretches, seconds: usable.iter().map(|(o, _)| o.seconds).sum() }
}

/// Seconds one attack takes, from the wiki's table by weapon and speed tier (0 fastest to 12
/// slowest). Tiers between the table's are drawn on a straight line; past its ends, the nearest
/// two carry on.
pub fn attack_seconds(weapon_type: Option<&str>, tier: Option<u32>, magic: bool) -> f64 {
    let kind = weapon_type.unwrap_or("").to_lowercase();
    if magic {
        return if kind.contains("staff") { 0.87 } else { 0.81 };
    }
    let table: &[(f64, f64)] = if kind.contains("crossbow") {
        &[(4.0, 720.0), (6.0, 840.0), (8.0, 930.0)]
    } else if kind.contains("two-handed") || kind.contains("spear") || kind.contains("polearm") || kind.contains("pole arm") {
        &[(4.0, 660.0), (6.0, 750.0), (8.0, 870.0)]
    } else {
        &[(2.0, 600.0), (4.0, 720.0), (5.0, 750.0), (6.0, 810.0), (8.0, 900.0)]
    };
    let t = f64::from(tier.unwrap_or(6));
    let i = table.iter().position(|&(x, _)| x >= t).unwrap_or(table.len() - 1).max(1);
    let ((x0, y0), (x1, y1)) = (table[i - 1], table[i]);
    let ms = y0 + (y1 - y0) * (t - x0) / (x1 - x0);
    (ms / 1000.0).clamp(0.4, 1.5)
}

/// The damage range of a spell, from the wiki: with `M` the Magic shown in the stat window,
/// `max = (base + M/7) × ((2M + INT)/100 + 1)`, and the minimum the same with `2M` times the
/// mastery factor `(0.1 + mastery/10) × 0.8`.
pub fn spell_range(base: f64, magic: f64, int: f64, mastery: f64) -> (f64, f64) {
    let factor = (0.1 + mastery / 10.0) * 0.8;
    let first = base + magic / 7.0;
    let high = first * ((2.0 * magic + int) / 100.0 + 1.0);
    let low = first * ((2.0 * magic * factor + int) / 100.0 + 1.0);
    (low.min(high), high)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn foe(level: u32, hp: u32, defence: u32, avoid: u32) -> MobRow {
        MobRow {
            id: 1,
            name: "Foe".into(),
            level,
            hp,
            exp: 50,
            count: 10,
            boss: false,
            undead: false,
            weapon_def: Some(defence),
            magic_def: Some(defence),
            evasion: Some(avoid),
        }
    }

    fn fighter(low: f64, high: f64) -> Fighter {
        Fighter {
            level: 30,
            low,
            high,
            lines: 1,
            targets: 1,
            accuracy: None,
            magic: false,
            crit_rate: 0.0,
            crit_bonus: 0.0,
            attack_seconds: 0.8,
        }
    }

    #[test]
    fn the_wikis_hit_chance_example() {
        // A level 50 Assassin with 89 accuracy against a level 60 Red Drake (avoid 34) lands
        // "about 14%" of swings.
        let p = hit_chance(89.0, 50, 60, 34.0);
        assert!((p - 0.14).abs() < 0.01, "{p}");
        assert_eq!(hit_chance(200.0, 30, 20, 13.0), 1.0, "far more accuracy than avoid always lands");
        assert_eq!(hit_chance(10.0, 30, 30, 0.0), 1.0, "nothing to avoid with");
    }

    #[test]
    fn the_wikis_level_penalty_table() {
        assert!((level_factor(30, 35) - 0.889).abs() < 0.001);
        assert!((level_factor(30, 40) - 0.667).abs() < 0.001);
        assert!((level_factor(30, 50) - 0.5).abs() < 0.001);
        assert_eq!(level_factor(30, 20), 1.0, "no bonus below the player's level");
    }

    #[test]
    fn one_hit_kills_and_the_breakpoint_between() {
        let sure = duel(&fighter(100.0, 100.0), &foe(30, 50, 0, 0), 100.0);
        assert_eq!(sure.attacks, 1.0);
        // Blows of 40 to 140 against 100 HP: one blow does it 40% of the time, two nearly always.
        let split = duel(&fighter(40.0, 140.0), &foe(30, 100, 0, 0), 100.0);
        assert!((1.55..1.70).contains(&split.attacks), "{}", split.attacks);
    }

    #[test]
    fn defence_divides_and_misses_add_attacks() {
        let plain = duel(&fighter(100.0, 100.0), &foe(30, 1000, 0, 0), 100.0);
        let armoured = duel(&fighter(100.0, 100.0), &foe(30, 1000, 100, 0), 100.0);
        assert_eq!(plain.attacks, 10.0);
        assert_eq!(armoured.attacks, 20.0, "a defence of K halves each blow");
        let mut sharp = fighter(100.0, 100.0);
        sharp.accuracy = Some(89.0);
        let missing = duel(&sharp, &foe(30, 1000, 0, 34), 100.0);
        assert!(missing.hit < 1.0 && missing.attacks > 10.0, "{missing:?}");
        sharp.magic = true;
        assert_eq!(duel(&sharp, &foe(30, 1000, 0, 34), 100.0).hit, 1.0, "spells never miss");
    }

    #[test]
    fn the_spawn_limit_eases_in() {
        assert!((kills_per_hour(100.0, 10_000.0) - 100.0).abs() < 0.01);
        assert!(kills_per_hour(1000.0, 1000.0) < 800.0);
        let measured = kills_per_hour(300.0, 1000.0);
        assert!((free_kills(measured, 1000.0).unwrap() - 300.0).abs() < 0.01);
        assert_eq!(free_kills(950.0, 1000.0), None);
    }

    #[test]
    fn attack_times_follow_the_wikis_table() {
        assert!((attack_seconds(Some("One-handed Sword"), Some(5), false) - 0.75).abs() < 1e-9);
        assert!((attack_seconds(Some("Crossbow"), Some(6), false) - 0.84).abs() < 1e-9);
        assert!((attack_seconds(Some("Spear"), Some(8), false) - 0.87).abs() < 1e-9);
        assert!((attack_seconds(Some("Bow"), Some(3), false) - 0.66).abs() < 1e-9);
        assert!((attack_seconds(Some("Staff"), Some(7), true) - 0.87).abs() < 1e-9);
        assert!((attack_seconds(None, None, false) - 0.81).abs() < 1e-9);
    }

    #[test]
    fn the_fit_finds_how_the_player_plays() {
        // Play made by a known tuning, on maps of different toughness and walking.
        let truth = Tuning { efficiency: 0.6, overhead: 2.5, defence: 100.0 };
        let maps: Vec<(Vec<MobRow>, f64)> = vec![
            (vec![foe(25, 400, 0, 0)], 1.0),
            (vec![foe(28, 900, 10, 0)], 3.0),
            (vec![foe(30, 1500, 30, 0)], 2.0),
            (vec![foe(32, 2500, 0, 0)], 4.0),
        ];
        let me = fighter(150.0, 300.0);
        let observations: Vec<Observation> = maps
            .iter()
            .map(|(mobs, walk)| {
                let b = battle(&me, mobs, truth.defence, 3.0).unwrap();
                let limit = spawn_limit(b.count, 8.0);
                let free = 3600.0 / kill_seconds(&b, &truth, *walk);
                Observation {
                    fighter: me,
                    mobs,
                    crowd: 3.0,
                    walk_seconds: *walk,
                    limit,
                    kills_per_hour: kills_per_hour(free, limit),
                    seconds: 1200.0,
                }
            })
            .collect();
        let fit = fit(&observations);
        assert!((fit.tuning.efficiency - 0.6).abs() < 0.05, "{fit:?}");
        assert!((fit.tuning.overhead - 2.5).abs() < 0.5, "{fit:?}");
        assert_eq!(fit.stretches, 4);
        assert!(fit.spread <= 0.16, "{fit:?}");
        assert_eq!(super::fit(&[]), Fit::unfitted());
    }

    #[test]
    fn a_spell_range_from_the_wiki() {
        let (low, high) = spell_range(65.0, 200.0, 150.0, 10.0);
        assert!((high - (65.0 + 200.0 / 7.0) * 6.5).abs() < 1e-9);
        assert!(low < high && low > 0.0);
    }
}
