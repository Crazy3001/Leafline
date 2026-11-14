//! The quickest way between two maps, timed by the walk across each one. OSMS places every
//! portal on its map, so the walk from the portal a player comes out of to the one they take next
//! can be measured, with climbing counted extra. Rides and scrolls take a fixed while instead, so
//! a route takes a taxi when that is quicker than the walk.
//!
//! Every speed and time here is an assumption about a character with no speed bonus; the point is
//! to rank routes, and the seconds shown are rough.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BinaryHeap, HashMap};

use super::atlas::{Atlas, Direction, Route, RouteStep, TravelStep};
use super::layout::WALK_SPEED;
use super::osms::{MapRecord, PointKind};
use super::travel::{Travel, TravelKind, TravelLink};

/// Going up a rope, ladder or slope against walking the same distance flat.
const CLIMB_FACTOR: f64 = 2.0;
/// Seconds a portal takes: pressing ↑ and the next map loading.
const PORTAL_SECONDS: f64 = 2.0;
/// The walk across a map whose portals have no places: about half of a middling map.
const UNPLACED_SECONDS: f64 = 10.0;

/// Seconds a ride takes, counting the walk to whoever gives it and the talk.
fn ride_seconds(kind: TravelKind) -> f64 {
    match kind {
        TravelKind::Cab | TravelKind::VipCab => 20.0,
        TravelKind::Scroll => 15.0,
        TravelKind::Npc => 30.0,
    }
}

/// A portal at its place on its map, in the game's pixels (y grows downward).
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Spot {
    /// The map it leads to; its own map for a teleport or where one comes out.
    pub to: u32,
    pub name: String,
    /// The portal it comes out at on the other map.
    pub to_portal: Option<String>,
    pub x: i32,
    pub y: i32,
}

/// Takes the portals' places out of the maps' points, by map. The other points (NPCs, spawns)
/// are left; the cache loads maps with portal points only.
pub(super) fn take_spots(maps: &mut BTreeMap<u32, MapRecord>) -> HashMap<u32, Vec<Spot>> {
    let mut spots = HashMap::new();
    for map in maps.values_mut() {
        let points = std::mem::take(&mut map.points);
        let (portals, rest): (Vec<_>, Vec<_>) = points.into_iter().partition(|p| p.kind == PointKind::Portal);
        map.points = rest;
        let list: Vec<Spot> = portals
            .into_iter()
            .filter_map(|p| Some(Spot { to: p.id, name: p.name?, to_portal: p.to_portal, x: p.x, y: p.y }))
            .collect();
        if !list.is_empty() {
            spots.insert(map.id, list);
        }
    }
    spots
}

/// Seconds to walk from one place to another on a map: across, plus up counted extra.
fn walk(from: (f64, f64), to: (f64, f64)) -> f64 {
    let across = (to.0 - from.0).abs();
    let rise = (from.1 - to.1).max(0.0);
    let drop = (to.1 - from.1).max(0.0);
    (across + rise * CLIMB_FACTOR + drop) / WALK_SPEED
}

/// Where a player is on a map: at a portal they came out of, or somewhere unknown.
type At = Option<usize>;

#[derive(Clone, Copy)]
enum Hop<'a> {
    /// Through the portal at this index of the map's spots.
    Spot(usize),
    /// Through a portal the data lists without a place.
    Unplaced(u32),
    Ride(&'a TravelLink),
}

struct Reached<'a> {
    seconds: f64,
    /// Seconds of it spent on guesses, where a map's portals have no places.
    guessed: bool,
    prev: Option<((u32, At), Hop<'a>)>,
}

#[derive(PartialEq)]
struct Queued {
    seconds: f64,
    map: u32,
    at: At,
}

impl Eq for Queued {}

impl Ord for Queued {
    // The quickest first; ties go to the lower map id so the search is the same each time.
    fn cmp(&self, other: &Self) -> Ordering {
        other.seconds.total_cmp(&self.seconds).then_with(|| other.map.cmp(&self.map)).then_with(|| other.at.cmp(&self.at))
    }
}

impl PartialOrd for Queued {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Atlas {
    /// The middle of a map's portals, for a player whose place on it isn't known.
    fn middle(&self, map: u32) -> Option<(f64, f64)> {
        let spots = self.spots.get(&map)?;
        let n = spots.len() as f64;
        Some((spots.iter().map(|s| f64::from(s.x)).sum::<f64>() / n, spots.iter().map(|s| f64::from(s.y)).sum::<f64>() / n))
    }

    fn place(&self, map: u32, at: At) -> Option<(f64, f64)> {
        match at {
            Some(i) => self.spots.get(&map)?.get(i).map(|s| (f64::from(s.x), f64::from(s.y))),
            None => self.middle(map),
        }
    }

    /// Where a portal from `from` comes out on `to`: the portal it names, or else one on `to`
    /// leading back to `from`.
    fn arrival(&self, from: u32, to: u32, to_portal: Option<&str>) -> At {
        let spots = self.spots.get(&to)?;
        to_portal
            .and_then(|name| spots.iter().position(|s| s.name == name))
            .or_else(|| spots.iter().position(|s| s.to == from))
    }

    /// The quickest route from one map to another through portals and `travel`, timed by the
    /// walk across each map. `None` when no way is known.
    pub(super) fn quickest(&self, from: u32, to: u32, travel: &Travel) -> Option<Route> {
        if !self.maps.contains_key(&from) || !self.maps.contains_key(&to) {
            return None;
        }
        let mut reached: HashMap<(u32, At), Reached> =
            HashMap::from([((from, None), Reached { seconds: 0.0, guessed: false, prev: None })]);
        let mut queue = BinaryHeap::from([Queued { seconds: 0.0, map: from, at: None }]);
        let mut end = None;
        while let Some(Queued { seconds, map, at }) = queue.pop() {
            if reached.get(&(map, at)).is_some_and(|r| r.seconds < seconds) {
                continue;
            }
            if map == to {
                end = Some((map, at));
                break;
            }
            let guessed = reached[&(map, at)].guessed;
            let here = self.place(map, at);
            let mut next: Vec<(u32, At, f64, bool, Hop)> = Vec::new();
            let spots = self.spots.get(&map).map(Vec::as_slice).unwrap_or_default();
            for (i, spot) in spots.iter().enumerate() {
                if spot.to == map || !self.maps.contains_key(&spot.to) {
                    continue;
                }
                let (cost, guess) = match here {
                    Some(place) => (walk(place, (f64::from(spot.x), f64::from(spot.y))), false),
                    None => (UNPLACED_SECONDS, true),
                };
                let arrive = self.arrival(map, spot.to, spot.to_portal.as_deref());
                next.push((spot.to, arrive, cost + PORTAL_SECONDS, guess, Hop::Spot(i)));
            }
            // Exits the data lists with no portal placed: a rough walk.
            for exit in self.exits.get(&map).into_iter().flatten().filter(|e| e.to.known) {
                if spots.iter().any(|s| s.to == exit.to.id) {
                    continue;
                }
                let arrive = self.arrival(map, exit.to.id, None);
                next.push((exit.to.id, arrive, UNPLACED_SECONDS + PORTAL_SECONDS, true, Hop::Unplaced(exit.to.id)));
            }
            // Leaving the island can't be undone, so a scroll is never suggested as a way off it.
            let stays =
                |l: &&TravelLink| l.from.is_some() || !self.on_starting_island(map) || self.on_starting_island(l.to);
            for link in travel.leaving(map).filter(|l| self.maps.contains_key(&l.to)).filter(stays) {
                next.push((link.to, None, ride_seconds(link.kind), false, Hop::Ride(link)));
            }
            for (to_map, to_at, cost, guess, hop) in next {
                let total = seconds + cost;
                let better = reached.get(&(to_map, to_at)).is_none_or(|r| total < r.seconds - 1e-9);
                if better {
                    reached.insert(
                        (to_map, to_at),
                        Reached { seconds: total, guessed: guessed || guess, prev: Some(((map, at), hop)) },
                    );
                    queue.push(Queued { seconds: total, map: to_map, at: to_at });
                }
            }
        }
        let end = end?;
        let mut steps = Vec::new();
        let mut node = end;
        while let Some((prev, hop)) = reached.get(&node).and_then(|r| r.prev) {
            steps.push(self.step(prev.0, node.0, hop));
            node = prev;
        }
        steps.reverse();
        let done = &reached[&end];
        let rides: f64 = steps.iter().filter_map(|s| s.travel.as_ref()).map(|t| ride_seconds(t.kind)).sum();
        let walking = if steps.iter().any(|s| s.travel.is_some()) { self.portals_on_foot(from, to) } else { None };
        Some(Route {
            from: self.map_ref(from),
            to: self.map_ref(to),
            steps,
            walking,
            walk_seconds: (!done.guessed).then_some(done.seconds - rides),
        })
    }

    fn step(&self, from: u32, to: u32, hop: Hop) -> RouteStep {
        match hop {
            Hop::Ride(link) => RouteStep {
                from: self.map_ref(from),
                to: self.map_ref(link.to),
                direction: Direction::Other,
                side: None,
                portal: None,
                hidden: false,
                travel: Some(TravelStep { kind: link.kind, service: link.service.clone(), fare: link.fare }),
            },
            Hop::Spot(i) => {
                let name = &self.spots[&from][i].name;
                let exits = self.exits.get(&from).into_iter().flatten();
                let exit = exits.clone().find(|e| e.to.id == to && e.portals.contains(name));
                let (direction, hidden) = Direction::of_portal(name);
                RouteStep {
                    from: self.map_ref(from),
                    to: self.map_ref(to),
                    direction: exit.map_or(direction, |e| e.direction),
                    side: exit.and_then(|e| e.side),
                    portal: Some(name.clone()),
                    hidden: exit.map_or(hidden, |e| e.hidden),
                    travel: None,
                }
            }
            Hop::Unplaced(dest) => {
                let exit = self.exits.get(&from).into_iter().flatten().find(|e| e.to.id == dest);
                RouteStep {
                    from: self.map_ref(from),
                    to: self.map_ref(dest),
                    direction: exit.map_or(Direction::Other, |e| e.direction),
                    side: exit.and_then(|e| e.side),
                    portal: exit.and_then(|e| e.portals.first().cloned()),
                    hidden: exit.is_some_and(|e| e.hidden),
                    travel: None,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mapdata::atlas::tests::map;
    use crate::mapdata::osms::{Dataset, MapPoint, Portal};

    fn portal(name: &str, dest: u32, to_portal: &str) -> Portal {
        Portal { name: name.into(), dest_map_id: Some(dest), dest_portal: Some(to_portal.into()), side: None, height: None }
    }

    fn point(name: &str, dest: u32, to_portal: &str, x: i32, y: i32) -> MapPoint {
        MapPoint { kind: PointKind::Portal, id: dest, name: Some(name.into()), to_portal: Some(to_portal.into()), x, y }
    }

    // A made-up world, none of it real game data. Start has a far portal straight to Goal, and two
    // near ones into Lane, whose portal to Goal stands close to where Lane is entered.
    //
    //   Start --east00 (far)--> Goal
    //     \--west00/west01--> Lane --east00--> Goal
    fn world() -> Atlas {
        let mut start = map(1, "Start", "Test Road", vec![portal("east00", 3, "west00"), portal("west00", 2, "east00"), portal("west01", 2, "east00")]);
        start.points = vec![point("east00", 3, "west00", 4000, 0), point("west00", 2, "east00", 0, 0), point("west01", 2, "east00", 200, 0)];
        let mut lane = map(2, "Lane", "Test Road", vec![portal("east00", 1, "west00"), portal("h_out00", 3, "west01")]);
        lane.points = vec![point("east00", 1, "west00", 100, 0), point("h_out00", 3, "west01", 300, -400)];
        let mut goal = map(3, "Goal", "Test Road", vec![portal("west00", 1, "east00"), portal("west01", 2, "h_out00")]);
        goal.points = vec![point("west00", 1, "east00", 0, 0), point("west01", 2, "h_out00", 50, 0)];
        Atlas::new(Dataset { maps: vec![start, lane, goal], ..Dataset::default() })
    }

    #[test]
    fn the_quickest_walk_beats_the_fewest_portals() {
        let atlas = world();
        let route = atlas.route(1, 3).unwrap();
        let steps: Vec<(&str, Option<&str>, bool)> =
            route.steps.iter().map(|s| (s.to.name.as_str(), s.portal.as_deref(), s.hidden)).collect();
        // Start's middle is x 1400: 1200 px to west01, against 2600 to east00 straight to Goal.
        assert_eq!(steps, [("Lane", Some("west01"), false), ("Goal", Some("h_out00"), true)]);
        // 1200 px, then 200 across and 400 up counted double from east00 to h_out00, and two portals.
        let expected = (1200.0 + 200.0 + 800.0) / WALK_SPEED + 2.0 * PORTAL_SECONDS;
        assert!((route.walk_seconds.unwrap() - expected).abs() < 1e-6, "{:?}", route.walk_seconds);
    }

    #[test]
    fn a_climb_counts_more_than_a_drop() {
        assert!(walk((0.0, 0.0), (0.0, -100.0)) > walk((0.0, -100.0), (0.0, 0.0)));
        assert_eq!(walk((0.0, 0.0), (250.0, 0.0)), 2.0);
    }

    #[test]
    fn maps_without_places_still_route_and_say_nothing_of_time() {
        let atlas = crate::mapdata::atlas::tests::world();
        let route = atlas.route(5, 4).unwrap();
        assert_eq!(route.steps.len(), 3);
        assert_eq!(route.walk_seconds, None, "the test world places no portals");
    }

    #[test]
    fn the_portals_leave_the_maps_other_points_alone() {
        let mut maps = BTreeMap::new();
        let mut m = map(1, "Start", "Test Road", vec![]);
        m.points = vec![point("east00", 3, "west00", 1, 2), MapPoint { kind: PointKind::Npc, id: 9, name: None, to_portal: None, x: 0, y: 0 }];
        maps.insert(1, m);
        let spots = take_spots(&mut maps);
        assert_eq!(spots[&1].len(), 1);
        assert_eq!(maps[&1].points.len(), 1, "the NPC stays");
    }
}
