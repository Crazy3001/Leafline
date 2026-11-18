//! "Go here": the one place the player is heading for, shared by every window and kept between
//! runs, with the places headed for lately. The overlay follows the live map reading towards it
//! and clears it on arrival; this only holds it.

use std::collections::HashSet;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use super::files::{Files, read_json, write_json};

use crate::error::{Error, Result};
use crate::mapdata::MapRef;

pub const FILE_NAME: &str = "destination.json";

/// Emitted to every window when the destination or the recent places change.
pub const NAV_EVENT: &str = "destination";

/// How many places headed for lately are kept.
const RECENT: usize = 6;

/// The most maps one place may stand for: an NPC found on several, or maps sharing a name.
const MAX_MAPS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlaceKind {
    Map,
    Npc,
}

/// A place to head for: a map, or an NPC on any of the maps they stand on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Destination {
    pub kind: PlaceKind,
    /// The map's or the NPC's name, as the bar names it.
    pub name: String,
    /// Reaching any of these is arriving. The overlay heads for the nearest.
    pub maps: Vec<MapRef>,
}

impl Destination {
    /// The same place, whichever order its maps came in.
    fn same_place(&self, other: &Destination) -> bool {
        let ids = |d: &Destination| {
            let mut ids: Vec<u32> = d.maps.iter().map(|m| m.id).collect();
            ids.sort_unstable();
            ids
        };
        self.kind == other.kind && self.name.eq_ignore_ascii_case(&other.name) && ids(self) == ids(other)
    }

    /// Trimmed, with each map once and no more than [`MAX_MAPS`]; refused without a name or a map.
    fn checked(mut self) -> Result<Self> {
        self.name = self.name.trim().to_string();
        let mut seen = HashSet::new();
        self.maps.retain(|m| seen.insert(m.id));
        self.maps.truncate(MAX_MAPS);
        if self.name.is_empty() || self.maps.is_empty() {
            return Err(Error::Invalid("A destination needs a name and a map.".into()));
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Nav {
    pub destination: Option<Destination>,
    /// The newest first, the destination included.
    pub recent: Vec<Destination>,
}

impl Nav {
    /// Heads for a place, or stops heading anywhere. A place set again moves to the front of
    /// the recent ones instead of showing twice.
    fn go(&mut self, destination: Option<Destination>) -> Result<()> {
        let Some(destination) = destination.map(Destination::checked).transpose()? else {
            self.destination = None;
            return Ok(());
        };
        self.recent.retain(|d| !d.same_place(&destination));
        self.recent.insert(0, destination.clone());
        self.recent.truncate(RECENT);
        self.destination = Some(destination);
        Ok(())
    }

    /// Clears the destination on arrival, unless another window already set a new one.
    fn arrive(&mut self, at: &Destination) -> bool {
        if self.destination.as_ref().is_some_and(|d| d.same_place(at)) {
            self.destination = None;
            return true;
        }
        false
    }

    /// Takes a place off the recent list.
    fn forget(&mut self, place: &Destination) {
        self.recent.retain(|d| !d.same_place(place));
    }
}

pub struct NavState {
    current: Mutex<Nav>,
}

impl NavState {
    /// Loads what was saved. A missing or unreadable file means nowhere to go.
    pub fn load(files: &dyn Files) -> Self {
        Self { current: Mutex::new(read_json(files, FILE_NAME).unwrap_or_default()) }
    }

    /// Changes the state, then saves it, if `change` says it changed. Returns the state and
    /// whether it changed, for the caller to tell every window.
    fn update(&self, files: &dyn Files, change: impl FnOnce(&mut Nav) -> Result<bool>) -> Result<(Nav, bool)> {
        let mut nav = self.current.lock().expect("nav lock");
        let changed = change(&mut nav)?;
        if changed {
            write_json(files, FILE_NAME, &*nav)?;
        }
        Ok((nav.clone(), changed))
    }

    pub fn get(&self) -> Nav {
        self.current.lock().expect("nav lock").clone()
    }

    /// Heads for a place, or with none, cancels.
    pub fn set_destination(&self, files: &dyn Files, destination: Option<Destination>) -> Result<(Nav, bool)> {
        self.update(files, |nav| nav.go(destination).map(|()| true))
    }

    /// The overlay reached the place: clears it if it is still the one set.
    pub fn arrive_at(&self, files: &dyn Files, destination: Destination) -> Result<(Nav, bool)> {
        self.update(files, |nav| Ok(nav.arrive(&destination)))
    }

    pub fn forget_destination(&self, files: &dyn Files, destination: Destination) -> Result<(Nav, bool)> {
        self.update(files, |nav| {
            let before = nav.recent.len();
            nav.forget(&destination);
            Ok(nav.recent.len() != before)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(id: u32, name: &str) -> MapRef {
        MapRef { id, name: name.into(), street: None, known: true }
    }

    fn to_map(id: u32, name: &str) -> Destination {
        Destination { kind: PlaceKind::Map, name: name.into(), maps: vec![map(id, name)] }
    }

    fn to_npc(name: &str, maps: &[u32]) -> Destination {
        Destination { kind: PlaceKind::Npc, name: name.into(), maps: maps.iter().map(|&id| map(id, "Town")).collect() }
    }

    #[test]
    fn setting_a_place_heads_there_and_remembers_it_first() {
        let mut nav = Nav::default();
        nav.go(Some(to_map(1, "Henesys"))).unwrap();
        nav.go(Some(to_npc("Olaf", &[2]))).unwrap();
        assert_eq!(nav.destination, Some(to_npc("Olaf", &[2])));
        assert_eq!(nav.recent, vec![to_npc("Olaf", &[2]), to_map(1, "Henesys")]);
    }

    #[test]
    fn a_place_set_again_moves_to_the_front_instead_of_showing_twice() {
        let mut nav = Nav::default();
        nav.go(Some(to_map(1, "Henesys"))).unwrap();
        nav.go(Some(to_map(2, "Ellinia"))).unwrap();
        nav.go(Some(to_npc("Olaf", &[3, 4]))).unwrap();
        nav.go(Some(to_npc("olaf", &[4, 3]))).unwrap();
        nav.go(Some(to_map(1, "Henesys"))).unwrap();
        let names: Vec<&str> = nav.recent.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(names, ["Henesys", "olaf", "Ellinia"]);
    }

    #[test]
    fn a_map_and_an_npc_of_the_same_name_are_different_places() {
        let mut nav = Nav::default();
        nav.go(Some(to_map(1, "Olaf"))).unwrap();
        nav.go(Some(to_npc("Olaf", &[1]))).unwrap();
        assert_eq!(nav.recent.len(), 2);
    }

    #[test]
    fn only_the_newest_few_places_are_kept() {
        let mut nav = Nav::default();
        for id in 0..10 {
            nav.go(Some(to_map(id, &format!("Map {id}")))).unwrap();
        }
        assert_eq!(nav.recent.len(), RECENT);
        assert_eq!(nav.recent[0].name, "Map 9");
        assert_eq!(nav.recent[RECENT - 1].name, "Map 4");
    }

    #[test]
    fn cancelling_keeps_the_recent_places() {
        let mut nav = Nav::default();
        nav.go(Some(to_map(1, "Henesys"))).unwrap();
        nav.go(None).unwrap();
        assert_eq!(nav.destination, None);
        assert_eq!(nav.recent.len(), 1);
    }

    #[test]
    fn a_place_with_no_name_or_no_map_is_refused_and_changes_nothing() {
        let mut nav = Nav::default();
        nav.go(Some(to_map(1, "Henesys"))).unwrap();
        assert!(nav.go(Some(Destination { kind: PlaceKind::Npc, name: "Olaf".into(), maps: vec![] })).is_err());
        assert!(nav.go(Some(to_map(2, "   "))).is_err());
        assert_eq!(nav.destination, Some(to_map(1, "Henesys")));
        assert_eq!(nav.recent.len(), 1);
    }

    #[test]
    fn names_are_trimmed_and_maps_kept_once() {
        let mut nav = Nav::default();
        let mut olaf = to_npc("  Olaf ", &[3, 3, 4]);
        olaf.maps.extend((10..20).map(|id| map(id, "Far")));
        nav.go(Some(olaf)).unwrap();
        let set = nav.destination.unwrap();
        assert_eq!(set.name, "Olaf");
        assert_eq!(set.maps.len(), MAX_MAPS);
        assert_eq!(set.maps.iter().filter(|m| m.id == 3).count(), 1);
    }

    #[test]
    fn arriving_clears_only_the_place_arrived_at() {
        let mut nav = Nav::default();
        nav.go(Some(to_npc("Olaf", &[3]))).unwrap();
        // Another window set somewhere else before this arrival was reported.
        nav.go(Some(to_map(1, "Henesys"))).unwrap();
        assert!(!nav.arrive(&to_npc("Olaf", &[3])));
        assert_eq!(nav.destination, Some(to_map(1, "Henesys")));
        assert!(nav.arrive(&to_map(1, "Henesys")));
        assert_eq!(nav.destination, None);
    }

    #[test]
    fn forgetting_takes_a_place_off_the_recent_list() {
        let mut nav = Nav::default();
        nav.go(Some(to_map(1, "Henesys"))).unwrap();
        nav.go(Some(to_map(2, "Ellinia"))).unwrap();
        nav.forget(&to_map(1, "Henesys"));
        assert_eq!(nav.recent, vec![to_map(2, "Ellinia")]);
    }

    #[test]
    fn the_saved_file_reads_back_and_an_old_or_partial_one_still_loads() {
        let mut nav = Nav::default();
        nav.go(Some(to_npc("Olaf", &[3]))).unwrap();
        let json = serde_json::to_string(&nav).unwrap();
        assert_eq!(serde_json::from_str::<Nav>(&json).unwrap(), nav);
        assert_eq!(serde_json::from_str::<Nav>("{}").unwrap(), Nav::default());
        let camel: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(camel["destination"]["kind"], "npc");
    }
}
