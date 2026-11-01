//! The cached data indexed for the map card: finding a map from OCR'd text, gathering what is
//! known about it, and text routes between maps.

use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, HashMap, VecDeque};

use serde::Serialize;
use strsim::normalized_levenshtein;

use super::osms::{Dataset, Height, MapRecord, MonsterRecord, NO_MAP, QuestRecord, Side};
use super::travel::{Travel, TravelKind, TravelLink};
use super::turnins;

/// Below this similarity an OCR'd name is not taken for any map.
const MIN_SCORE: f64 = 0.72;
/// Other maps scoring this close to the best are offered as alternatives.
const NEAR: f64 = 0.08;
/// At or above this similarity, with no alternatives, an OCR'd name is taken for the map.
const SETTLED_SCORE: f64 = 0.85;

/// Which way a portal leads, judged from its name in the map data (`west00`, `in01`, ...).
/// Names follow the client's conventions, so this is a good guess rather than a certainty.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    West,
    East,
    /// Into a building or sub-area, or up.
    In,
    /// Back out, or down.
    Out,
    Other,
}

impl Direction {
    /// The direction a portal name suggests, and whether the name marks it as hidden.
    pub fn of_portal(name: &str) -> (Self, bool) {
        let name = name.to_ascii_lowercase();
        let hidden = name.starts_with("h_") || name.starts_with("hide");
        let base = name.trim_start_matches("h_");
        let direction = if base.starts_with("west") {
            Self::West
        } else if base.starts_with("east") {
            Self::East
        } else if ["jobin", "in", "up", "top"].iter().any(|p| base.starts_with(p)) {
            Self::In
        } else if ["jobout", "out", "down", "dn", "under"].iter().any(|p| base.starts_with(p)) {
            Self::Out
        } else {
            Self::Other
        };
        (direction, hidden)
    }
}

/// A map named in a card or route.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapRef {
    pub id: u32,
    pub name: String,
    pub street: Option<String>,
    /// False for IDs the data refers to but does not contain (cut or server-only maps).
    pub known: bool,
}

/// Where new characters start. An NPC takes them off it for good, so no portal leads out and
/// none leads back.
pub(super) const STARTING_REGION: &str = "Maple Island";

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapMatch {
    /// How closely the OCR'd name matched, 0 to 1.
    pub score: f64,
    pub card: MapCard,
    /// Other maps that matched nearly as well, such as maps sharing the name.
    pub alternatives: Vec<MapRef>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapCard {
    pub map: MapRef,
    pub region: Option<String>,
    pub is_town: bool,
    pub return_map: Option<MapRef>,
    /// Lowest and highest monster level here.
    pub level_range: Option<(u32, u32)>,
    pub mobs: Vec<MobRow>,
    /// Monsters on the map at once when it is full.
    pub mob_total: u32,
    /// Base EXP for killing every monster once, before any rate or party modifiers.
    pub exp_per_clear: u64,
    pub npcs: Vec<NpcRow>,
    /// Quests that ask for monsters found here.
    pub quests: Vec<QuestNeed>,
    pub exits: Vec<Exit>,
    pub links: Vec<Link>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobRow {
    pub id: u32,
    pub name: String,
    pub level: u32,
    pub hp: u32,
    pub exp: u32,
    pub count: u32,
    pub boss: bool,
    pub undead: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcRow {
    pub id: u32,
    pub name: String,
    /// Quests this NPC gives.
    pub quests: Vec<QuestRef>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestRef {
    pub id: u32,
    pub name: String,
    pub level_min: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestNeed {
    pub quest: QuestRef,
    pub giver: Option<String>,
    /// The monsters from this map the quest wants, and how many.
    pub targets: Vec<QuestTarget>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestTarget {
    pub mob_id: u32,
    pub name: String,
    pub count: u32,
}

/// One way out of a map: the destination and the portals that lead there.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Exit {
    pub direction: Direction,
    /// Roughly where the portal stands, when the data can tell. This outranks `direction`,
    /// which only guesses from the name: an `out00` can stand at either end of a map.
    pub side: Option<Side>,
    pub height: Option<Height>,
    pub to: MapRef,
    /// Portal names, empty when the data lists the exit without a portal.
    pub portals: Vec<String>,
    pub hidden: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Link {
    pub label: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Route {
    pub from: MapRef,
    pub to: MapRef,
    pub steps: Vec<RouteStep>,
    /// For a route that takes a ride or scroll: the portals the walk alone takes, or `None`
    /// when no portals lead there.
    pub walking: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteStep {
    pub from: MapRef,
    pub to: MapRef,
    pub direction: Direction,
    /// Roughly where the portal stands, when the data can tell.
    pub side: Option<Side>,
    pub portal: Option<String>,
    pub hidden: bool,
    /// Set when the step is a ride or scroll rather than a portal.
    pub travel: Option<TravelStep>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TravelStep {
    pub kind: TravelKind,
    pub service: String,
    pub fare: Option<u32>,
}

/// A trip that rides or scrolls part of the way, shorter than walking.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Shortcut {
    /// Steps, counting each ride or scroll as one.
    pub hops: u32,
    /// Where the first ride or scroll goes.
    pub via: MapRef,
}

/// A way from one map to the next: a portal, or travel the player turned on.
#[derive(Clone, Copy)]
enum Edge<'a> {
    Portal(&'a Exit),
    Travel(&'a TravelLink),
}

/// A map reached in a breadth-first search: in how many steps, from where, and where the
/// first ride or scroll on the way went.
struct Visit<'a> {
    hops: u32,
    prev: Option<(u32, Edge<'a>)>,
    via: Option<u32>,
}

/// The cached data, indexed for lookups.
pub struct Atlas {
    pub(super) maps: BTreeMap<u32, MapRecord>,
    pub(super) monsters: HashMap<u32, MonsterRecord>,
    pub(super) npc_names: HashMap<u32, String>,
    pub(super) quests: BTreeMap<u32, QuestRecord>,
    /// Map → (monster, count), from the monsters' spawn lists.
    pub(super) spawns: HashMap<u32, Vec<(u32, u32)>>,
    /// Map → NPCs standing there.
    pub(super) npcs_on: HashMap<u32, Vec<u32>>,
    /// Monster → quests that want it.
    pub(super) quests_for_mob: HashMap<u32, Vec<u32>>,
    /// NPC → quests they give.
    pub(super) quests_from: HashMap<u32, Vec<u32>>,
    /// Item → quests that can give it, the lowest level first.
    pub(super) quests_rewarding: HashMap<u32, Vec<u32>>,
    /// Quest → the NPC who takes it in, where the wiki says.
    pub(super) turn_ins: HashMap<u32, u32>,
    /// Map → its exits, in display order.
    pub(super) exits: HashMap<u32, Vec<Exit>>,
    /// Each map's name, street and town folded for fuzzy matching.
    pub(super) keys: Vec<(u32, String, Option<String>)>,
}

impl Atlas {
    pub fn new(data: Dataset) -> Self {
        // A few names in the data carry stray spaces.
        let maps: BTreeMap<u32, MapRecord> =
            data.maps.into_iter().map(|m| (m.id, MapRecord { name: m.name.trim().to_string(), ..m })).collect();
        let mut spawns: HashMap<u32, Vec<(u32, u32)>> = HashMap::new();
        for mob in &data.monsters {
            for spawn in mob.spawns.iter().filter(|s| s.count > 0) {
                spawns.entry(spawn.map_id).or_default().push((mob.id, spawn.count));
            }
        }
        let mut npcs_on: HashMap<u32, Vec<u32>> = HashMap::new();
        for npc in &data.npcs {
            for map in &npc.maps {
                npcs_on.entry(*map).or_default().push(npc.id);
            }
        }
        let mut quests_for_mob: HashMap<u32, Vec<u32>> = HashMap::new();
        let mut quests_from: HashMap<u32, Vec<u32>> = HashMap::new();
        let mut quests_rewarding: HashMap<u32, Vec<u32>> = HashMap::new();
        let mut by_level: Vec<&super::osms::QuestRecord> = data.quests.iter().collect();
        by_level.sort_by_key(|q| (q.level_min.unwrap_or(0), q.id));
        for quest in by_level {
            for reward in &quest.reward_items {
                quests_rewarding.entry(reward.item_id).or_default().push(quest.id);
            }
        }
        for quest in &data.quests {
            for need in &quest.requirements {
                quests_for_mob.entry(need.mob_id).or_default().push(quest.id);
            }
            if let Some(npc) = quest.npc_id {
                quests_from.entry(npc).or_default().push(quest.id);
            }
        }
        let keys = maps.values().map(|m| (m.id, fold(&m.name), m.street_name.as_deref().map(fold))).collect();
        let mut atlas = Self {
            monsters: data.monsters.into_iter().map(|m| (m.id, m)).collect(),
            npc_names: data.npcs.into_iter().map(|n| (n.id, n.name)).collect(),
            quests: data.quests.into_iter().map(|q| (q.id, q)).collect(),
            maps,
            spawns,
            npcs_on,
            quests_for_mob,
            quests_from,
            quests_rewarding,
            turn_ins: HashMap::new(),
            exits: HashMap::new(),
            keys,
        };
        atlas.exits = atlas.maps.values().map(|m| (m.id, atlas.exits_of(m))).collect();
        atlas.turn_ins = turnins::resolve(&data.turn_ins, |quest, name| atlas.turn_in_npc(quest, name));
        atlas
    }

    pub fn map_count(&self) -> usize {
        self.maps.len()
    }

    /// The map an OCR'd minimap header names. The street, when read, settles maps that share
    /// a name.
    pub fn find(&self, name: &str, street: Option<&str>) -> Option<MapMatch> {
        let query = fold(name);
        if query.is_empty() {
            return None;
        }
        let street = street.map(fold).filter(|s| !s.is_empty());
        let mut scored: Vec<(f64, f64, u32)> = self
            .keys
            .iter()
            .map(|(id, key, map_street)| {
                let score = similarity(&query, key);
                let bonus = match (&street, map_street) {
                    (Some(a), Some(b)) => 0.1 * similarity(a, b),
                    _ => 0.0,
                };
                (score, score + bonus, *id)
            })
            .filter(|(score, ..)| *score >= MIN_SCORE)
            .collect();
        scored.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.2.cmp(&b.2)));
        let &(score, _, best) = scored.first()?;
        let alternatives =
            scored[1..].iter().filter(|(s, ..)| *s >= score - NEAR).take(4).map(|(.., id)| self.map_ref(*id)).collect();
        Some(MapMatch { score, card: self.card(best)?, alternatives })
    }

    /// The map an OCR'd header can only mean, to replace the reading with its real name. OCR
    /// can spell one header differently from one moment to the next, which would split a
    /// session's per-map numbers. Near matches are left as read for the player to settle; maps
    /// sharing the name and street exactly read the same either way.
    pub fn settled(&self, name: &str, street: Option<&str>) -> Option<MapRef> {
        self.find(name, street)
            .filter(|found| found.score >= SETTLED_SCORE && self.twins(found).len() > found.alternatives.len())
            .map(|found| found.card.map)
    }

    /// The best match and the maps sharing its name and street exactly. A header can't tell
    /// these apart, such as the two Mushroom Towns on Maple Road.
    fn twins(&self, found: &MapMatch) -> Vec<u32> {
        let place = &found.card.map;
        let same = found.alternatives.iter().filter(|m| m.name == place.name && m.street == place.street);
        std::iter::once(place.id).chain(same.map(|m| m.id)).collect()
    }

    /// Whether another map shares this header exactly.
    pub fn has_twin(&self, name: &str, street: Option<&str>) -> bool {
        self.find(name, street).is_some_and(|found| self.twins(&found).len() > 1)
    }

    /// Which of the maps sharing a header the player is on, given the text `seen` on screen: an
    /// NPC only one of them has settles it, and `held`, the last answer, stands until another
    /// does. `None` for a header that only one map has, or until an NPC shows.
    pub fn twin(&self, name: &str, street: Option<&str>, seen: &[&str], held: Option<u32>) -> Option<u32> {
        let twins = self.twins(&self.find(name, street)?);
        if twins.len() < 2 {
            return None;
        }
        let npcs = |id: &u32| self.npcs_on.get(id).into_iter().flatten().filter_map(|npc| self.npc_names.get(npc));
        let only_on =
            |id: &u32, npc: &String| twins.iter().filter(|&other| other != id).all(|o| !npcs(o).any(|n| n == npc));
        let shown = |npc: &String| seen.iter().any(|text| similarity(&fold(text), &fold(npc)) >= SETTLED_SCORE);
        let on_screen = twins.iter().copied().find(|id| npcs(id).any(|npc| only_on(id, npc) && shown(npc)));
        on_screen.or(held.filter(|id| twins.contains(id)))
    }

    /// [`Self::find`], with the answer of [`Self::twin`] if there is one.
    pub fn find_twin(&self, name: &str, street: Option<&str>, twin: Option<u32>) -> Option<MapMatch> {
        let mut found = self.find(name, street)?;
        if let Some(id) = twin.filter(|id| self.twins(&found).contains(id)) {
            let place = found.card.map.clone();
            found.alternatives.retain(|m| m.name != place.name || m.street != place.street);
            found.card = self.card(id)?;
        }
        Some(found)
    }

    pub fn is_town(&self, id: u32) -> bool {
        self.maps.get(&id).is_some_and(|map| map.is_town)
    }

    /// Maps whose name or street contains `text`, then close fuzzy matches, for a picker.
    pub fn search(&self, text: &str, limit: usize) -> Vec<MapRef> {
        let query = fold(text);
        if query.is_empty() {
            return Vec::new();
        }
        let mut scored: Vec<(f64, u32)> = self
            .keys
            .iter()
            .filter_map(|(id, key, street)| {
                let score = if key.starts_with(&query) {
                    3.0
                } else if key.contains(&query) {
                    2.0
                } else if street.as_ref().is_some_and(|s| s.contains(&query)) {
                    1.0
                } else {
                    let fuzzy = similarity(&query, key);
                    if fuzzy < MIN_SCORE {
                        return None;
                    }
                    fuzzy
                };
                Some((score, *id))
            })
            .collect();
        scored.sort_by(|a, b| b.0.total_cmp(&a.0).then_with(|| self.maps[&a.1].name.cmp(&self.maps[&b.1].name)));
        scored.into_iter().take(limit).map(|(_, id)| self.map_ref(id)).collect()
    }

    pub fn card(&self, id: u32) -> Option<MapCard> {
        let map = self.maps.get(&id)?;
        let mut mobs: Vec<MobRow> = self
            .spawns
            .get(&id)
            .into_iter()
            .flatten()
            .filter_map(|&(mob_id, count)| {
                let m = self.monsters.get(&mob_id)?;
                Some(MobRow {
                    id: m.id,
                    name: m.name.clone(),
                    level: m.level,
                    hp: m.hp,
                    exp: m.exp,
                    count,
                    boss: m.is_boss,
                    undead: m.undead,
                })
            })
            .collect();
        mobs.sort_by(|a, b| (a.level, &a.name, a.id).cmp(&(b.level, &b.name, b.id)));
        let levels = mobs.iter().map(|m| m.level);
        let level_range = levels.clone().min().zip(levels.max());

        let mut npcs: Vec<NpcRow> = self
            .npcs_on
            .get(&id)
            .into_iter()
            .flatten()
            .filter_map(|npc| {
                Some(NpcRow {
                    id: *npc,
                    name: self.npc_names.get(npc)?.clone(),
                    quests: self
                        .quests_from
                        .get(npc)
                        .into_iter()
                        .flatten()
                        .filter_map(|q| self.quest_ref(*q))
                        .collect(),
                })
            })
            .collect();
        npcs.sort_by(|a, b| a.name.cmp(&b.name));

        Some(MapCard {
            map: self.map_ref(id),
            region: map.region.clone(),
            is_town: map.is_town,
            return_map: map.return_map_id.filter(|r| *r != id && *r != NO_MAP).map(|r| self.map_ref(r)),
            level_range,
            mob_total: mobs.iter().map(|m| m.count).sum(),
            exp_per_clear: mobs.iter().map(|m| m.exp as u64 * m.count as u64).sum(),
            quests: self.quests_needing(&mobs),
            mobs,
            npcs,
            exits: self.exits.get(&id).cloned().unwrap_or_default(),
            links: links(map),
        })
    }

    /// The fewest portals from one map to another, by breadth-first search over the portals'
    /// destinations. `None` when no path is known.
    pub fn route(&self, from: u32, to: u32) -> Option<Route> {
        self.route_with(from, to, &Travel::none())
    }

    /// The fewest steps from one map to another, where a ride or scroll in `travel` is one
    /// step like a portal. Between routes of the same length, walking wins. A route that
    /// takes travel also says how many portals the walk alone takes.
    pub fn route_with(&self, from: u32, to: u32, travel: &Travel) -> Option<Route> {
        if !self.maps.contains_key(&from) || !self.maps.contains_key(&to) {
            return None;
        }
        let visits = self.explore(from, travel, Some(to));
        visits.get(&to)?;
        let mut steps = Vec::new();
        let mut at = to;
        while let Some((prev, edge)) = visits.get(&at).and_then(|v| v.prev) {
            steps.push(match edge {
                Edge::Portal(exit) => RouteStep {
                    from: self.map_ref(prev),
                    to: exit.to.clone(),
                    direction: exit.direction,
                    side: exit.side,
                    portal: exit.portals.first().cloned(),
                    hidden: exit.hidden,
                    travel: None,
                },
                Edge::Travel(link) => RouteStep {
                    from: self.map_ref(prev),
                    to: self.map_ref(link.to),
                    direction: Direction::Other,
                    side: None,
                    portal: None,
                    hidden: false,
                    travel: Some(TravelStep { kind: link.kind, service: link.service.clone(), fare: link.fare }),
                },
            });
            at = prev;
        }
        steps.reverse();
        let walking = if steps.iter().any(|s| s.travel.is_some()) {
            self.explore(from, &Travel::none(), Some(to)).get(&to).map(|v| v.hops)
        } else {
            None
        };
        Some(Route { from: self.map_ref(from), to: self.map_ref(to), steps, walking })
    }

    /// Portals from `from` to every map reachable through them.
    pub(super) fn on_starting_island(&self, id: u32) -> bool {
        self.maps.get(&id).is_some_and(|m| m.region.as_deref() == Some(STARTING_REGION))
    }

    pub(super) fn hops_from(&self, from: u32) -> HashMap<u32, u32> {
        self.explore(from, &Travel::none(), None).into_iter().map(|(id, v)| (id, v.hops)).collect()
    }

    /// The maps `travel` brings closer than walking does from `from`, with the steps it takes.
    pub(super) fn shortcuts_from(
        &self,
        from: u32,
        travel: &Travel,
        walking: &HashMap<u32, u32>,
    ) -> HashMap<u32, Shortcut> {
        if travel.is_empty() {
            return HashMap::new();
        }
        self.explore(from, travel, None)
            .into_iter()
            .filter_map(|(id, visit)| {
                let via = visit.via?;
                let shorter = walking.get(&id).is_none_or(|&walk| visit.hops < walk);
                shorter.then(|| (id, Shortcut { hops: visit.hops, via: self.map_ref(via) }))
            })
            .collect()
    }

    /// Breadth-first search from `from` over portals and travel, stopping once `stop` is
    /// reached. Portals are tried first, so a walk wins over a ride of the same length.
    fn explore<'a>(&'a self, from: u32, travel: &'a Travel, stop: Option<u32>) -> HashMap<u32, Visit<'a>> {
        let mut visits = HashMap::from([(from, Visit { hops: 0, prev: None, via: None })]);
        let mut queue = VecDeque::from([from]);
        while let Some(at) = queue.pop_front() {
            if stop == Some(at) {
                break;
            }
            let (hops, via) = (visits[&at].hops + 1, visits[&at].via);
            let portals =
                self.exits.get(&at).into_iter().flatten().filter(|e| e.to.known).map(|e| (e.to.id, Edge::Portal(e)));
            // Leaving the island can't be undone, so a scroll is never suggested as a shortcut off it.
            let stays =
                |l: &&TravelLink| l.from.is_some() || !self.on_starting_island(at) || self.on_starting_island(l.to);
            let rides = travel
                .leaving(at)
                .filter(|l| self.maps.contains_key(&l.to))
                .filter(stays)
                .map(|l| (l.to, Edge::Travel(l)));
            for (next, edge) in portals.chain(rides) {
                if let Entry::Vacant(entry) = visits.entry(next) {
                    let via = via.or(matches!(edge, Edge::Travel(_)).then_some(next));
                    entry.insert(Visit { hops, prev: Some((at, edge)), via });
                    queue.push_back(next);
                }
            }
        }
        visits
    }

    pub(super) fn map_ref(&self, id: u32) -> MapRef {
        match self.maps.get(&id) {
            Some(m) => MapRef { id, name: m.name.clone(), street: m.street_name.clone(), known: true },
            None => MapRef { id, name: "Unknown map".into(), street: None, known: false },
        }
    }

    pub(super) fn quest_ref(&self, id: u32) -> Option<QuestRef> {
        let q = self.quests.get(&id)?;
        Some(QuestRef { id, name: q.name.clone(), level_min: q.level_min })
    }

    fn quests_needing(&self, mobs: &[MobRow]) -> Vec<QuestNeed> {
        let mut needs: BTreeMap<u32, QuestNeed> = BTreeMap::new();
        for mob in mobs {
            for quest_id in self.quests_for_mob.get(&mob.id).into_iter().flatten() {
                let Some(quest) = self.quests.get(quest_id) else { continue };
                let need = needs.entry(*quest_id).or_insert_with(|| QuestNeed {
                    quest: QuestRef { id: quest.id, name: quest.name.clone(), level_min: quest.level_min },
                    giver: quest.npc_id.and_then(|n| self.npc_names.get(&n).cloned()),
                    targets: Vec::new(),
                });
                if need.targets.iter().any(|t| t.mob_id == mob.id) {
                    continue;
                }
                let count = quest.requirements.iter().filter(|r| r.mob_id == mob.id).map(|r| r.count).sum();
                need.targets.push(QuestTarget { mob_id: mob.id, name: mob.name.clone(), count });
            }
        }
        let mut needs: Vec<QuestNeed> = needs.into_values().collect();
        needs.sort_by(|a, b| (a.quest.level_min, &a.quest.name).cmp(&(b.quest.level_min, &b.quest.name)));
        needs
    }

    /// Exits grouped by destination and where they stand, from the portals first and then any
    /// exit listed without a portal. Two portals to one map stay apart if they are far apart.
    fn exits_of(&self, map: &MapRecord) -> Vec<Exit> {
        let leads_out = |dest: u32| dest != map.id && dest != NO_MAP;
        let mut exits: Vec<Exit> = Vec::new();
        for portal in &map.portals {
            let Some(dest) = portal.dest_map_id.filter(|d| leads_out(*d)) else { continue };
            let (direction, hidden) = Direction::of_portal(&portal.name);
            let (side, height) = (portal.side, portal.height);
            let same =
                |e: &&mut Exit| e.to.id == dest && e.direction == direction && e.side == side && e.height == height;
            match exits.iter_mut().find(same) {
                Some(exit) => {
                    exit.portals.push(portal.name.clone());
                    exit.hidden &= hidden;
                }
                None => exits.push(Exit {
                    direction,
                    side,
                    height,
                    to: self.map_ref(dest),
                    portals: vec![portal.name.clone()],
                    hidden,
                }),
            }
        }
        for &dest in &map.exits {
            if leads_out(dest) && !exits.iter().any(|e| e.to.id == dest) {
                exits.push(Exit {
                    direction: Direction::Other,
                    side: None,
                    height: None,
                    to: self.map_ref(dest),
                    portals: vec![],
                    hidden: false,
                });
            }
        }
        exits.sort_by(|a, b| (a.direction, !a.to.known, &a.to.name).cmp(&(b.direction, !b.to.known, &b.to.name)));
        exits
    }
}

/// Link-outs to community pages about the map. Nothing is fetched from them.
fn links(map: &MapRecord) -> Vec<Link> {
    let search: String = map
        .name
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' => (b as char).to_string(),
            b' ' => "+".into(),
            _ => format!("%{b:02X}"),
        })
        .collect();
    vec![
        Link {
            label: "mapleclassic.wiki".into(),
            url: format!("https://mapleclassic.wiki/index.php?search={search}&go=Go"),
        },
        Link { label: "MeowDB".into(), url: format!("https://meowdb.com/msclassic/maps/{:09}", map.id) },
        Link { label: "henesys.gg".into(), url: format!("https://henesys.gg/maps/{}", map.id) },
    ]
}

/// Lowercase words with punctuation dropped: `<Area 1>` and `Area 1` compare equal.
pub(super) fn fold(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Edit similarity, averaged with a pass that forgives the OCR's usual look-alikes
/// (`l`/`1`/`I`, `0`/`O`, `rn`/`m`), so a misread letter costs less than a different one.
pub(super) fn similarity(a: &str, b: &str) -> f64 {
    let lookalike =
        |s: &str| s.replace("rn", "m").replace(['l', '1', '|', 'j'], "i").replace('0', "o").replace('5', "s");
    (normalized_levenshtein(a, b) + normalized_levenshtein(&lookalike(a), &lookalike(b))) / 2.0
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::mapdata::osms::{
        MobRequirement, MonsterDetails, MonsterRecord, NpcRecord, Portal, QuestRecord, RewardItem, Spawn,
    };

    // A small made-up world; none of it is real game data.
    //
    //   Mossbank (town) --east--> Mossbank Meadow I --east--> Mossbank Meadow II
    //        |  in                     | in (hidden)
    //   Mossbank Inn             Burrow Den
    fn portal(name: &str, dest: u32) -> Portal {
        Portal { name: name.into(), dest_map_id: Some(dest), dest_portal: None, side: None, height: None }
    }

    #[test]
    fn settles_a_misread_name_only_when_it_can_mean_one_map() {
        let atlas = world();
        let settled = |name| atlas.settled(name, None).map(|m| m.name);
        assert_eq!(settled("Burrow Den").as_deref(), Some("Burrow Den"));
        // One slipped letter, with no other map close by.
        assert_eq!(settled("Burrow Dem").as_deref(), Some("Burrow Den"));
        assert_eq!(settled("Mossbanh Inn").as_deref(), Some("Mossbank Inn"));
        // Two maps share this name, and Meadow I and II are too close to tell: the reading stands.
        assert_eq!(settled("Mossbank"), None);
        assert_eq!(settled("Mossbank Meadow"), None);
        assert_eq!(settled("Nowhere At All"), None);
    }

    #[test]
    fn portals_to_one_map_stay_apart_when_far_apart() {
        let placed = |name: &str, side, height| Portal { side: Some(side), height: Some(height), ..portal(name, 2) };
        let town = map(
            1,
            "Mossbank",
            "Fern Road",
            vec![
                placed("east00", Side::Right, Height::Low),
                placed("east01", Side::Right, Height::Low),
                placed("east02", Side::Right, Height::High),
                placed("out00", Side::Left, Height::Low),
            ],
        );
        let atlas = Atlas::new(Dataset { maps: vec![town.clone()], ..Dataset::default() });
        let places: Vec<(Option<Side>, Option<Height>, usize)> =
            atlas.exits_of(&town).iter().map(|e| (e.side, e.height, e.portals.len())).collect();
        assert_eq!(
            places,
            [
                (Some(Side::Right), Some(Height::Low), 2),
                (Some(Side::Right), Some(Height::High), 1),
                (Some(Side::Left), Some(Height::Low), 1),
            ]
        );
    }

    pub(crate) fn map(id: u32, name: &str, street: &str, portals: Vec<Portal>) -> MapRecord {
        MapRecord {
            id,
            name: name.into(),
            street_name: Some(street.into()),
            region: Some("Fern Isle".into()),
            is_town: false,
            return_map_id: Some(1),
            map_mark: None,
            exits: portals.iter().filter_map(|p| p.dest_map_id).collect(),
            portals,
        }
    }

    pub(crate) fn mob(id: u32, name: &str, level: u32, exp: u32, spawns: &[(u32, u32)]) -> MonsterRecord {
        MonsterRecord {
            id,
            name: name.into(),
            level,
            hp: level * 10,
            exp,
            is_boss: false,
            undead: false,
            spawns: spawns.iter().map(|&(map_id, count)| Spawn { map_id, count, respawn: None }).collect(),
            details: MonsterDetails::default(),
        }
    }

    pub(crate) fn world() -> Atlas {
        let mut town =
            map(1, "Mossbank", "Fern Road", vec![portal("east00", 2), portal("in00", 5), portal("sp", NO_MAP)]);
        town.is_town = true;
        town.return_map_id = Some(1);
        let mut meadow = map(
            2,
            "Mossbank Meadow I",
            "Fern Road",
            vec![portal("west00", 1), portal("east00", 3), portal("east01", 3), portal("h_in00", 4)],
        );
        meadow.exits.push(77); // listed in the data, but not a map it contains
        Atlas::new(Dataset {
            maps: vec![
                town,
                meadow,
                map(3, "Mossbank Meadow II", "Fern Road", vec![portal("west00", 2)]),
                map(4, "Burrow Den", "Fern Road", vec![portal("out00", 2)]),
                map(5, "Mossbank Inn", "Fern Road", vec![portal("out00", 1)]),
                map(6, "Mossbank", "Old Fern Road", vec![]),
            ],
            monsters: vec![
                mob(10, "Pebble Snail", 2, 4, &[(2, 6), (3, 2)]),
                mob(11, "Moss Slime", 5, 9, &[(2, 4)]),
                mob(12, "Burrow Rat", 8, 15, &[(4, 5)]),
            ],
            npcs: vec![
                NpcRecord { id: 50, name: "Warden Ilse".into(), maps: vec![1] },
                NpcRecord { id: 51, name: "Innkeeper Bo".into(), maps: vec![5] },
            ],
            quests: vec![
                QuestRecord {
                    id: 70,
                    name: "Shell Sorting".into(),
                    npc_id: Some(50),
                    level_min: Some(3),
                    next_quest_id: None,
                    requirements: vec![
                        MobRequirement { mob_id: 10, count: 8 },
                        MobRequirement { mob_id: 12, count: 2 },
                    ],
                    reward_items: vec![RewardItem { item_id: 90, count: 2, sure: true }],
                    ..QuestRecord::default()
                },
                QuestRecord {
                    id: 71,
                    name: "Slime Samples".into(),
                    npc_id: Some(51),
                    level_min: Some(1),
                    next_quest_id: None,
                    requirements: vec![MobRequirement { mob_id: 11, count: 5 }],
                    reward_items: vec![
                        RewardItem { item_id: 90, count: 1, sure: false },
                        RewardItem { item_id: 91, count: 1, sure: false },
                    ],
                    ..QuestRecord::default()
                },
            ],
            ..Dataset::default()
        })
    }

    // A made-up starting island and the mainland beyond it. No portal joins the two.
    pub(crate) fn island_world() -> Atlas {
        let island = |mut m: MapRecord| {
            m.region = Some(STARTING_REGION.into());
            m
        };
        let mut port = island(map(20, "Cove", "Sand Road", vec![portal("east00", 21)]));
        port.is_town = true;
        let mut harbor = map(30, "Harbor", "Main Road", vec![portal("east00", 31)]);
        harbor.is_town = true;
        Atlas::new(Dataset {
            maps: vec![
                port,
                island(map(21, "Cove Shore", "Sand Road", vec![portal("west00", 20)])),
                harbor,
                map(31, "Harbor Field", "Main Road", vec![portal("west00", 30)]),
            ],
            // The mainland has more of the same monster, and better EXP.
            monsters: vec![mob(10, "Pebble Snail", 1, 3, &[(21, 5), (31, 9)]), mob(11, "Moss Slime", 2, 9, &[(31, 5)])],
            npcs: vec![],
            quests: vec![QuestRecord {
                id: 70,
                name: "Shell Sorting".into(),
                npc_id: None,
                level_min: Some(1),
                next_quest_id: None,
                requirements: vec![MobRequirement { mob_id: 10, count: 8 }],
                ..QuestRecord::default()
            }],
            ..Dataset::default()
        })
    }

    #[test]
    fn npcs_tell_maps_with_one_header_apart() {
        let town = |id| map(id, "Mushroom Town", "Maple Road", vec![]);
        let atlas = Atlas::new(Dataset {
            maps: vec![town(10), town(20), map(30, "Harbor", "Main Road", vec![])],
            monsters: vec![],
            npcs: vec![
                NpcRecord { id: 1, name: "Roger".into(), maps: vec![10] },
                NpcRecord { id: 2, name: "Nina".into(), maps: vec![20] },
            ],
            quests: vec![],
            ..Dataset::default()
        });
        let (name, street) = ("Mushrooun Town", Some("Maple Road"));

        assert_eq!(atlas.twin(name, street, &["PebbleThief", "Nina"], None), Some(20));
        assert_eq!(atlas.twin(name, street, &["Roger"], Some(20)), Some(10));
        assert_eq!(atlas.twin(name, street, &["PebbleThief"], Some(20)), Some(20), "the last answer stands");
        assert_eq!(atlas.twin(name, street, &[], None), None);
        assert_eq!(atlas.twin("Harbor", Some("Main Road"), &["Nina"], Some(20)), None, "leaving lets it go");

        let known = atlas.find_twin(name, street, Some(20)).unwrap();
        assert_eq!(known.card.map.id, 20);
        assert!(known.alternatives.is_empty(), "nothing left to ask the player");
        let unsure = atlas.find_twin(name, street, None).unwrap();
        assert_eq!((unsure.card.map.id, unsure.alternatives.len()), (10, 1));
        assert_eq!(atlas.find_twin("Harbor", Some("Main Road"), Some(20)).unwrap().card.map.id, 30);

        assert_eq!(atlas.settled(name, street).unwrap().name, "Mushroom Town", "the spelling at least is certain");
    }

    #[test]
    fn portal_names_give_directions() {
        assert_eq!(Direction::of_portal("west00"), (Direction::West, false));
        assert_eq!(Direction::of_portal("h_east01"), (Direction::East, true));
        assert_eq!(Direction::of_portal("in02"), (Direction::In, false));
        assert_eq!(Direction::of_portal("out00"), (Direction::Out, false));
        assert_eq!(Direction::of_portal("hide01_"), (Direction::Other, true));
        assert_eq!(Direction::of_portal("dun00"), (Direction::Other, false));
    }

    #[test]
    fn finds_maps_from_misread_names() {
        let atlas = world();
        let found = atlas.find("Mossbank Meadovv l", Some("Fern Road")).unwrap();
        assert_eq!(found.card.map.name, "Mossbank Meadow I");
        assert!(found.score > 0.8 && found.score < 1.0, "{}", found.score);
        assert_eq!(atlas.find("Mossbank Meadow II", None).unwrap().card.map.id, 3);
        assert!(atlas.find("Sunken Galleon", None).is_none());
        assert!(atlas.find("  ·  ", None).is_none());
    }

    #[test]
    fn the_street_settles_shared_names() {
        let atlas = world();
        let old = atlas.find("Mossbank", Some("Old Fern Road")).unwrap();
        assert_eq!(old.card.map.id, 6);
        assert_eq!(old.alternatives.iter().map(|m| m.id).collect::<Vec<_>>(), [1]);
        assert_eq!(atlas.find("Mossbank", Some("Fern Road")).unwrap().card.map.id, 1);
        // Without a street, the lower ID wins and the other is offered.
        let either = atlas.find("Mossbank", None).unwrap();
        assert_eq!((either.card.map.id, either.alternatives[0].id), (1, 6));
    }

    #[test]
    fn card_gathers_mobs_npcs_quests_and_exits() {
        let atlas = world();
        let card = atlas.card(2).unwrap();
        let mobs: Vec<(&str, u32)> = card.mobs.iter().map(|m| (m.name.as_str(), m.count)).collect();
        assert_eq!(mobs, [("Pebble Snail", 6), ("Moss Slime", 4)]);
        assert_eq!(card.level_range, Some((2, 5)));
        assert_eq!(card.mob_total, 10);
        assert_eq!(card.exp_per_clear, 6 * 4 + 4 * 9);
        assert_eq!(card.return_map.as_ref().map(|m| m.name.as_str()), Some("Mossbank"));

        // Quests list only the targets found on this map, lowest level first.
        let quests: Vec<String> = card
            .quests
            .iter()
            .map(|q| {
                let targets: Vec<String> = q.targets.iter().map(|t| format!("{} {}", t.count, t.name)).collect();
                format!("{} from {:?}: {}", q.quest.name, q.giver.as_deref().unwrap_or("?"), targets.join(", "))
            })
            .collect();
        assert_eq!(
            quests,
            [
                r#"Slime Samples from "Innkeeper Bo": 5 Moss Slime"#,
                r#"Shell Sorting from "Warden Ilse": 8 Pebble Snail"#
            ]
        );

        let exits: Vec<(Direction, &str, Vec<&str>, bool)> = card
            .exits
            .iter()
            .map(|e| (e.direction, e.to.name.as_str(), e.portals.iter().map(String::as_str).collect(), e.hidden))
            .collect();
        assert_eq!(
            exits,
            [
                (Direction::West, "Mossbank", vec!["west00"], false),
                (Direction::East, "Mossbank Meadow II", vec!["east00", "east01"], false),
                (Direction::In, "Burrow Den", vec!["h_in00"], true),
                (Direction::Other, "Unknown map", vec![], false),
            ]
        );
        assert!(!card.exits[3].to.known);

        let town = atlas.card(1).unwrap();
        assert!(town.is_town && town.mobs.is_empty() && town.return_map.is_none());
        assert_eq!(town.npcs[0].name, "Warden Ilse");
        assert_eq!(town.npcs[0].quests[0].name, "Shell Sorting");
        assert_eq!(town.exits.len(), 2, "the spawn point leads nowhere");
        assert_eq!(town.links[1].url, "https://meowdb.com/msclassic/maps/000000001");
    }

    #[test]
    fn routes_take_the_fewest_portals() {
        let atlas = world();
        let route = atlas.route(5, 4).unwrap();
        let steps: Vec<(&str, Direction, Option<&str>, &str)> = route
            .steps
            .iter()
            .map(|s| (s.from.name.as_str(), s.direction, s.portal.as_deref(), s.to.name.as_str()))
            .collect();
        assert_eq!(
            steps,
            [
                ("Mossbank Inn", Direction::Out, Some("out00"), "Mossbank"),
                ("Mossbank", Direction::East, Some("east00"), "Mossbank Meadow I"),
                ("Mossbank Meadow I", Direction::In, Some("h_in00"), "Burrow Den"),
            ]
        );
        assert!(route.steps[2].hidden);
        assert!(atlas.route(3, 3).unwrap().steps.is_empty());
        assert!(atlas.route(1, 6).is_none(), "no portal reaches the other Mossbank");
        assert!(atlas.route(1, 77).is_none());
        assert_eq!(route.walking, None, "a walk says nothing about walking");
    }

    #[test]
    fn travel_counts_a_ride_as_one_step() {
        use crate::mapdata::travel::{TravelChoice, TravelFile};
        let atlas = world();
        let file = TravelFile::parse(
            r#"{"services": [
                {"kind": "cab", "name": "Cart", "source": "x", "routes": [{"from": "Mossbank Inn", "to": "Mossbank Meadow II", "fare": 50}]},
                {"kind": "scroll", "name": "Scroll to Mossbank", "source": "x", "routes": [{"to": "Mossbank"}]}
            ]}"#,
        )
        .unwrap();
        let (travel, _) = atlas.travel(&file, TravelChoice { enabled: true, ..TravelChoice::default() });

        // Inn to Meadow II: three portals on foot, one cart ride.
        let route = atlas.route_with(5, 3, &travel).unwrap();
        assert_eq!(route.steps.len(), 1);
        let ride = route.steps[0].travel.as_ref().unwrap();
        assert_eq!((ride.service.as_str(), ride.fare), ("Cart", Some(50)));
        assert_eq!(route.walking, Some(3));

        // Meadow I to Mossbank: one portal west ties with the scroll, and walking wins.
        let route = atlas.route_with(2, 1, &travel).unwrap();
        assert!(route.steps[0].travel.is_none());
        assert_eq!(route.walking, None);

        // Burrow Den to the Inn: out, west, in on foot; scroll home, then in.
        let walking = atlas.hops_from(4);
        assert_eq!(walking[&5], 3);
        let shortcuts = atlas.shortcuts_from(4, &travel, &walking);
        assert_eq!(shortcuts[&5].hops, 2);
        assert_eq!(shortcuts[&5].via.name, "Mossbank");
        assert!(!shortcuts.contains_key(&2), "the walk to Meadow I is already one portal");
        assert!(atlas.shortcuts_from(4, &Travel::none(), &walking).is_empty());
    }

    #[test]
    fn search_prefers_prefixes() {
        let atlas = world();
        let names: Vec<String> = atlas.search("meadow", 5).into_iter().map(|m| m.name).collect();
        assert_eq!(names, ["Mossbank Meadow I", "Mossbank Meadow II"]);
        assert_eq!(atlas.search("burr", 5)[0].name, "Burrow Den");
        assert!(atlas.search("", 5).is_empty());
    }
}
