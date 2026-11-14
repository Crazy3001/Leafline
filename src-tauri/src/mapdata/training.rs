//! Training spots: every map with monsters to hunt, with the base numbers a player compares
//! them by, the EXP per hour the player measured there themselves, and for the rest an estimate
//! from how fast the player gets through monster HP. Nothing here models the game's damage
//! formulas; the estimate scales the player's own solo play by the map data's HP and EXP.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use super::layout::{Layouts, MapLayout};
use super::atlas::{Atlas, MapMatch, MapRef, MobRow, Shortcut};
use super::combat::{self, Battle, Fighter, Fit, Observation};
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
    /// Kills counted in that play, when they were.
    #[serde(default)]
    pub kills: Option<f64>,
    /// The character as they were then, when it was known.
    #[serde(default)]
    pub fighter: Option<Fighter>,
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
    /// The player's attack range from their stat window, when they entered it. With it the
    /// estimates take each monster's weapon defence into account.
    pub attack_min: Option<f64>,
    pub attack_max: Option<f64>,
    /// How hard a blow of their usual skill hits over a plain attack.
    pub skill_factor: f64,
}

impl Default for Assumptions {
    fn default() -> Self {
        Self { respawn_seconds: 8.0, exp_rate: 1.0, attack_min: None, attack_max: None, skill_factor: 2.4 }
    }
}

/// How much of a monster's weapon defence is taken off each blow. Fitted to a Lv 33 Crossbowman's
/// fights (attack 107-289, Arrow Blow Lv 20 at 240%): Wooden Masks (no defence) fell in one or two
/// blows, Iron Hogs (defence 500) took seven. The real formula isn't published, so this is a fit
/// from one player's fights.
const DEFENCE_CUT: f64 = 0.85;

/// What the player's blows do, for working out what a monster's defence costs them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Damage {
    low: f64,
    high: f64,
}

impl Damage {
    /// None unless the player has entered a sensible attack range.
    pub fn from_assumptions(a: &Assumptions) -> Option<Self> {
        let (low, high) = (a.attack_min?, a.attack_max?);
        if !(low > 0.0 && high >= low) {
            return None;
        }
        let factor = if a.skill_factor > 0.0 { a.skill_factor } else { 1.0 };
        Some(Self { low: low * factor, high: high * factor })
    }

    /// What a blow does on average to a monster with no defence.
    fn plain(&self) -> f64 {
        (self.low + self.high) / 2.0
    }

    /// What a blow does on average to a monster with this weapon defence: each roll between the
    /// range's ends loses `DEFENCE_CUT` of the defence, and none does less than 1.
    pub fn blow(&self, defence: f64) -> f64 {
        let cut = DEFENCE_CUT * defence.max(0.0);
        let (a, b) = (self.low, self.high);
        let floor = cut + 1.0;
        if b <= floor {
            return 1.0;
        }
        if b <= a {
            return (a - cut).max(1.0);
        }
        // Rolls under `floor` do 1; those over do the roll less the cut.
        let from = a.max(floor);
        let under = ((from - a) / (b - a)).clamp(0.0, 1.0);
        let over_mean = (from + b) / 2.0 - cut;
        under + (b - from) / (b - a) * over_mean
    }

    /// How many times as much HP a monster is to this player as one with no defence.
    fn weight(&self, defence: f64) -> f64 {
        self.plain() / self.blow(defence)
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
    /// Weapon defence of the monsters in that play, on average.
    pub defence: f64,
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
    /// Blows to defeat the average monster here, when the player's attack range is known.
    pub hits_per_kill: Option<f64>,
    /// Weapon defence of the average monster here.
    pub weapon_def: f64,
    /// Whether the player's attack was taken into account, so defence is counted.
    pub damage_known: bool,
    /// How the fight goes, when the estimate came from the combat model.
    pub fight: Option<FightDetail>,
}

/// What the combat model says of fighting a map's monsters.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FightDetail {
    /// Chance a blow lands, on average over the monsters.
    pub hit_chance: f64,
    /// Attacks a kill takes.
    pub attacks_per_kill: f64,
    /// Seconds a kill takes, everything counted, before the spawn limit.
    pub kill_seconds: f64,
    /// Monsters here on average above the player's level, when it is known.
    pub level_gap: Option<f64>,
}

/// What the player can expect on a map: their own measured play there, carried to their level
/// now, together with the model's estimate, each counting by how much there is behind it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Forecast {
    pub exp_per_hour: f64,
    /// Four times in five the real rate should land between these.
    pub low: f64,
    pub high: f64,
    /// 3 fairly sure, 2 rough, 1 a guess.
    pub sureness: u8,
    /// Solo seconds played here that count, near the current level.
    pub played_seconds: f64,
    /// The model's own estimate, before measured play is folded in.
    pub model_exp_per_hour: Option<f64>,
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
    /// The best guess at what the player gets here now, with a range.
    pub forecast: Option<Forecast>,
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
    /// The player's blows, when they entered their attack range.
    pub damage: Option<Damage>,
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
    /// HP, counted heavier for monsters that shrug off the player's blows when their attack is
    /// known.
    hp: f64,
    exp: f64,
    level: f64,
    count: u32,
    /// Blows to defeat it, when the player's attack is known.
    hits: Option<f64>,
    defence: f64,
}

fn per_kill(mobs: &[MobRow], damage: Option<&Damage>) -> Option<PerKill> {
    let common: Vec<&MobRow> = mobs.iter().filter(|m| !m.boss && m.count > 0).collect();
    let count: u32 = common.iter().map(|m| m.count).sum();
    if count == 0 {
        return None;
    }
    let defence = |m: &MobRow| m.weapon_def.unwrap_or(0) as f64;
    let mean = |value: &dyn Fn(&MobRow) -> f64| common.iter().map(|m| value(m) * m.count as f64).sum::<f64>() / count as f64;
    let kill = PerKill {
        hp: mean(&|m| m.hp as f64 * damage.map_or(1.0, |d| d.weight(defence(m)))),
        exp: mean(&|m| m.exp as f64),
        level: mean(&|m| m.level as f64),
        count,
        hits: damage.map(|d| mean(&|m| (m.hp as f64 / d.blow(defence(m))).max(1.0))),
        defence: mean(&defence),
    };
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
        model: Option<&Model>,
        layouts: &Layouts,
    ) -> Vec<TrainingSpot> {
        let carried = model.map(|m| self.carried_play(measured, m, layouts)).unwrap_or_default();
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
                let layout = layouts.get(&m.id);
                let modelled = model.and_then(|model| model.estimate(&card.mobs, layout));
                let estimate = modelled.as_ref().map(|(e, _)| e.clone()).or_else(|| estimator.and_then(|e| e.estimate(&card.mobs, layout)));
                let forecast = match (model, &modelled) {
                    (Some(model), Some((estimate, spread))) => Some(model.forecast(estimate, *spread, carried.get(&m.id))),
                    _ => None,
                };
                Some(TrainingSpot {
                    hp_per_clear: card.mobs.iter().map(|mob| mob.hp as u64 * mob.count as u64).sum(),
                    estimate,
                    forecast,
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
            defence: f64,
        }
        let exp_rate = if assumptions.exp_rate > 0.0 { assumptions.exp_rate } else { 1.0 };
        let damage = Damage::from_assumptions(&assumptions);
        let places = self.places(measured);
        let all: Vec<Stretch> = measured
            .iter()
            .filter(|r| r.active_seconds >= MIN_STRETCH_SECONDS && r.exp_per_hour > 0.0 && r.party.unwrap_or(1) <= 1)
            .filter_map(|r| {
                let found = places.get(r.label.as_str())?.as_ref()?;
                let kill = per_kill(&found.card.mobs, damage.as_ref())?;
                let kills = r.exp_per_hour / (kill.exp * exp_rate);
                Some(Stretch {
                    seconds: r.active_seconds,
                    hp_per_hour: kills * kill.hp,
                    hp_per_kill: kill.hp,
                    level: r.level,
                    map: found.card.map.id,
                    walk: layouts.get(&found.card.map.id).and_then(|l| l.walk_seconds_per_kill()),
                    defence: kill.defence,
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
            defence: weighed(|s| s.defence),
        };
        Some(Estimator { speed, level, assumptions: Assumptions { exp_rate, ..assumptions }, damage })
    }
}

impl Estimator {
    /// Kills an hour on a map from its average monster's HP, held to what the map can spawn
    /// in an hour at the assumed respawn time, then turned into EXP.
    pub fn estimate(&self, mobs: &[MobRow], layout: Option<&MapLayout>) -> Option<Estimate> {
        let kill = per_kill(mobs, self.damage.as_ref())?;
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
        // Without the player's attack, a monster far tougher in defence than the ones they
        // measured on may take many more blows than its HP says.
        if self.damage.is_none() && kill.defence - self.speed.defence > 100.0 {
            sureness -= 2;
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
            hits_per_kill: kill.hits,
            weapon_def: kill.defence,
            damage_known: self.damage.is_some(),
            fight: None,
        })
    }
}

/// Stretches of play shorter than this don't tune the model: a map passed through says little.
const TUNE_SECONDS: f64 = 300.0;
/// Seconds of play a model estimate counts for against the player's own play on a map.
const FORECAST_SECONDS: f64 = 1200.0;
/// Play without a record of the character then is taken as from the character now only this
/// many levels either side.
const SAME_LEVELS: u32 = 3;
/// When a map's layout isn't known: seconds of walking a kill, and monsters on a platform.
const DEFAULT_WALK: f64 = 2.0;
const DEFAULT_CROWD: f64 = 3.0;
/// 1.28 standard deviations either side holds four in five.
const RANGE_Z: f64 = 1.28;

/// How many monsters stand together on a platform, for attacks that reach several.
fn crowd(layout: Option<&MapLayout>) -> f64 {
    layout
        .filter(|l| l.spawn_platforms > 0 && l.spawn_points > 0)
        .map_or(DEFAULT_CROWD, |l| f64::from(l.spawn_points) / f64::from(l.spawn_platforms))
}

fn walk(layout: Option<&MapLayout>) -> f64 {
    layout.and_then(MapLayout::walk_seconds_per_kill).unwrap_or(DEFAULT_WALK)
}

/// The combat model for the character now, tuned to their play.
#[derive(Debug, Clone)]
pub struct Model {
    pub fighter: Fighter,
    pub fit: Fit,
    pub assumptions: Assumptions,
}

/// The player's own solo play on one map, carried to their level now.
#[derive(Debug, Clone, Copy, Default)]
pub struct Carried {
    /// EXP an hour as it would be now, weighted by seconds.
    pub exp_per_hour: f64,
    pub seconds: f64,
}

impl Model {
    fn exp_rate(&self) -> f64 {
        if self.assumptions.exp_rate > 0.0 { self.assumptions.exp_rate } else { 1.0 }
    }

    /// EXP an hour for `fighter` on a map, with what the fight takes.
    fn rate(&self, fighter: &Fighter, mobs: &[MobRow], layout: Option<&MapLayout>) -> Option<(Battle, f64, f64, f64)> {
        let battle = combat::battle(fighter, mobs, self.fit.tuning.defence, crowd(layout))?;
        let limit = combat::spawn_limit(battle.count, self.assumptions.respawn_seconds);
        let seconds = combat::kill_seconds(&battle, &self.fit.tuning, walk(layout));
        let kills = combat::kills_per_hour(3600.0 / seconds, limit);
        Some((battle, kills, limit, seconds))
    }

    /// The model's estimate for a map, and how far off it may be (the standard deviation of its
    /// logarithm).
    pub fn estimate(&self, mobs: &[MobRow], layout: Option<&MapLayout>) -> Option<(Estimate, f64)> {
        let (battle, kills, limit, seconds) = self.rate(&self.fighter, mobs, layout)?;
        let free = 3600.0 / seconds;
        let capped = free >= limit;
        let level_gap = Some(battle.level - f64::from(self.fighter.level));
        let mut spread = self.fit.spread;
        // Each of these is a place the rules are least sure of.
        if battle.hit < 0.9 {
            spread += 0.1;
        }
        if free > 0.8 * limit {
            spread += 0.1;
        }
        if level_gap.is_some_and(|g| g > 5.0) {
            spread += 0.1;
        }
        let estimate = Estimate {
            exp_per_hour: kills * battle.exp_per_kill * self.exp_rate(),
            kills_per_hour: kills,
            ceiling: limit,
            capped,
            hp_ratio: 1.0,
            level_gap,
            sureness: sureness(spread),
            walk_seconds_per_kill: layout.and_then(MapLayout::walk_seconds_per_kill),
            hits_per_kill: Some(battle.attacks * battle.hit * f64::from(self.fighter.lines.max(1))),
            weapon_def: mobs
                .iter()
                .filter(|m| !m.boss && m.count > 0)
                .map(|m| f64::from(m.weapon_def.unwrap_or(0)) * f64::from(m.count))
                .sum::<f64>()
                / f64::from(battle.count),
            damage_known: true,
            fight: Some(FightDetail {
                hit_chance: battle.hit,
                attacks_per_kill: battle.attacks,
                kill_seconds: seconds,
                level_gap,
            }),
        };
        Some((estimate, spread))
    }

    /// The estimate with the player's own play on the map folded in: each counts by the time
    /// behind it, the model for [`FORECAST_SECONDS`].
    pub fn forecast(&self, estimate: &Estimate, spread: f64, played: Option<&Carried>) -> Forecast {
        let model = estimate.exp_per_hour;
        let (rate, seconds) = played.filter(|p| p.seconds > 0.0).map_or((0.0, 0.0), |p| (p.exp_per_hour, p.seconds));
        let exp_per_hour = (rate * seconds + model * FORECAST_SECONDS) / (seconds + FORECAST_SECONDS);
        // Measured play narrows the range as it outweighs the model, down to the spread of play
        // itself.
        let spread = (spread * FORECAST_SECONDS / (seconds + FORECAST_SECONDS)).max(0.08);
        Forecast {
            exp_per_hour,
            low: exp_per_hour * (-RANGE_Z * spread).exp(),
            high: exp_per_hour * (RANGE_Z * spread).exp(),
            sureness: sureness(spread),
            played_seconds: seconds,
            model_exp_per_hour: Some(model),
        }
    }
}

fn sureness(spread: f64) -> u8 {
    if spread < 0.2 {
        3
    } else if spread < 0.35 {
        2
    } else {
        1
    }
}

impl Atlas {
    /// The combat model for `fighter`, the character now, tuned to their solo play: each stretch
    /// of five minutes or more on a map the data knows, with the character as they were then.
    /// Play recorded before that was kept counts only near the current level, as the character
    /// now. `None` without the character's damage.
    pub fn model(
        &self,
        measured: &[MeasuredRate],
        fighter: Option<Fighter>,
        assumptions: Assumptions,
        layouts: &Layouts,
    ) -> Option<Model> {
        let fighter = fighter?;
        let exp_rate = if assumptions.exp_rate > 0.0 { assumptions.exp_rate } else { 1.0 };
        let places = self.places(measured);
        let stretches: Vec<(Vec<MobRow>, Option<&MapLayout>, &MeasuredRate, Fighter)> = measured
            .iter()
            .filter(|r| r.active_seconds >= TUNE_SECONDS && r.exp_per_hour > 0.0 && r.party.unwrap_or(1) <= 1)
            .filter_map(|r| {
                let found = places.get(r.label.as_str())?.as_ref()?;
                let then = r.fighter.or_else(|| {
                    let near = r.level.is_some_and(|l| l.abs_diff(fighter.level) <= SAME_LEVELS);
                    near.then_some(fighter)
                })?;
                Some((found.card.mobs.clone(), layouts.get(&found.card.map.id), r, then))
            })
            .collect();
        let observations: Vec<Observation> = stretches
            .iter()
            .filter_map(|(mobs, layout, r, then)| {
                let exp_per_kill = {
                    let common: Vec<&MobRow> = mobs.iter().filter(|m| !m.boss && m.count > 0).collect();
                    let count: u32 = common.iter().map(|m| m.count).sum();
                    (count > 0).then(|| common.iter().map(|m| f64::from(m.exp * m.count)).sum::<f64>() / f64::from(count))?
                };
                let kills_per_hour = match r.kills.filter(|&k| k > 0.0) {
                    Some(kills) => kills * 3600.0 / r.active_seconds,
                    None => r.exp_per_hour / (exp_per_kill * exp_rate),
                };
                let count: u32 = mobs.iter().filter(|m| !m.boss).map(|m| m.count).sum();
                Some(Observation {
                    fighter: *then,
                    mobs,
                    crowd: crowd(*layout),
                    walk_seconds: walk(*layout),
                    limit: combat::spawn_limit(count, assumptions.respawn_seconds),
                    kills_per_hour,
                    seconds: r.active_seconds,
                })
            })
            .collect();
        let fit = combat::fit(&observations);
        Some(Model { fighter, fit, assumptions: Assumptions { exp_rate, ..assumptions } })
    }

    /// The player's solo play on each map, carried to the character now: a stretch with a record
    /// of the character then is scaled by how much faster the model says they are now; one
    /// without counts only near the current level.
    pub fn carried_play(&self, measured: &[MeasuredRate], model: &Model, layouts: &Layouts) -> HashMap<u32, Carried> {
        let places = self.places(measured);
        let mut out: HashMap<u32, Carried> = HashMap::new();
        for r in measured.iter().filter(|r| r.active_seconds >= MIN_STRETCH_SECONDS && r.exp_per_hour > 0.0 && r.party.unwrap_or(1) <= 1) {
            let Some(Some(found)) = places.get(r.label.as_str()) else { continue };
            let layout = layouts.get(&found.card.map.id);
            let scale = match r.fighter {
                Some(then) => {
                    let now = model.rate(&model.fighter, &found.card.mobs, layout).map(|x| x.1);
                    let before = model.rate(&then, &found.card.mobs, layout).map(|x| x.1);
                    match (now, before) {
                        (Some(now), Some(before)) if before > 0.0 => now / before,
                        _ => continue,
                    }
                }
                None if r.level.is_some_and(|l| l.abs_diff(model.fighter.level) <= SAME_LEVELS) => 1.0,
                None => continue,
            };
            let entry = out.entry(found.card.map.id).or_default();
            let total = entry.seconds + r.active_seconds;
            entry.exp_per_hour = (entry.exp_per_hour * entry.seconds + r.exp_per_hour * scale * r.active_seconds) / total;
            entry.seconds = total;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mapdata::atlas::tests::{island_world, world};

    fn reachable(atlas: &Atlas, from: Option<u32>) -> Vec<String> {
        let spots = atlas.training_spots(from, &[], &Travel::none(), None, None, &Layouts::new());
        let mut names: Vec<String> = spots.into_iter().filter(|s| s.reachable).map(|s| s.map.name).collect();
        names.sort();
        names
    }

    fn rate(label: &str, exp_per_hour: f64, minutes: f64, level: Option<u32>) -> MeasuredRate {
        MeasuredRate { label: label.into(), exp_per_hour, active_seconds: minutes * 60.0, level, party: None, kills: None, fighter: None }
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
        let spots = atlas.training_spots(Some(1), &[], &Travel::none(), None, None, &Layouts::new());
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
        let spots = atlas.training_spots(None, &measured, &Travel::none(), None, None, &Layouts::new());
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
        let spots = atlas.training_spots(None, &measured, &Travel::none(), Some(&estimator), None, &Layouts::new());
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
        let slow = Assumptions { respawn_seconds: 60.0, ..Assumptions::default() };
        let estimator = atlas.estimator(&measured, Some(5), slow, &Layouts::new()).unwrap();
        // Meadow II holds 2 snails: back once a minute, 120 kills an hour at most.
        let spots = atlas.training_spots(None, &measured, &Travel::none(), Some(&estimator), None, &Layouts::new());
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
            let spots = atlas.training_spots(None, &measured, &Travel::none(), Some(e), None, &Layouts::new());
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

    fn armoured(defence: u32) -> Vec<MobRow> {
        vec![MobRow { id: 1, name: "Iron Hog".into(), level: 28, hp: 572, exp: 52, count: 20, boss: false, undead: false, weapon_def: Some(defence), magic_def: Some(100), evasion: Some(17) }]
    }

    fn estimator_with(attack: Option<(f64, f64)>) -> Estimator {
        let assumptions = Assumptions { attack_min: attack.map(|a| a.0), attack_max: attack.map(|a| a.1), ..Assumptions::default() };
        Estimator {
            speed: KillSpeed {
                hp_per_hour: 475_000.0,
                hp_per_kill: 503.0,
                active_seconds: 3600.0,
                maps: 2,
                levels: Some((32, 33)),
                widened: false,
                walk_seconds_per_kill: None,
                defence: 10.0,
            },
            level: Some(33),
            damage: Damage::from_assumptions(&assumptions),
            assumptions,
        }
    }

    #[test]
    fn defence_costs_blows_as_a_lv33_crossbowman_saw() {
        let damage = Damage::from_assumptions(&Assumptions {
            attack_min: Some(107.0),
            attack_max: Some(289.0),
            ..Assumptions::default()
        })
        .unwrap();
        // Wooden Masks (462 HP, no defence) fell in one or two blows.
        let mask = 462.0 / damage.blow(0.0);
        assert!((0.8..2.2).contains(&mask), "{mask}");
        // Iron Hogs (572 HP, defence 500) took seven.
        let hog_blow = damage.blow(500.0);
        assert!((60.0..110.0).contains(&hog_blow), "{hog_blow}");
        assert!((5.0..9.5).contains(&(572.0 / hog_blow)));
        // No blow does less than 1.
        assert_eq!(damage.blow(5000.0), 1.0);
    }

    #[test]
    fn armoured_monsters_rank_below_soft_ones_once_the_attack_is_known() {
        let aware = estimator_with(Some((107.0, 289.0)));
        let blind = estimator_with(None);
        let soft = aware.estimate(&armoured(0), None).unwrap();
        let hog = aware.estimate(&armoured(500), None).unwrap();
        assert!(hog.exp_per_hour < soft.exp_per_hour / 3.0, "{} vs {}", hog.exp_per_hour, soft.exp_per_hour);
        assert!(hog.hits_per_kill.unwrap() > 5.0 && soft.hits_per_kill.unwrap() < 3.0);
        assert!(hog.damage_known);
        // Without the attack the estimate can't see it, and says so by being less sure.
        let unaware = blind.estimate(&armoured(500), None).unwrap();
        assert!(!unaware.damage_known && unaware.hits_per_kill.is_none());
        assert!(unaware.sureness < blind.estimate(&armoured(0), None).unwrap().sureness);
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
        let spots = atlas.training_spots(None, &measured, &Travel::none(), Some(&far), None, &Layouts::new());
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
            let spots = atlas.training_spots(None, &measured, &Travel::none(), Some(&estimator), None, layouts);
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

    fn hero(level: u32) -> Fighter {
        Fighter {
            level,
            low: 100.0,
            high: 200.0,
            lines: 1,
            targets: 1,
            accuracy: None,
            magic: false,
            crit_rate: 0.0,
            crit_bonus: 0.0,
            attack_seconds: 0.8,
        }
    }

    const MEADOW: &str = "Fern Road : Mossbank Meadow I";

    fn forecast(atlas: &Atlas, measured: &[MeasuredRate], model: &Model) -> Forecast {
        let spots = atlas.training_spots(None, measured, &Travel::none(), None, Some(model), &Layouts::new());
        spots.into_iter().find(|s| s.map.name == "Mossbank Meadow I").and_then(|s| s.forecast).unwrap()
    }

    #[test]
    fn the_model_estimates_maps_never_played_and_play_pulls_its_map_towards_it() {
        let atlas = world();
        let model = atlas.model(&[], Some(hero(5)), Assumptions::default(), &Layouts::new()).unwrap();
        assert_eq!(model.fit.stretches, 0);
        let fresh = forecast(&atlas, &[], &model);
        assert_eq!((fresh.sureness, fresh.played_seconds), (1, 0.0));
        assert!(fresh.low < fresh.exp_per_hour && fresh.exp_per_hour < fresh.high);

        // An hour played there at this level, at twice the model's rate: the hour counts three
        // times as much as the model's twenty minutes.
        let r = fresh.exp_per_hour;
        let played = [rate(MEADOW, 2.0 * r, 60.0, Some(5))];
        let blended = forecast(&atlas, &played, &model);
        assert!((blended.exp_per_hour - 1.75 * r).abs() < 1e-6 * r, "{blended:?}");
        assert!(blended.high / blended.low < fresh.high / fresh.low, "play narrows the range");

        // Play long ago, with no record of the character then, says nothing about now.
        let old = [rate(MEADOW, 2.0 * r, 60.0, Some(1))];
        let model_later = atlas.model(&[], Some(hero(9)), Assumptions::default(), &Layouts::new()).unwrap();
        assert_eq!(forecast(&atlas, &old, &model_later).played_seconds, 0.0);
    }

    #[test]
    fn play_with_a_record_of_the_character_then_is_carried_to_now() {
        let atlas = world();
        let weak = Fighter { low: 20.0, high: 40.0, ..hero(3) };
        let model = atlas.model(&[], Some(hero(5)), Assumptions::default(), &Layouts::new()).unwrap();
        let then = MeasuredRate { fighter: Some(weak), ..rate(MEADOW, 1_000.0, 60.0, Some(3)) };
        let carried = atlas.carried_play(&[then], &model, &Layouts::new());
        let meadow = carried.values().next().unwrap();
        assert!(meadow.exp_per_hour > 1_000.0, "stronger now, so faster now: {meadow:?}");
    }
}
