//! Each map's layout: its size, how much floor it has, and how its monsters spread over its
//! platforms. OSMS doesn't publish these, so they come from `client/maps.json` in Leafline's data
//! repository, read from the game client by map id. Downloaded whole like the other reference
//! data and checked once a day; nothing about the player goes with the request.
//!
//! Two things use them: training estimates, for the walking between kills, and routes, for
//! roughly how long the walk takes.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::atlas::Route;
use super::drops::{reason, recent_failure};
use super::fetch::Source;
use super::store::{LAYOUTS_FAILED, Store};
use crate::error::{Error, Result};

/// Where the layout file lives.
pub const LAYOUTS_URL: &str = "https://raw.githubusercontent.com/Crazy3001/Leafline-data/main/client/maps.json";
/// The only `schema` this reader understands.
const SCHEMA: u32 = 1;
/// The file only changes when the game is patched, so it is checked at most once a day.
pub const MAX_AGE: i64 = 24 * 60 * 60;

/// How fast a character walks, in the game's pixels a second, at base speed. An assumption:
/// speed items and skills change it.
pub const WALK_SPEED: f64 = 125.0;
/// Of the floor each monster has to itself, how much is walked to reach the next one. Monsters
/// wander and several come into range at once, so less than all of it.
const WALK_SHARE: f64 = 0.5;
/// Seconds lost going up or down to another platform, by rope, ladder or jump.
const CLIMB_SECONDS: f64 = 2.0;

/// One map's layout, in the game's pixels.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct MapLayout {
    #[serde(rename(deserialize = "id"))]
    pub map: u32,
    pub width: Option<u32>,
    pub height: Option<u32>,
    /// Summed horizontal length of every floor.
    pub walkable: u32,
    /// Connected pieces of floor.
    pub platforms: u32,
    pub spawn_points: u32,
    /// How many platforms monsters spawn on.
    pub spawn_platforms: u32,
    #[serde(default)]
    pub ropes: u32,
    #[serde(default)]
    pub ladders: u32,
}

impl MapLayout {
    /// Seconds of walking and climbing between one kill and the next, on average: half the floor
    /// each monster has to itself, plus a climb for each move to another platform monsters are
    /// on. None for a map without spawns.
    pub fn walk_seconds_per_kill(&self) -> Option<f64> {
        if self.spawn_points == 0 {
            return None;
        }
        let points = self.spawn_points as f64;
        let floor = self.walkable as f64 / points;
        let climbs = self.spawn_platforms.saturating_sub(1) as f64 / points;
        Some(floor * WALK_SHARE / WALK_SPEED + climbs * CLIMB_SECONDS)
    }
}

/// Every map's layout, by map id.
pub type Layouts = HashMap<u32, MapLayout>;

/// About how long a route takes on foot: half of each map's width walked for every portal taken,
/// since where a portal stands against the one arrived by isn't known closely. Rides and scrolls
/// count as no walking. None when a map on the way has no layout.
pub fn route_walk_seconds(route: &Route, layouts: &Layouts) -> Option<f64> {
    let mut pixels = 0.0;
    for step in route.steps.iter().filter(|s| s.travel.is_none()) {
        pixels += layouts.get(&step.from.id)?.width? as f64 / 2.0;
    }
    Some(pixels / WALK_SPEED)
}

/// Downloads the file if there is no copy or it is over a day old, and says whether it did. A
/// failed download keeps the old copy and is not tried again for a day.
pub fn refresh(store: &mut Store, source: &dyn Source, now: i64) -> Result<bool> {
    let fetched_at = store.layouts_fetched_at()?;
    if fetched_at.is_some_and(|at| now - at < MAX_AGE) {
        return Ok(false);
    }
    if let Some(reason) = recent_failure(store, LAYOUTS_FAILED, now)? {
        return if fetched_at.is_some() { Ok(false) } else { Err(Error::Download(reason)) };
    }
    match source.get(LAYOUTS_URL).and_then(|body| parse(&body)) {
        Ok(rows) => {
            store.replace_layouts(&rows, now)?;
            Ok(true)
        }
        Err(err) => {
            store.set_failure(LAYOUTS_FAILED, Some((now, &reason(&err))))?;
            if fetched_at.is_none() {
                return Err(err);
            }
            eprintln!("map layout update failed: {err}");
            Ok(false)
        }
    }
}

fn parse(body: &[u8]) -> Result<Vec<MapLayout>> {
    #[derive(Deserialize)]
    struct File {
        schema: u32,
        maps: Vec<MapLayout>,
    }
    let file: File = serde_json::from_slice(body)
        .map_err(|err| Error::Download(format!("Leafline's map layout file can't be read: {err}")))?;
    if file.schema != SCHEMA {
        return Err(Error::Download(format!(
            "Leafline's map layout file is in format {}; this version reads format {SCHEMA}. Update Leafline.",
            file.schema
        )));
    }
    if file.maps.is_empty() {
        return Err(Error::Download("Leafline's map layout file lists no maps".into()));
    }
    Ok(file.maps)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mapdata::drops::RETRY_AFTER;
    use crate::mapdata::fetch::tests::FakeSource;

    // Made-up maps in the data repository's shape.
    const FILE: &str = r#"{"schema": 1, "about": "test", "maps": [
        {"id": 1, "width": 1000, "height": 600, "walkable": 4000, "platforms": 1, "spawn_points": 20,
         "spawn_platforms": 1, "ropes": 0, "ladders": 0},
        {"id": 2, "width": 2500, "height": 2000, "walkable": 10000, "platforms": 12, "spawn_points": 40,
         "spawn_platforms": 9, "ropes": 5, "ladders": 2},
        {"id": 3, "width": 3000, "height": 900, "walkable": 3000, "platforms": 3, "spawn_points": 0,
         "spawn_platforms": 0}
    ]}"#;

    fn source(body: &str) -> FakeSource {
        let mut source = FakeSource::default();
        source.files.insert(LAYOUTS_URL.to_string(), body.as_bytes().to_vec());
        source
    }

    fn layouts() -> Layouts {
        parse(FILE.as_bytes()).unwrap().into_iter().map(|l| (l.map, l)).collect()
    }

    #[test]
    fn a_flat_map_walks_less_between_kills_than_a_spread_one() {
        let all = layouts();
        let flat = all[&1].walk_seconds_per_kill().unwrap();
        let spread = all[&2].walk_seconds_per_kill().unwrap();
        assert!((flat - 200.0 * 0.5 / WALK_SPEED).abs() < 1e-9, "{flat}");
        assert!(spread > flat, "{spread} vs {flat}");
        assert_eq!(all[&3].walk_seconds_per_kill(), None, "no spawns, nothing to walk between");
    }

    #[test]
    fn stores_and_reloads_the_file() {
        let mut store = Store::open_in_memory().unwrap();
        assert!(refresh(&mut store, &source(FILE), 1000).unwrap());
        assert_eq!(store.layouts().unwrap(), layouts());
        assert!(!refresh(&mut store, &source(FILE), 1000 + MAX_AGE - 1).unwrap(), "served from the copy");
    }

    #[test]
    fn keeps_the_old_copy_and_waits_a_day_after_a_failure() {
        let mut store = Store::open_in_memory().unwrap();
        refresh(&mut store, &source(FILE), 1000).unwrap();
        let offline = FakeSource::default();
        assert!(!refresh(&mut store, &offline, 1000 + MAX_AGE).unwrap());
        assert!(!refresh(&mut store, &offline, 1000 + MAX_AGE + RETRY_AFTER - 1).unwrap());
        assert_eq!(offline.requests.borrow().len(), 1);
        assert_eq!(store.layouts().unwrap().len(), 3);
    }

    #[test]
    fn refuses_a_format_it_does_not_know() {
        let mut store = Store::open_in_memory().unwrap();
        let err = refresh(&mut store, &source(r#"{"schema": 2, "maps": []}"#), 1000).unwrap_err();
        assert!(err.to_string().contains("Update Leafline"), "{err}");
    }

    #[test]
    fn a_route_on_foot_takes_half_of_each_map_it_crosses() {
        use crate::mapdata::atlas::tests::world;
        use crate::mapdata::travel::Travel;
        let route = world().route_with(5, 3, &Travel::none()).unwrap();
        assert_eq!(route.steps.len(), 3);
        let wide = |id| MapLayout { map: id, width: Some(1000), height: Some(600), walkable: 0, platforms: 1,
            spawn_points: 0, spawn_platforms: 0, ropes: 0, ladders: 0 };
        let mut all: Layouts = (1..=9).map(|id| (id, wide(id))).collect();
        let seconds = route_walk_seconds(&route, &all).unwrap();
        assert!((seconds - 3.0 * 500.0 / WALK_SPEED).abs() < 1e-9, "{seconds}");
        all.remove(&route.steps[1].from.id);
        assert_eq!(route_walk_seconds(&route, &all), None, "a map without a layout leaves it unknown");
    }
}
