//! Travel that is not a portal: taxis and return scrolls.
//!
//! The map data has none of it, so the connections come from a small file of facts players
//! recorded on mapleclassic.wiki (`travel.json`), each with its source. The player can edit a
//! copy in the app's data folder. Routes use them only when the player turns them on, and a
//! ride or scroll is one step. Like every route, this is information: nothing moves the
//! character.

use serde::{Deserialize, Serialize};

use super::atlas::Atlas;

/// The connections Leafline ships with.
pub const DEFAULT_FILE: &str = include_str!("travel.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TravelKind {
    /// Regular taxis and rides between towns.
    Cab,
    /// Pricier rides straight to a hunting ground.
    VipCab,
    /// Return scrolls, usable from any map.
    Scroll,
}

/// Which travel the player turned on, kept in the settings. Off by default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TravelChoice {
    pub enabled: bool,
    pub cabs: bool,
    pub vip_cabs: bool,
    pub scrolls: bool,
}

impl Default for TravelChoice {
    fn default() -> Self {
        Self { enabled: false, cabs: true, vip_cabs: false, scrolls: true }
    }
}

impl TravelChoice {
    pub fn allows(&self, kind: TravelKind) -> bool {
        self.enabled
            && match kind {
                TravelKind::Cab => self.cabs,
                TravelKind::VipCab => self.vip_cabs,
                TravelKind::Scroll => self.scrolls,
            }
    }
}

/// The travel file as written.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TravelFile {
    #[serde(default)]
    pub about: String,
    /// When the facts were last checked against the source.
    #[serde(default)]
    pub checked: Option<String>,
    pub services: Vec<Service>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Service {
    pub kind: TravelKind,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operator: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Where the facts come from.
    pub source: String,
    pub routes: Vec<ServiceRoute>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceRoute {
    /// The map the ride leaves from; none for a scroll, which works anywhere.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    pub to: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fare: Option<u32>,
}

impl TravelFile {
    pub fn parse(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|err| format!("travel.json: {err}"))
    }

    pub fn default_file() -> Self {
        Self::parse(DEFAULT_FILE).expect("the bundled travel.json is valid")
    }
}

/// One connection with its maps found in the data.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TravelLink {
    pub kind: TravelKind,
    pub service: String,
    /// `None` when usable from any map.
    pub from: Option<u32>,
    pub to: u32,
    pub fare: Option<u32>,
}

/// The connections a route may use.
#[derive(Debug, Clone, Default)]
pub struct Travel {
    links: Vec<TravelLink>,
}

impl Travel {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.links.is_empty()
    }

    /// The connections leaving `map`, scrolls included.
    pub(super) fn leaving(&self, map: u32) -> impl Iterator<Item = &TravelLink> {
        self.links.iter().filter(move |l| l.from.is_none_or(|from| from == map) && l.to != map)
    }
}

impl Atlas {
    /// The connections in `file` that `choice` turns on, with their maps found by name. Also
    /// returns the names no map in the data has, so they can be shown and fixed.
    pub fn travel(&self, file: &TravelFile, choice: TravelChoice) -> (Travel, Vec<String>) {
        let mut links = Vec::new();
        let mut missing = Vec::new();
        for service in &file.services {
            for route in &service.routes {
                let mut find = |name: &str| {
                    let found = self.map_named(name);
                    if found.is_none() && !missing.iter().any(|m| m == name) {
                        missing.push(name.to_string());
                    }
                    found
                };
                let from = route.from.as_deref().map(&mut find);
                let to = find(&route.to);
                if !choice.allows(service.kind) {
                    continue;
                }
                let from = match from {
                    None => None,
                    Some(Some(id)) => Some(id),
                    Some(None) => continue,
                };
                let Some(to) = to else { continue };
                links.push(TravelLink {
                    kind: service.kind,
                    service: service.name.clone(),
                    from,
                    to,
                    fare: route.fare,
                });
            }
        }
        (Travel { links }, missing)
    }

    /// The map with exactly this name, a town if several share it, else the lowest ID.
    fn map_named(&self, name: &str) -> Option<u32> {
        let name = name.trim();
        self.maps
            .values()
            .filter(|m| m.name.trim().eq_ignore_ascii_case(name))
            .min_by_key(|m| (!m.is_town, m.id))
            .map(|m| m.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mapdata::atlas::tests::world;

    fn file(json: &str) -> TravelFile {
        TravelFile::parse(json).unwrap()
    }

    #[test]
    fn the_bundled_file_parses_and_cites_its_sources() {
        let file = TravelFile::default_file();
        assert!(file.services.iter().all(|s| s.source.starts_with("https://mapleclassic.wiki/")));
        assert!(file.services.iter().any(|s| s.kind == TravelKind::Scroll && s.routes[0].from.is_none()));
    }

    #[test]
    fn names_resolve_and_choices_filter() {
        let atlas = world();
        let file = file(
            r#"{"services": [
                {"kind": "cab", "name": "Cab", "source": "x", "routes": [
                    {"from": "Mossbank Meadow II", "to": "mossbank", "fare": 100},
                    {"from": "Nowhere", "to": "Mossbank"}
                ]},
                {"kind": "scroll", "name": "Scroll to the Inn", "source": "x", "routes": [{"to": "Mossbank Inn"}]}
            ]}"#,
        );
        let off = atlas.travel(&file, TravelChoice::default());
        assert!(off.0.is_empty());
        assert_eq!(off.1, ["Nowhere"]);

        let on = TravelChoice { enabled: true, ..TravelChoice::default() };
        let (travel, missing) = atlas.travel(&file, on);
        assert_eq!(missing, ["Nowhere"]);
        // Two maps are called Mossbank: the town wins.
        let links: Vec<(Option<u32>, u32)> = travel.links.iter().map(|l| (l.from, l.to)).collect();
        assert_eq!(links, [(Some(3), 1), (None, 5)]);
        assert_eq!(travel.leaving(2).map(|l| l.to).collect::<Vec<_>>(), [5]);
        assert_eq!(travel.leaving(3).map(|l| l.to).collect::<Vec<_>>(), [1, 5]);

        let scrolls_off = TravelChoice { scrolls: false, ..on };
        assert_eq!(atlas.travel(&file, scrolls_off).0.links.len(), 1);
    }
}
