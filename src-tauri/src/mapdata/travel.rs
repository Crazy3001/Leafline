//! Travel that is not a portal: taxis and return scrolls.
//!
//! The map data has none of it. The connections are what players recorded on
//! mapleclassic.wiki: the paid rides and fares its Route Planner lists, and the return scrolls
//! in its item table. Like the drops, they are fetched only when the player asks, cached in the
//! app's data folder and credited; none ship with the app. The player can edit a copy, which
//! then replaces them. Routes use them only when the player turns them on, and a ride or
//! scroll is one step. Like every route, this is information: nothing moves the character.

use serde::{Deserialize, Serialize};

use super::atlas::Atlas;
use super::fetch::Source;
use crate::error::{Error, Result};

/// mapleclassic.wiki's API, which travel still reads: the Route Planner page and the `items` table
/// are among what it answers for anonymous visitors (checked 2026-11-11).
pub const WIKI_API: &str = "https://mapleclassic.wiki/api.php";

/// Percent-encodes everything but unreserved characters.
fn encode(text: &str) -> String {
    text.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// The wiki's Route Planner settings, which list its fast-travel services and their fares. Asked
/// through the API: the wiki answers `index.php?action=raw` with 403 for programs (seen 2026-11-05).
pub const ROUTE_PLANNER_QUERY: &str = "https://mapleclassic.wiki/api.php?action=query&prop=revisions\
    &titles=Module:RoutePlanner/config.json&rvprop=content&rvslots=main&format=json&formatversion=2";
const ROUTE_PLANNER_PAGE: &str = "https://mapleclassic.wiki/wiki/Module:RoutePlanner/config.json";
const WIKI_PAGE: &str = "https://mapleclassic.wiki/wiki/";
/// Return scrolls are named after where they go.
const SCROLL_PREFIX: &str = "Return Scroll to ";

const ABOUT: &str = "Ways to travel that are not portals, which the map data does not have, as players recorded \
    them on mapleclassic.wiki (CC BY-NC-SA 4.0). Leafline uses them in routes only when you turn them on in \
    Settings. Each ride or scroll counts as one step. Map names must match the map data exactly. A route without \
    \"from\" can be used from any map. Fares are in mesos at level 20 and up. Edit freely: this copy replaces \
    what Leafline downloads, until you delete it.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TravelKind {
    /// Regular taxis and rides between towns.
    Cab,
    /// Pricier rides straight to a hunting ground.
    VipCab,
    /// Return scrolls, usable from any map.
    Scroll,
    /// A ride an NPC gives, such as the flight to Florina Beach. Always in routes: some maps
    /// can be reached no other way, so there is nothing to turn off.
    Npc,
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
        match kind {
            TravelKind::Npc => true,
            _ => {
                self.enabled
                    && match kind {
                        TravelKind::Cab => self.cabs,
                        TravelKind::VipCab => self.vip_cabs,
                        _ => self.scrolls,
                    }
            }
        }
    }
}

/// The travel file as written.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TravelFile {
    #[serde(default)]
    pub about: String,
    /// When the facts were downloaded, in Unix seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fetched_at: Option<i64>,
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
    pub fn parse(json: &str) -> std::result::Result<Self, String> {
        serde_json::from_str(json).map_err(|err| format!("travel.json: {err}"))
    }

    /// The rides NPCs give that are not portals and are in no download. Written by hand from what
    /// players report, so each says so, and the fare is left out until someone records it. Add
    /// to this as more are confirmed in game.
    fn known_rides() -> Vec<Service> {
        const SOURCE: &str = "Reported by a player, not from the game's data or a download; the fare was not recorded.";
        let ride = |name: &str, operator: &str, from: &str, to: &str| Service {
            kind: TravelKind::Npc,
            name: name.to_string(),
            operator: Some(operator.to_string()),
            note: Some("a flight by NPC, so the map data doesn't list it".to_string()),
            source: SOURCE.to_string(),
            routes: vec![ServiceRoute { from: Some(from.to_string()), to: to.to_string(), fare: None }],
        };
        vec![
            ride("Pason's flight to Florina Beach", "Pason", "Lith Harbor", "Florina Beach"),
            ride("Pison's flight to Lith Harbor", "Pison", "Florina Beach", "Lith Harbor"),
        ]
    }

    /// This file with the known NPC rides added, unless it already has a service of that name
    /// (the player may have edited one).
    pub fn with_known_rides(mut self) -> Self {
        for ride in Self::known_rides() {
            if !self.services.iter().any(|s| s.name == ride.name) {
                self.services.push(ride);
            }
        }
        self
    }

    /// A file with nothing in it yet, for the player to fill in.
    pub fn empty() -> Self {
        Self { about: ABOUT.to_string(), fetched_at: None, services: Vec::new() }
    }

    /// The paid rides from the wiki's Route Planner, leaving out the ferries and gates it marks
    /// as required (trips of another kind), and every item named "Return Scroll to <town>".
    pub fn fetch(source: &dyn Source, now: i64) -> Result<Self> {
        let config = planner_config(&source.get(ROUTE_PLANNER_QUERY)?)
            .map_err(|err| Error::Download(format!("the wiki's Route Planner settings: {err}")))?;
        let reply: ScrollReply = serde_json::from_slice(&source.get(&scroll_query_url())?)
            .map_err(|err| Error::Download(format!("the wiki's return scrolls: {err}")))?;
        let mut services = rides(&config);
        services.extend(reply.cargoquery.into_iter().filter_map(|row| scroll(&row.title.name?)));
        Ok(Self { about: ABOUT.to_string(), fetched_at: Some(now), services })
    }
}

/// The page's text taken out of the API's reply and read as the Route Planner settings.
fn planner_config(reply: &[u8]) -> std::result::Result<PlannerConfig, String> {
    let reply: serde_json::Value = serde_json::from_slice(reply).map_err(|err| err.to_string())?;
    let content = reply
        .pointer("/query/pages/0/revisions/0/slots/main/content")
        .and_then(|content| content.as_str())
        .ok_or("the page wasn't in the reply")?;
    serde_json::from_str(content).map_err(|err| err.to_string())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlannerConfig {
    #[serde(default)]
    fast_travel: Vec<PlannerService>,
    #[serde(default)]
    fares: Vec<PlannerFare>,
}

#[derive(Deserialize)]
struct PlannerService {
    name: String,
    #[serde(default)]
    note: Option<String>,
    /// Towns served both ways between each other.
    #[serde(default)]
    maps: Vec<String>,
    /// One-way rides, `[from, to]`.
    #[serde(default)]
    routes: Vec<(String, String)>,
    #[serde(default)]
    required: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlannerFare {
    service: String,
    from: String,
    to: String,
    #[serde(default)]
    fare_from_level20: Option<u32>,
}

#[derive(Deserialize)]
struct ScrollReply {
    #[serde(default)]
    cargoquery: Vec<ScrollRow>,
}

#[derive(Deserialize)]
struct ScrollRow {
    title: ScrollTitle,
}

#[derive(Deserialize)]
struct ScrollTitle {
    name: Option<String>,
}

fn rides(config: &PlannerConfig) -> Vec<Service> {
    config
        .fast_travel
        .iter()
        .filter(|s| !s.required)
        .map(|service| {
            let mut pairs: Vec<(&str, &str)> = Vec::new();
            for from in &service.maps {
                for to in service.maps.iter().filter(|to| *to != from) {
                    pairs.push((from, to));
                }
            }
            for (from, to) in &service.routes {
                if !pairs.contains(&(from.as_str(), to.as_str())) {
                    pairs.push((from, to));
                }
            }
            let fare = |from: &str, to: &str| {
                config
                    .fares
                    .iter()
                    .find(|f| f.service == service.name && f.from == from && f.to == to)
                    .and_then(|f| f.fare_from_level20)
            };
            Service {
                kind: if service.name.contains("VIP") { TravelKind::VipCab } else { TravelKind::Cab },
                name: service.name.clone(),
                operator: None,
                note: service.note.as_deref().map(sentence),
                source: ROUTE_PLANNER_PAGE.to_string(),
                routes: pairs
                    .into_iter()
                    .map(|(from, to)| ServiceRoute {
                        from: Some(from.to_string()),
                        to: to.to_string(),
                        fare: fare(from, to),
                    })
                    .collect(),
            }
        })
        .collect()
}

fn scroll(name: &str) -> Option<Service> {
    let to = name.strip_prefix(SCROLL_PREFIX)?.trim();
    Some(Service {
        kind: TravelKind::Scroll,
        name: name.to_string(),
        operator: None,
        note: None,
        source: format!("{WIKI_PAGE}{}", name.replace(' ', "_")),
        routes: vec![ServiceRoute { from: None, to: to.to_string(), fare: None }],
    })
}

/// The wiki's notes start in lower case and end without a full stop.
fn sentence(note: &str) -> String {
    let mut chars = note.trim().chars();
    let first = chars.next().map(|c| c.to_uppercase().collect::<String>()).unwrap_or_default();
    let rest: String = chars.collect();
    let text = format!("{first}{rest}");
    if text.ends_with('.') { text } else { format!("{text}.") }
}

fn scroll_query_url() -> String {
    let query = [
        ("action", "cargoquery"),
        ("format", "json"),
        ("tables", "items"),
        ("fields", "name_en=name"),
        ("where", "name_en LIKE 'Return Scroll to %'"),
        ("limit", "100"),
    ];
    let encoded: Vec<String> = query.iter().map(|(k, v)| format!("{k}={}", encode(v))).collect();
    format!("{WIKI_API}?{}", encoded.join("&"))
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

    #[test]
    fn an_npc_ride_is_in_routes_even_with_travel_turned_off() {
        let off = TravelChoice::default();
        assert!(!off.allows(TravelKind::Cab) && !off.allows(TravelKind::Scroll));
        assert!(off.allows(TravelKind::Npc));

        let file = TravelFile::empty().with_known_rides();
        let names: Vec<_> = file.services.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["Pason's flight to Florina Beach", "Pison's flight to Lith Harbor"]);
        // Adding them again changes nothing, and an edited copy keeps the player's version.
        assert_eq!(file.clone().with_known_rides(), file);
        let mut edited = file.clone();
        edited.services[0].routes[0].fare = Some(1500);
        assert_eq!(edited.clone().with_known_rides().services[0].routes[0].fare, Some(1500));
    }
    use crate::mapdata::atlas::tests::{island_world, world};

    fn file(json: &str) -> TravelFile {
        TravelFile::parse(json).unwrap()
    }

    #[test]
    fn scrolls_are_not_a_way_off_the_starting_island() {
        let atlas = island_world();
        let scroll =
            |to| TravelLink { kind: TravelKind::Scroll, service: "Return Scroll".into(), from: None, to, fare: None };
        let travel = Travel { links: vec![scroll(30), scroll(20)] };
        assert!(atlas.route_with(21, 30, &travel).is_none(), "the scroll to the mainland is not offered");
        assert!(atlas.route_with(21, 20, &travel).is_some(), "the scroll home is");
        assert!(atlas.route_with(31, 30, &travel).is_some(), "and scrolls still work on the mainland");
    }

    #[test]
    fn fetches_rides_fares_and_scrolls_from_the_wiki() {
        use crate::mapdata::fetch::tests::FakeSource;
        // Hand-made replies in the wiki's shapes; none of this is real game data.
        let config = r#"{"fastTravel": [
            {"name": "Ox Cart", "note": "slow ride between the villages", "maps": ["Mossbank", "Fernwick"],
             "routes": [["Mossbank", "Old Mill"]]},
            {"name": "VIP Balloon", "routes": [["Fernwick", "Burrow Den"]]},
            {"name": "Ferry", "required": true, "routes": [["Pier", "Isle"]]}],
            "fares": [{"service": "Ox Cart", "from": "Mossbank", "to": "Fernwick", "fare": 50, "fareFromLevel20": 100}]}"#;
        let scrolls = r#"{"cargoquery": [{"title": {"name": "Return Scroll to Mossbank"}},
            {"title": {"name": "Return Scroll - Nearest Village"}}]}"#;
        let mut source = FakeSource::default();
        let page = serde_json::json!({"query": {"pages": [{"revisions": [{"slots": {"main": {"content": config}}}]}]}});
        source.files.insert(ROUTE_PLANNER_QUERY.into(), page.to_string().into_bytes());
        source.files.insert(scroll_query_url(), scrolls.as_bytes().to_vec());

        let file = TravelFile::fetch(&source, 1234).unwrap();
        assert_eq!(file.fetched_at, Some(1234));
        let names: Vec<(&str, TravelKind)> = file.services.iter().map(|s| (s.name.as_str(), s.kind)).collect();
        assert_eq!(
            names,
            [
                ("Ox Cart", TravelKind::Cab),
                ("VIP Balloon", TravelKind::VipCab),
                ("Return Scroll to Mossbank", TravelKind::Scroll)
            ]
        );
        let cart: Vec<(Option<&str>, &str, Option<u32>)> =
            file.services[0].routes.iter().map(|r| (r.from.as_deref(), r.to.as_str(), r.fare)).collect();
        assert_eq!(
            cart,
            [
                (Some("Mossbank"), "Fernwick", Some(100)),
                (Some("Fernwick"), "Mossbank", None),
                (Some("Mossbank"), "Old Mill", None)
            ]
        );
        assert_eq!(file.services[0].note.as_deref(), Some("Slow ride between the villages."));
        let scroll = &file.services[2];
        assert_eq!((scroll.routes[0].from.as_deref(), scroll.routes[0].to.as_str()), (None, "Mossbank"));
        assert_eq!(scroll.source, "https://mapleclassic.wiki/wiki/Return_Scroll_to_Mossbank");
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
