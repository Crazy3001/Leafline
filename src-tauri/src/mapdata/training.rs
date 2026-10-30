//! Training spots: every map with monsters to hunt, with the base numbers a player compares
//! them by. Nothing here models the game's EXP or damage formulas; the numbers are the map
//! data's base values, plus the EXP per hour the player measured there themselves.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::atlas::{Atlas, MapRef, MobRow};

/// EXP per hour the session tracker measured on a map, keyed by the minimap label it read.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeasuredRate {
    /// `Street : Name`, as the session tracker keys maps.
    pub label: String,
    pub exp_per_hour: f64,
    pub active_seconds: f64,
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
    /// Whether portals lead here from some town. Party quest stages and other instances
    /// entered through an NPC are not.
    pub reachable: bool,
    /// What the player measured here.
    pub measured: Option<Measured>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Measured {
    pub exp_per_hour: f64,
    pub active_seconds: f64,
}

impl Atlas {
    /// Every map outside town with at least one monster that isn't a boss. `from` is the map
    /// the player is on; `measured` the rates the player recorded, matched to maps by name.
    pub fn training_spots(&self, from: Option<u32>, measured: &[MeasuredRate]) -> Vec<TrainingSpot> {
        let hops = from.map(|id| self.hops_from(id)).unwrap_or_default();
        let mut reachable = std::collections::HashSet::new();
        for town in self.maps.values().filter(|m| m.is_town) {
            if !reachable.contains(&town.id) {
                reachable.extend(self.hops_from(town.id).into_keys());
            }
        }
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
                    map: card.map,
                    region: card.region,
                    level_range,
                    average_level,
                    mob_total: card.mob_total,
                    exp_per_clear: card.exp_per_clear,
                    mobs: card.mobs,
                    hops: hops.get(&m.id).copied(),
                    reachable: reachable.contains(&m.id),
                    measured: rates.get(&m.id).cloned(),
                })
            })
            .collect()
    }

    /// The measured rates by map ID. Several labels can land on one map (misreads): their
    /// time adds up and their rates are weighed by it.
    pub(super) fn measured_by_map(&self, measured: &[MeasuredRate]) -> HashMap<u32, Measured> {
        let mut totals: HashMap<u32, (f64, f64)> = HashMap::new();
        for rate in measured.iter().filter(|r| r.active_seconds > 0.0) {
            let (street, name) =
                rate.label.split_once(" : ").map_or((None, rate.label.as_str()), |(s, n)| (Some(s), n));
            if let Some(found) = self.find(name, street) {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mapdata::atlas::tests::world;

    #[test]
    fn lists_hunting_maps_with_base_numbers() {
        let atlas = world();
        let spots = atlas.training_spots(Some(1), &[]);
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
        assert!(meadow.measured.is_none());
    }

    #[test]
    fn measured_rates_attach_to_the_map_they_name() {
        let atlas = world();
        let measured = [
            MeasuredRate { label: "Fern Road : Mossbank Meadow I".into(), exp_per_hour: 1000.0, active_seconds: 600.0 },
            // A misread of the same map, measured for longer.
            MeasuredRate {
                label: "Fern Road : Mossbank Meadovv I".into(),
                exp_per_hour: 2000.0,
                active_seconds: 1800.0,
            },
            MeasuredRate { label: "Nowhere".into(), exp_per_hour: 9.0, active_seconds: 60.0 },
        ];
        let spots = atlas.training_spots(None, &measured);
        let meadow = spots.iter().find(|s| s.map.id == 2).unwrap();
        let rate = meadow.measured.as_ref().unwrap();
        assert_eq!(rate.active_seconds, 2400.0);
        assert!((rate.exp_per_hour - 1750.0).abs() < 1e-9);
        assert!(spots.iter().filter(|s| s.map.id != 2).all(|s| s.measured.is_none() && s.hops.is_none()));
    }
}
