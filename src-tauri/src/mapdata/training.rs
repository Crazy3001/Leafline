//! Training spots: every map with monsters to hunt, with the base numbers a player compares
//! them by, the EXP per hour the player measured there themselves, and for the rest an estimate
//! from how fast the player gets through monster HP. Nothing here models the game's damage
//! formulas; the estimate scales the player's own solo play by the map data's HP and EXP.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use super::layout::{Layouts, MapLayout};
use super::atlas::{Atlas, MapMatch, MapRef, MobRow, Shortcut};
use super::travel::Travel;

/// EXP per hour the session tracker measured on a map, keyed by the minimap label it read.
/// One entry per map, level and party size of a session, or a map's total.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeasuredRate {
    /// `Street : Name`, as the session tracker keys maps.
    pub label: String,
    pub exp_per_hour: f64,
    pub active_seconds: f64,
    /// The character's level then, when it was read.
    #[serde(default)]
    pub level: Option<u32>,
    /// Party size then; none or one is solo.
    #[serde(default)]
    pub party: Option<u32>,
}

/// What the estimates take as given because the map data doesn't say. The player can change
/// both; neither is a known fact about Classic.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Assumptions {
    /// Seconds before a defeated monster is back. Only the ceiling on kills depends on it.
    pub respawn_seconds: f64,
    /// EXP a kill gives over the data's base EXP, for events or server rates.
    pub exp_rate: f64,
}

impl Default for Assumptions {
    fn default() -> Self {
        Self { respawn_seconds: 8.0, exp_rate: 1.0 }
    }
}

/// How fast the player gets through monster HP, from their own solo play.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KillSpeed {
    pub hp_per_hour: f64,
    /// HP of the average monster the player defeated, weighted the same way.
    pub hp_per_kill: f64,
    pub active_seconds: f64,
    pub maps: u32,
    /// The lowest and highest character level in that play, when levels were read.
    pub levels: Option<(u32, u32)>,
    /// Too little was measured near the current level, so every solo session counts.
    pub widened: bool,
    /// Seconds of walking between kills on the maps that play was on, from their layouts. None
    /// when too little of the play was on maps with a known layout.
    pub walk_seconds_per_kill: Option<f64>,
}

/// A map's EXP per hour for this player, worked out from their [`KillSpeed`].
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Estimate {
    pub exp_per_hour: f64,
    pub kills_per_hour: f64,
    /// Kills an hour if every monster on the map were defeated as soon as it came back.
    pub ceiling: f64,
    /// The player would get through monsters faster than they come back, so the estimate is
    /// held to the ceiling.
    pub capped: bool,
    /// HP of the average monster here over the average one in the player's measured play.
    pub hp_ratio: f64,
    /// Average monster level here minus the character's level, when the level is known.
    pub level_gap: Option<f64>,
    /// 3 fairly sure, 2 rough, 1 a guess.
    pub sureness: u8,
    /// Seconds of walking between kills here, when the layout of this map and of the maps the
    /// kill speed came from are known. The rest of each kill scales with HP.
    pub walk_seconds_per_kill: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrainingSpot {
    pub map: MapRef,
    pub region: Option<String>,
    pub level_range: (u32, u32),
    /// Monster level averaged over everything the map spawns.
    pub average_level: f64,
    pub mob_total: u32,
    /// Base EXP for killing every monster once, before any rate or party modifiers.
    pub exp_per_clear: u64,
    /// Total HP of every monster on the map.
    pub hp_per_clear: u64,
    pub mobs: Vec<MobRow>,
    /// Portals from the current map, when it is known and a route exists.
    pub hops: Option<u32>,
    /// Fewer steps with the travel the player turned on, when that is shorter.
    pub travel: Option<Shortcut>,
    /// Whether portals lead here from some town. Party quest stages and other instances
    /// entered through an NPC are not.
    pub reachable: bool,
    /// What the player measured here.
    pub measured: Option<Measured>,
    /// What the player's kill speed gives here, once there is enough solo play to go on.
    pub estimate: Option<Estimate>,
    /// The map's size, floors and how its monsters spread, when known.
    pub layout: Option<MapLayout>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Measured {
    pub exp_per_hour: f64,
    pub active_seconds: f64,
}

/// The player's kill speed, with what the estimates need beside it.
#[derive(Debug, Clone)]
pub struct Estimator {
    pub speed: KillSpeed,
    pub level: Option<u32>,
    pub assumptions: Assumptions,
}

/// Less solo play than this and there is no kill speed to go on.
const MIN_SECONDS: f64 = 10.0 * 60.0;
/// Less than this behind the kill speed and every estimate is at most rough.
const SURE_SECONDS: f64 = 30.0 * 60.0;
/// A stretch shorter than this is left out of the kill speed: a map passed through, or a party
/// that changed for a moment, says little about how fast the player kills.
const MIN_STRETCH_SECONDS: f64 = 60.0;
/// Play this many levels below the character's (or one above) counts as near its level.
const NEAR_LEVELS: u32 = 5;
/// At least this much of the measured play must be on maps with a known layout before walking is
/// taken apart from fighting.
const WALK_COVERAGE: f64 = 0.5;
/// However much walking the layouts suggest, fighting is kept to at least this share of a kill.
const MIN_FIGHT_SHARE: f64 = 0.25;

/// The average monster a map's spawns give, bosses left out: what a kill is worth.
struct PerKill {
    hp: f64,
    exp: f64,
    level: f64,
    count: u32,
}

fn per_kill(mobs: &[MobRow]) -> Option<PerKill> {
    let common: Vec<&MobRow> = mobs.iter().filter(|m| !m.boss && m.count > 0).collect();
    let count: u32 = common.iter().map(|m| m.count).sum();
    if count == 0 {
        return None;
    }
    let mean = |value: fn(&MobRow) -> f64| common.iter().map(|m| value(m) * m.count as f64).sum::<f64>() / count as f64;
    let kill =
        PerKill { hp: mean(|m| m.hp as f64), exp: mean(|m| m.exp as f64), level: mean(|m| m.level as f64), count };
    (kill.hp > 0.0 && kill.exp > 0.0).then_some(kill)
}

impl Atlas {
    /// Every map outside town with at least one monster that isn't a boss. `from` is the map
    /// the player is on; `measured` the rates the player recorded, matched to maps by name;
    /// `travel` the rides and scrolls turned on; `estimator` the player's kill speed, when there
    /// is one.
    pub fn training_spots(
        &self,
        from: Option<u32>,
        measured: &[MeasuredRate],
        travel: &Travel,
        estimator: Option<&Estimator>,
        layouts: &Layouts,
    ) -> Vec<TrainingSpot> {
        let hops = from.map(|id| self.hops_from(id)).unwrap_or_default();
        let shortcuts = from.map(|id| self.shortcuts_from(id, travel, &hops)).unwrap_or_default();
        let mut reachable = HashSet::new();
        for town in self.maps.values().filter(|m| m.is_town) {
            if !reachable.contains(&town.id) {
                reachable.extend(self.hops_from(town.id).into_keys());
            }
        }
        // Nobody walks off the starting island or back to it, so where the player is decides
        // which side of the water counts.
        let on_island = from.is_some_and(|id| self.on_starting_island(id));
        let left_island = from.is_some() && !on_island;
        let rates = self.measured_by_map(measured);
        self.maps
            .values()
            .filter(|m| !m.is_town)
            .filter_map(|m| {
                let card = self.card(m.id)?;
                if card.mobs.is_empty() || card.mobs.iter().all(|mob| mob.boss) {
                    return None;
                }
                let level_range = card.level_range?;
                let total = card.mob_total.max(1) as f64;
                let average_level =
                    card.mobs.iter().map(|mob| mob.level as f64 * mob.count as f64).sum::<f64>() / total;
                Some(TrainingSpot {
                    hp_per_clear: card.mobs.iter().map(|mob| mob.hp as u64 * mob.count as u64).sum(),
                    estimate: estimator.and_then(|e| e.estimate(&card.mobs, layouts.get(&m.id))),
                    layout: layouts.get(&m.id).copied(),
                    map: card.map,
                    region: card.region,
                    level_range,
                    average_level,
                    mob_total: card.mob_total,
                    exp_per_clear: card.exp_per_clear,
                    mobs: card.mobs,
                    hops: hops.get(&m.id).copied(),
                    travel: shortcuts.get(&m.id).cloned(),
                    reachable: if on_island {
                        hops.contains_key(&m.id) || shortcuts.contains_key(&m.id)
                    } else {
                        reachable.contains(&m.id) && !(left_island && self.on_starting_island(m.id))
                    },
                    measured: rates.get(&m.id).cloned(),
                })
            })
            .collect()
    }

    /// The map a measured rate's label names. Labels repeat across a session's lines, so each
    /// is matched once.
    fn places<'a>(&self, measured: &'a [MeasuredRate]) -> HashMap<&'a str, Option<MapMatch>> {
        let mut found = HashMap::new();
        for rate in measured {
            found.entry(rate.label.as_str()).or_insert_with(|| {
                let (street, name) =
                    rate.label.split_once(" : ").map_or((None, rate.label.as_str()), |(s, n)| (Some(s), n));
                self.find(name, street)
            });
        }
        found
    }

    /// The measured rates by map ID. Several labels can land on one map (misreads): their
    /// time adds up and their rates are weighed by it.
    pub(super) fn measured_by_map(&self, measured: &[MeasuredRate]) -> HashMap<u32, Measured> {
        let places = self.places(measured);
        let mut totals: HashMap<u32, (f64, f64)> = HashMap::new();
        for rate in measured.iter().filter(|r| r.active_seconds > 0.0) {
            if let Some(Some(found)) = places.get(rate.label.as_str()) {
                let entry = totals.entry(found.card.map.id).or_default();
                entry.0 += rate.exp_per_hour * rate.active_seconds;
                entry.1 += rate.active_seconds;
            }
        }
        totals
            .into_iter()
            .map(|(id, (weighted, seconds))| {
                (id, Measured { exp_per_hour: weighted / seconds, active_seconds: seconds })
            })
            .collect()
    }

    /// How fast the player gets through monster HP, from their solo play on maps the data
    /// knows: each stretch's EXP per hour, over the base EXP of the map's average monster,
    /// gives kills an hour, and those times the average monster's HP give HP an hour. Play near
    /// `level` counts when there is enough of it, since damage grows with level; otherwise all
    /// solo play does. None with under ten minutes to go on.
    pub fn estimator(
        &self,
        measured: &[MeasuredRate],
        level: Option<u32>,
        assumptions: Assumptions,
        layouts: &Layouts,
    ) -> Option<Estimator> {
        struct Stretch {
            seconds: f64,
            hp_per_hour: f64,
            hp_per_kill: f64,
            level: Option<u32>,
            map: u32,
            walk: Option<f64>,
        }
        let exp_rate = if assumptions.exp_rate > 0.0 { assumptions.exp_rate } else { 1.0 };
        let places = self.places(measured);
        let all: Vec<Stretch> = measured
            .iter()
            .filter(|r| r.active_seconds >= MIN_STRETCH_SECONDS && r.exp_per_hour > 0.0 && r.party.unwrap_or(1) <= 1)
            .filter_map(|r| {
                let found = places.get(r.label.as_str())?.as_ref()?;
                let kill = per_kill(&found.card.mobs)?;
                let kills = r.exp_per_hour / (kill.exp * exp_rate);
                Some(Stretch {
                    seconds: r.active_seconds,
                    hp_per_hour: kills * kill.hp,
                    hp_per_kill: kill.hp,
                    level: r.level,
                    map: found.card.map.id,
                    walk: layouts.get(&found.card.map.id).and_then(|l| l.walk_seconds_per_kill()),
                })
            })
            .collect();
        let seconds = |set: &[&Stretch]| set.iter().map(|s| s.seconds).sum::<f64>();
        let near: Vec<&Stretch> = match level {
            Some(at) => all.iter().filter(|s| s.level.is_some_and(|l| l + NEAR_LEVELS >= at && l <= at + 1)).collect(),
            None => all.iter().collect(),
        };
        let (used, widened) = if seconds(&near) >= MIN_SECONDS {
            (near, false)
        } else {
            (all.iter().collect::<Vec<_>>(), level.is_some())
        };
        let total = seconds(&used);
        if total < MIN_SECONDS {
            return None;
        }
        let weighed = |value: fn(&Stretch) -> f64| used.iter().map(|s| value(s) * s.seconds).sum::<f64>() / total;
        let levels = used.iter().filter_map(|s| s.level);
        let walked: Vec<&&Stretch> = used.iter().filter(|s| s.walk.is_some()).collect();
        let walked_seconds: f64 = walked.iter().map(|s| s.seconds).sum();
        let walk_seconds_per_kill = (walked_seconds >= total * WALK_COVERAGE)
            .then(|| walked.iter().map(|s| s.walk.unwrap_or(0.0) * s.seconds).sum::<f64>() / walked_seconds);
        let speed = KillSpeed {
            hp_per_hour: weighed(|s| s.hp_per_hour),
            hp_per_kill: weighed(|s| s.hp_per_kill),
            active_seconds: total,
            maps: used.iter().map(|s| s.map).collect::<HashSet<_>>().len() as u32,
            levels: levels.clone().min().zip(levels.max()),
            widened,
            walk_seconds_per_kill,
        };
        Some(Estimator { speed, level, assumptions: Assumptions { exp_rate, ..assumptions } })
    }
}

impl Estimator {
    /// Kills an hour on a map from its average monster's HP, held to what the map can spawn
    /// in an hour at the assumed respawn time, then turned into EXP.
    pub fn estimate(&self, mobs: &[MobRow], layout: Option<&MapLayout>) -> Option<Estimate> {
        let kill = per_kill(mobs)?;
        let respawn = self.assumptions.respawn_seconds.max(1.0);
        let ceiling = kill.count as f64 * 3600.0 / respawn;
        let hp_ratio = kill.hp / self.speed.hp_per_kill;
        // With layouts on both sides, a measured kill is fighting plus walking: only the fighting
        // grows with the monsters' HP, and this map's own walking is added back.
        let walk_here = layout.and_then(|l| l.walk_seconds_per_kill());
        let (free, walk_seconds_per_kill) = match (self.speed.walk_seconds_per_kill, walk_here) {
            (Some(walked), Some(here)) if self.speed.hp_per_hour > 0.0 => {
                let measured_kill = 3600.0 * self.speed.hp_per_kill / self.speed.hp_per_hour;
                let fighting = (measured_kill - walked).max(measured_kill * MIN_FIGHT_SHARE);
                (3600.0 / (fighting * hp_ratio + here), Some(here))
            }
            _ => (self.speed.hp_per_hour / kill.hp, None),
        };
        let kills = free.min(ceiling);
        let capped = free > ceiling;
        let level_gap = self.level.map(|level| kill.level - level as f64);

        let mut sureness: i32 = 3;
        if self.speed.active_seconds < SURE_SECONDS {
            sureness -= 1;
        }
        if self.speed.widened {
            sureness -= 1;
        }
        // Much tougher monsters may take more hits than HP alone says (misses, knock-back);
        // much weaker ones may already fall to one hit, so getting through less HP is no faster.
        let spread = hp_ratio.max(1.0 / hp_ratio);
        if spread > 4.0 {
            sureness -= 2;
        } else if spread > 2.0 {
            sureness -= 1;
        }
        // Monsters above the character's level are harder to hit.
        if level_gap.is_some_and(|gap| gap > 3.0) {
            sureness -= 1;
        }
        // The ceiling rests on the assumed respawn time.
        if capped {
            sureness -= 1;
        }

        Some(Estimate {
            exp_per_hour: kills * kill.exp * self.assumptions.exp_rate,
            kills_per_hour: kills,
            ceiling,
            capped,
            hp_ratio,
            level_gap,
            sureness: sureness.clamp(1, 3) as u8,
            walk_seconds_per_kill,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mapdata::atlas::tests::{island_world, world};

    fn reachable(atlas: &Atlas, from: Option<u32>) -> Vec<String> {
        let spots = atlas.training_spots(from, &[], &Travel::none(), None, &Layouts::new());
        let mut names: Vec<String> = spots.into_iter().filter(|s| s.reachable).map(|s| s.map.name).collect();
        names.sort();
        names
    }

    fn rate(label: &str, exp_per_hour: f64, minutes: f64, level: Option<u32>) -> MeasuredRate {
        MeasuredRate { label: label.into(), exp_per_hour, active_seconds: minutes * 60.0, level, party: None }
    }

    #[test]
    fn the_starting_island_and_the_mainland_are_kept_apart() {
        let atlas = island_world();
        assert_eq!(reachable(&atlas, Some(20)), ["Cove Shore"]);
        assert_eq!(reachable(&atlas, Some(30)), ["Harbor Field"]);
        assert_eq!(reachable(&atlas, None), ["Cove Shore", "Harbor Field"]);
    }

    #[test]
    fn lists_hunting_maps_with_base_numbers() {
        let atlas = world();
        let spots = atlas.training_spots(Some(1), &[], &Travel::none(), None, &Layouts::new());
        let names: Vec<&str> = spots.iter().map(|s| s.map.name.as_str()).collect();
        assert_eq!(names, ["Mossbank Meadow I", "Mossbank Meadow II", "Burrow Den"]);

        let meadow = &spots[0];
        assert_eq!(meadow.level_range, (2, 5));
        assert_eq!(meadow.mob_total, 10);
        assert_eq!(meadow.exp_per_clear, 6 * 4 + 4 * 9);
        assert_eq!(meadow.hp_per_clear, 6 * 20 + 4 * 50);
        assert!((meadow.average_level - (6.0 * 2.0 + 4.0 * 5.0) / 10.0).abs() < 1e-9);
        assert_eq!(meadow.hops, Some(1));
        assert!(spots.iter().all(|s| s.reachable), "every map in the test world is walkable from Mossbank");
        assert!(meadow.measured.is_none() && meadow.estimate.is_none());
    }

    #[test]
    fn measured_rates_attach_to_the_map_they_name() {
        let atlas = world();
        let measured = [
            rate("Fern Road : Mossbank Meadow I", 1000.0, 10.0, None),
            // A misread of the same map, measured for longer.
            rate("Fern Road : Mossbank Meadovv I", 2000.0, 30.0, None),
            rate("Nowhere", 9.0, 1.0, None),
        ];
        let spots = atlas.training_spots(None, &measured, &Travel::none(), None, &Layouts::new());
        let meadow = spots.iter().find(|s| s.map.id == 2).unwrap();
        let rate = meadow.measured.as_ref().unwrap();
        assert_eq!(rate.active_seconds, 2400.0);
        assert!((rate.exp_per_hour - 1750.0).abs() < 1e-9);
        assert!(spots.iter().filter(|s| s.map.id != 2).all(|s| s.measured.is_none() && s.hops.is_none()));
    }

    #[test]
    fn kill_speed_turns_measured_exp_into_hp_an_hour() {
        let atlas = world();
        // Meadow I's average monster: 32 HP and 6 base EXP (6 snails, 4 slimes).
        let measured = [rate("Fern Road : Mossbank Meadow I", 600.0, 60.0, Some(5))];
        let estimator = atlas.estimator(&measured, Some(5), Assumptions::default(), &Layouts::new()).unwrap();
        let speed = &estimator.speed;
        assert!((speed.hp_per_kill - 32.0).abs() < 1e-9);
        // 600 EXP an hour is 100 kills, 3,200 HP.
        assert!((speed.hp_per_hour - 3200.0).abs() < 1e-9);
        assert_eq!((speed.maps, speed.levels, speed.widened), (1, Some((5, 5)), false));

        // Burrow Den: rats of 80 HP and 15 EXP. 3,200 HP is 40 rats, 600 EXP.
        let spots = atlas.training_spots(None, &measured, &Travel::none(), Some(&estimator), &Layouts::new());
        let den = spots.iter().find(|s| s.map.id == 4).unwrap().estimate.as_ref().unwrap();
        assert!((den.kills_per_hour - 40.0).abs() < 1e-9);
        assert!((den.exp_per_hour - 600.0).abs() < 1e-9);
        assert!(!den.capped);
        assert!((den.hp_ratio - 2.5).abs() < 1e-9);
        // An hour solo at the level, but rats with 2.5 times the HP: rough.
        assert_eq!(den.sureness, 2);
    }

    #[test]
    fn kills_are_held_to_what_the_map_can_respawn() {
        let atlas = world();
        let measured = [rate("Fern Road : Mossbank Meadow I", 60_000.0, 120.0, Some(5))];
        let slow = Assumptions { respawn_seconds: 60.0, exp_rate: 1.0 };
        let estimator = atlas.estimator(&measured, Some(5), slow, &Layouts::new()).unwrap();
        // Meadow II holds 2 snails: back once a minute, 120 kills an hour at most.
        let spots = atlas.training_spots(None, &measured, &Travel::none(), Some(&estimator), &Layouts::new());
        let small = spots.iter().find(|s| s.map.id == 3).unwrap().estimate.as_ref().unwrap();
        assert!(small.capped);
        assert!((small.ceiling - 120.0).abs() < 1e-9);
        assert!((small.exp_per_hour - 120.0 * 4.0).abs() < 1e-9);
    }

    #[test]
    fn an_exp_rate_only_moves_the_ceiling() {
        let atlas = world();
        let measured = [rate("Fern Road : Mossbank Meadow I", 1200.0, 60.0, None)];
        let base = atlas.estimator(&measured, None, Assumptions::default(), &Layouts::new()).unwrap();
        let doubled = atlas.estimator(&measured, None, Assumptions { exp_rate: 2.0, ..Assumptions::default() }, &Layouts::new()).unwrap();
        let den = |e: &Estimator| {
            let spots = atlas.training_spots(None, &measured, &Travel::none(), Some(e), &Layouts::new());
            spots.into_iter().find(|s| s.map.id == 4).unwrap().estimate.unwrap()
        };
        let (a, b) = (den(&base), den(&doubled));
        assert!((a.exp_per_hour - b.exp_per_hour).abs() < 1e-9);
        assert!((a.kills_per_hour - 2.0 * b.kills_per_hour).abs() < 1e-9);
    }

    #[test]
    fn party_play_and_short_play_give_no_kill_speed() {
        let atlas = world();
        let mut party = rate("Fern Road : Mossbank Meadow I", 600.0, 60.0, Some(5));
        party.party = Some(3);
        assert!(atlas.estimator(&[party], Some(5), Assumptions::default(), &Layouts::new()).is_none());
        let short = rate("Fern Road : Mossbank Meadow I", 600.0, 5.0, Some(5));
        assert!(atlas.estimator(&[short], Some(5), Assumptions::default(), &Layouts::new()).is_none());
    }

    #[test]
    fn brief_stretches_are_left_out_of_the_kill_speed() {
        let atlas = world();
        let mut measured = vec![rate("Fern Road : Mossbank Meadow I", 600.0, 60.0, Some(5))];
        // Thirty seconds passing through at a poor rate must not drag the kill speed down.
        measured.push(MeasuredRate { active_seconds: 30.0, exp_per_hour: 60.0, ..measured[0].clone() });
        let estimator = atlas.estimator(&measured, Some(5), Assumptions::default(), &Layouts::new()).unwrap();
        assert!((estimator.speed.hp_per_hour - 3200.0).abs() < 1e-9);
        assert!((estimator.speed.active_seconds - 3600.0).abs() < 1e-9);
    }

    #[test]
    fn play_near_the_level_wins_over_old_play() {
        let atlas = world();
        let measured = [
            rate("Fern Road : Mossbank Meadow I", 300.0, 120.0, Some(2)),
            rate("Fern Road : Mossbank Meadow I", 900.0, 20.0, Some(9)),
        ];
        let near = atlas.estimator(&measured, Some(10), Assumptions::default(), &Layouts::new()).unwrap();
        assert_eq!((near.speed.levels, near.speed.widened), (Some((9, 9)), false));
        assert!((near.speed.hp_per_hour - 900.0 / 6.0 * 32.0).abs() < 1e-9);
        // Nothing near Lv 30: every solo session counts, and the estimates say so.
        let far = atlas.estimator(&measured, Some(30), Assumptions::default(), &Layouts::new()).unwrap();
        assert_eq!((far.speed.levels, far.speed.widened), (Some((2, 9)), true));
        let spots = atlas.training_spots(None, &measured, &Travel::none(), Some(&far), &Layouts::new());
        let meadow = spots.iter().find(|s| s.map.id == 2).unwrap().estimate.as_ref().unwrap();
        assert_eq!(meadow.sureness, 2);
    }

    #[test]
    fn walking_between_kills_is_taken_apart_from_fighting() {
        let atlas = world();
        let measured = [rate("Fern Road : Mossbank Meadow I", 600.0, 60.0, Some(5))];
        let layout = |map, walkable, spawn_points, spawn_platforms| MapLayout {
            map, width: Some(1000), height: Some(600), walkable, platforms: spawn_platforms, spawn_points,
            spawn_platforms, ropes: 0, ladders: 0,
        };
        let den = |layouts: &Layouts| {
            let estimator = atlas.estimator(&measured, Some(5), Assumptions::default(), layouts).unwrap();
            let spots = atlas.training_spots(None, &measured, &Travel::none(), Some(&estimator), layouts);
            spots.into_iter().find(|s| s.map.id == 4).unwrap().estimate.unwrap()
        };
        // Meadow I: 100 kills an hour, 36 s each, 0.8 s of it walking (200 px of floor a monster).
        let flat: Layouts = [(2, layout(2, 2000, 10, 1)), (4, layout(4, 2000, 10, 1))].into();
        let same = den(&flat);
        assert_eq!(same.walk_seconds_per_kill, Some(0.8));
        // Only the 35.2 s of fighting grows with the rats' 2.5 times the HP.
        assert!((same.kills_per_hour - 3600.0 / (35.2 * 2.5 + 0.8)).abs() < 1e-9, "{}", same.kills_per_hour);

        // The Den spread over 6 platforms with twice the floor: more walking, fewer kills.
        let spread: Layouts = [(2, layout(2, 2000, 10, 1)), (4, layout(4, 4000, 10, 6))].into();
        assert!(den(&spread).kills_per_hour < same.kills_per_hour);

        // Without layouts, HP alone, as before.
        let none = den(&Layouts::new());
        assert!((none.kills_per_hour - 40.0).abs() < 1e-9);
        assert_eq!(none.walk_seconds_per_kill, None);
    }
}
